import { isTauri } from '@tauri-apps/api/core';
import { SvelteSet } from 'svelte/reactivity';
import { errorIdentity, errorMessage, errorRevision, type AppErrorStore } from '$lib/app-error';
import { commands, events } from '$lib/ipc';
import type {
  ConfigError,
  ConfigSnapshot,
  CreatedProcessingProfile,
  OverlayPosition,
  UiMode,
} from '$lib/generated/bindings';
import { processingProfileSnapshotFromDraft } from '$lib/processing-profile-state';

export type ConfigMutationResponse = ConfigSnapshot | CreatedProcessingProfile;

function createReactiveState(initialState: ConfigSessionState): ConfigSessionState {
  const state = $state(initialState);
  return state;
}

export type ConfigSessionState = {
  snapshot: ConfigSnapshot;
  revision: number;
  loaded: boolean;
  loading: boolean;
  error: string | null;
};

export type ConfigSession = {
  readonly state: ConfigSessionState;
  readonly current: ConfigSnapshot;
  mount: () => () => void;
  subscribe: (listener: (snapshot: ConfigSnapshot) => void) => () => void;
  mutate: (
    operation: (expectedRevision: number) => Promise<ConfigMutationResponse>,
  ) => Promise<ConfigMutationResponse>;
  setSoundsEnabled: (enabled: boolean) => Promise<ConfigMutationResponse>;
  setKeepHistoryOriginals: (enabled: boolean) => Promise<ConfigMutationResponse>;
  setAutomaticUpdateChecks: (enabled: boolean) => Promise<ConfigMutationResponse>;
  setUiMode: (mode: UiMode) => Promise<ConfigMutationResponse>;
  setOverlayEnabled: (enabled: boolean) => Promise<ConfigMutationResponse>;
  setOverlayPosition: (position: OverlayPosition) => Promise<ConfigMutationResponse>;
};

export type ConfigSessionEnvironment = {
  isTauri?: () => boolean;
  commands?: Pick<typeof commands, 'getCaptureConfig' | 'setUiMode'> &
    Partial<
      Pick<
        typeof commands,
        | 'setOverlayEnabled'
        | 'setOverlayPosition'
        | 'setSoundsEnabled'
        | 'setKeepHistoryOriginals'
        | 'setAutomaticUpdateChecks'
      >
    >;
  events?: {
    configChanged: Pick<typeof events.configChanged, 'listen'>;
    configError: Pick<typeof events.configError, 'listen'>;
    localModelsChanged?: Pick<typeof events.localModelsChanged, 'listen'>;
  };
};

export function isConfigError(value: unknown): value is ConfigError {
  return (
    typeof value === 'object' &&
    value !== null &&
    'currentBindings' in value &&
    Array.isArray((value as { currentBindings?: unknown }).currentBindings)
  );
}

export function createConfigSession(
  availability?: { desktopAvailable: boolean },
  errors?: AppErrorStore,
  stateFactory?: <T>(value: T) => T,
  environment: ConfigSessionEnvironment = {},
): ConfigSession {
  const commandApi = { ...commands, ...environment.commands };
  const eventApi = environment.events ?? events;
  const isDesktop = environment.isTauri ?? isTauri;
  const initialState: ConfigSessionState = {
    snapshot: {
      version: 0,
      captureBindings: [{ id: 'default', shortcut: 'CommandOrControl+Shift+2', profileId: null }],
      openShortcut: 'CommandOrControl+Alt+KeyZ',
      uiMode: 'basic',
      soundsEnabled: true,
      keepHistoryOriginals: true,
      automaticUpdateChecks: true,
      overlayEnabled: true,
      overlayPosition: 'bottom_center',
      configPath: '',
      registered: false,
      registrationError: null,
      lastError: null,
      processing: {
        activeProfileId: 'default',
        profiles: [processingProfileSnapshotFromDraft('default')],
        promptTemplateDefaults: { conversion: '', postprocessing: '' },
      },
      inference: { providers: [] },
      revision: 0,
    },
    revision: 0,
    loaded: false,
    loading: false,
    error: null,
  };
  const state = stateFactory ? stateFactory(initialState) : createReactiveState(initialState);
  const subscribers = new SvelteSet<(snapshot: ConfigSnapshot) => void>();
  let cleanups: Array<() => void> = [];
  let mounted = false;
  let mountCleanup: (() => void) | null = null;
  let readyPromise: Promise<void> | null = null;
  let disposed = false;
  let mutationTail: Promise<void> = Promise.resolve();
  let localErrorRevision = 0;

  function reportError(error: unknown, source = 'config'): void {
    const message = errorMessage(error);
    state.error = message;
    errors?.show(message, errorIdentity(source, errorRevision(error, ++localErrorRevision)));
  }

  function applySnapshot(snapshot: ConfigSnapshot, allowSameRevision = false): boolean {
    if (disposed) return false;
    const revision = snapshot.revision ?? state.revision;
    if (revision < state.revision) return false;
    if (!allowSameRevision && state.loaded && revision === state.revision) return false;
    state.snapshot = snapshot;
    state.revision = revision;
    state.loaded = true;
    state.loading = false;
    state.error = null;
    if (snapshot.lastError?.message) {
      errors?.show(
        snapshot.lastError.message,
        errorIdentity('config', snapshot.lastError.revision),
      );
    }
    if (snapshot.registrationError) {
      errors?.show(
        snapshot.registrationError,
        errorIdentity('config', snapshot.lastError?.revision ?? snapshot.revision),
      );
    }
    subscribers.forEach((listener) => listener(snapshot));
    return true;
  }

  function handleConfigError(payload: ConfigError): void {
    if (disposed) return;
    if (payload.revision < state.revision) return;
    const current = state.snapshot;
    applySnapshot(
      {
        ...current,
        configPath: payload.configPath,
        captureBindings: payload.currentBindings,
        openShortcut: payload.currentOpenShortcut ?? current.openShortcut,
        revision: payload.revision,
        registered: payload.registered,
        registrationError: payload.message,
        lastError: payload,
      },
      true,
    );
    state.revision = Math.max(state.revision, payload.revision);
    state.error = payload.message;
    state.loaded = true;
    state.loading = false;
    errors?.show(payload.message, errorIdentity('config', payload.revision));

    // Error payloads contain only shortcut state. Reconcile the complete backend
    // snapshot, accepting it only if it cannot regress the current revision.
    const resyncRevision = state.revision;
    void commandApi
      .getCaptureConfig()
      .then((snapshot) => {
        if (snapshot.revision === state.revision && state.revision > resyncRevision) return false;
        return applySnapshot(snapshot, true);
      })
      .catch((error) => {
        if (!disposed) reportError(error);
      });
  }

  function subscribe(listener: (snapshot: ConfigSnapshot) => void): () => void {
    if (disposed) return () => undefined;
    subscribers.add(listener);
    if (state.snapshot) listener(state.snapshot);
    return () => subscribers.delete(listener);
  }

  function mutate(
    operation: (expectedRevision: number) => Promise<ConfigMutationResponse>,
  ): Promise<ConfigMutationResponse> {
    if (disposed) return Promise.reject(new Error('Config session is disposed.'));
    let result!: ConfigMutationResponse;
    let failure: unknown;
    const task = mutationTail.then(async () => {
      try {
        if (readyPromise) await readyPromise;
        if (disposed) throw new Error('Config session is disposed.');
        result = await operation(state.revision);
        const snapshot = 'snapshot' in result ? result.snapshot : result;
        applySnapshot(snapshot);
      } catch (error) {
        failure = error;
        if (isConfigError(error)) handleConfigError(error);
      }
    });
    mutationTail = task.catch(() => undefined);
    return task.then(() => {
      if (failure !== undefined) throw failure;
      return result;
    });
  }

  function setSoundsEnabled(enabled: boolean): Promise<ConfigMutationResponse> {
    return mutate((expectedRevision) =>
      commandApi.setSoundsEnabled(enabled, expectedRevision),
    ).catch((error) => {
      if (!isConfigError(error)) reportError(error, 'config');
      throw error;
    });
  }

  function setKeepHistoryOriginals(enabled: boolean): Promise<ConfigMutationResponse> {
    return mutate((expectedRevision) =>
      commandApi.setKeepHistoryOriginals(enabled, expectedRevision),
    ).catch((error) => {
      if (!isConfigError(error)) reportError(error, 'config');
      throw error;
    });
  }

  function setAutomaticUpdateChecks(enabled: boolean): Promise<ConfigMutationResponse> {
    return mutate((expectedRevision) =>
      commandApi.setAutomaticUpdateChecks(enabled, expectedRevision),
    ).catch((error) => {
      if (!isConfigError(error)) reportError(error, 'config');
      throw error;
    });
  }

  function setUiMode(mode: UiMode): Promise<ConfigMutationResponse> {
    return mutate((expectedRevision) => commandApi.setUiMode(mode, expectedRevision)).catch(
      (error) => {
        if (!isConfigError(error)) reportError(error, 'config');
        throw error;
      },
    );
  }

  function setOverlayEnabled(enabled: boolean): Promise<ConfigMutationResponse> {
    return mutate((expectedRevision) =>
      commandApi.setOverlayEnabled(enabled, expectedRevision),
    ).catch((error) => {
      if (!isConfigError(error)) reportError(error, 'config');
      throw error;
    });
  }

  function setOverlayPosition(position: OverlayPosition): Promise<ConfigMutationResponse> {
    return mutate((expectedRevision) =>
      commandApi.setOverlayPosition(position, expectedRevision),
    ).catch((error) => {
      if (!isConfigError(error)) reportError(error, 'config');
      throw error;
    });
  }

  function mount(): () => void {
    if (mounted) return mountCleanup ?? (() => dispose());
    mounted = true;
    if (!isDesktop()) {
      if (availability) availability.desktopAvailable = false;
      state.loaded = true;
      mountCleanup = () => dispose();
      return mountCleanup;
    }

    state.loading = true;
    const initializePromise = Promise.allSettled([
      eventApi.configChanged.listen((event) => {
        if (!disposed) applySnapshot(event.payload);
      }),
      ...(eventApi.localModelsChanged
        ? [
            eventApi.localModelsChanged.listen((event) => {
              if (disposed || !state.loaded) return;
              const changed = state.snapshot.processing.profiles
                .flatMap((profile) => [profile.modelSettings, profile.rulesModelSettings])
                .some((profile) => {
                  if (!profile) return false;
                  if (profile.provider !== 'local') return false;
                  const model = event.payload.models.find((item) => item.id === profile.model);
                  return model && model.downloaded !== profile.localModelDownloaded;
                });
              if (!changed) return;
              void commandApi
                .getCaptureConfig()
                .then((snapshot) => applySnapshot(snapshot, true))
                .catch((error) => {
                  if (!disposed) reportError(error);
                });
            }),
          ]
        : []),
      eventApi.configError.listen((event) => {
        if (!disposed) handleConfigError(event.payload);
      }),
    ])
      .then((results) => {
        const listeners = results.flatMap((result) =>
          result.status === 'fulfilled' ? [result.value] : [],
        );
        const failure = results.find((result) => result.status === 'rejected');
        if (failure) {
          listeners.forEach((cleanup) => cleanup());
          throw failure.reason;
        }
        if (disposed) {
          listeners.forEach((cleanup) => cleanup());
          return;
        }
        cleanups = listeners;
        return commandApi.getCaptureConfig().then((snapshot) => {
          if (!disposed) applySnapshot(snapshot);
        });
      })
      .catch((error) => {
        cleanups.forEach((cleanup) => cleanup());
        cleanups = [];
        if (!disposed) {
          state.loaded = true;
          state.loading = false;
          reportError(error, 'config');
        }
      });
    readyPromise = initializePromise;
    void initializePromise;
    mountCleanup = () => dispose();
    return mountCleanup;
  }

  function dispose(): void {
    if (disposed) return;
    disposed = true;
    cleanups.forEach((cleanup) => cleanup());
    cleanups = [];
    subscribers.clear();
  }

  return {
    state,
    get current() {
      return state.snapshot;
    },
    mount,
    subscribe,
    mutate,
    setUiMode,
    setSoundsEnabled,
    setKeepHistoryOriginals,
    setAutomaticUpdateChecks,
    setOverlayEnabled,
    setOverlayPosition,
  };
}
