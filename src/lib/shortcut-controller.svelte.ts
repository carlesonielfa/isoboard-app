import { commands } from '$lib/ipc';
import { createSerializedBooleanWorker } from '$lib/serialized-boolean-worker';
import { restoreThenSaveShortcut } from '$lib/shortcut-autosave';
import { errorIdentity, errorMessage, errorRevision, type AppErrorStore } from '$lib/app-error';
import { recordingTargetExists, shouldHandleRecordingCancel } from '$lib/shortcut-controller-state';
import { draftShortcutAfterSave, resolveNewBindingProfileId } from '$lib/shortcut-binding-state';
import { isConfigError, type ConfigSession } from '$lib/config-session.svelte';
import type {
  DesktopAvailability,
  ShortcutRecordingTarget,
  ShortcutViewState,
} from '$lib/page-types';

function createReactiveState(initialState: ShortcutViewState): ShortcutViewState {
  const state = $state(initialState);
  return state;
}

export type ShortcutControllerOptions = {
  availability: DesktopAvailability;
  session: ConfigSession;
  errors?: AppErrorStore;
  commands?: Pick<
    typeof commands,
    | 'setCaptureShortcutRecording'
    | 'createCaptureBinding'
    | 'setOpenShortcut'
    | 'setCaptureBinding'
    | 'setCaptureBindingProfile'
    | 'deleteCaptureBinding'
  >;
  stateFactory?: <T>(value: T) => T;
};

export function createShortcutController({
  availability,
  session,
  errors,
  commands: commandApi = commands,
  stateFactory,
}: ShortcutControllerOptions) {
  const nativeRecordingWorker = createSerializedBooleanWorker((recording) =>
    commandApi.setCaptureShortcutRecording(recording),
  );
  const initialState: ShortcutViewState = {
    newBindingProfileId: 'default',
    newBindingShortcut: '',
    recordingTarget: null,
    recording: false,
    recordingPending: false,
    saving: false,
    error: null,
  };
  const state = stateFactory ? stateFactory(initialState) : createReactiveState(initialState);
  let recordingOperation = 0;
  let localErrorRevision = 0;
  let cleanupSession: (() => void) | null = null;

  function reportLocalError(message: string): void {
    errors?.show(message, errorIdentity('shortcut', ++localErrorRevision));
  }

  function chooseNewBindingProfile(): void {
    const processing = session.current.processing;
    state.newBindingProfileId = resolveNewBindingProfileId(
      state.newBindingProfileId,
      processing.profiles,
      processing.activeProfileId,
      session.state.loaded,
    );
  }

  function revalidateExternalConfig(): void {
    chooseNewBindingProfile();
    if (
      (state.recording || state.recordingPending) &&
      state.recordingTarget &&
      !recordingTargetExists(
        state.recordingTarget,
        session.current.captureBindings,
        session.current.processing.profiles,
      )
    )
      cancelRecording();
  }

  function setNativeRecording(recording: boolean): Promise<boolean> {
    if (!availability.desktopAvailable) return Promise.resolve(true);
    return nativeRecordingWorker.request(recording);
  }

  function failRecordingStart(): void {
    state.recording = false;
    state.recordingPending = false;
    state.recordingTarget = null;
  }

  function requestRecordingStart(target: ShortcutRecordingTarget): void {
    if (!availability.desktopAvailable) {
      state.recording = true;
      state.recordingPending = false;
      state.recordingTarget = target;
      return;
    }
    state.recordingPending = true;
    state.recordingTarget = target;
    const operation = ++recordingOperation;
    void setNativeRecording(true)
      .then((enabled) => {
        if (operation !== recordingOperation || !state.recordingPending) return;
        if (enabled) {
          state.recordingPending = false;
          state.recording = true;
        } else {
          failRecordingStart();
          state.error ??= 'Keep the isoboard window focused to record a shortcut.';
          if (state.error) reportLocalError(state.error);
        }
      })
      .catch((error) => {
        if (operation === recordingOperation && state.recordingPending) {
          failRecordingStart();
          state.error = errorMessage(error);
          reportLocalError(state.error);
        }
      });
  }

  function restoreNativeRecording(target: ShortcutRecordingTarget | null = null): void {
    state.recordingPending = true;
    const operation = ++recordingOperation;
    void setNativeRecording(false)
      .then((restored) => {
        if (operation !== recordingOperation) return;
        state.recordingPending = false;
        if (!restored) {
          state.recording = true;
          state.recordingTarget = target;
          state.error =
            'Could not restore the active global shortcut. Try Escape and record again.';
          reportLocalError(state.error);
        }
      })
      .catch((error) => {
        if (operation !== recordingOperation) return;
        state.recordingPending = false;
        state.recording = true;
        state.recordingTarget = target;
        state.error = errorMessage(error);
        reportLocalError(state.error);
      });
  }

  function saveBinding(
    shortcutValue: string,
    target: ShortcutRecordingTarget,
    operation: number,
  ): Promise<void> {
    const shortcut = shortcutValue.trim();
    if (!availability.desktopAvailable || !shortcut || operation !== recordingOperation)
      return Promise.resolve();
    state.saving = true;
    state.error = null;
    return session
      .mutate((expectedRevision) => {
        switch (target.kind) {
          case 'create':
            return commandApi.createCaptureBinding(target.profileId, shortcut, expectedRevision);
          case 'open':
            return commandApi.setOpenShortcut(shortcut, expectedRevision);
          case 'binding':
            return commandApi.setCaptureBinding(target.bindingId, shortcut, expectedRevision);
        }
      })
      .then(() => {
        if (operation !== recordingOperation) return;
        state.newBindingShortcut = draftShortcutAfterSave(
          state.newBindingShortcut,
          target.kind,
          shortcut,
          true,
        );
      })
      .catch((error) => {
        if (operation !== recordingOperation) return;
        state.newBindingShortcut = draftShortcutAfterSave(
          state.newBindingShortcut,
          target.kind,
          shortcut,
          false,
        );
        if (isConfigError(error)) {
          state.error =
            error.kind === 'conflict' ? `${error.message} Choose another shortcut.` : error.message;
        } else {
          state.error = errorMessage(error);
          errors?.show(
            state.error,
            errorIdentity('shortcut', errorRevision(error, ++localErrorRevision)),
          );
        }
      })
      .finally(() => {
        if (operation === recordingOperation) state.saving = false;
      });
  }

  function restoreAndSaveRecordedShortcut(shortcut: string, target: ShortcutRecordingTarget): void {
    const operation = ++recordingOperation;
    state.recordingPending = true;
    void restoreThenSaveShortcut(shortcut, {
      restoreNativeShortcut: () => setNativeRecording(false),
      isCurrent: () => operation === recordingOperation && state.recordingPending,
      saveShortcut: async (value) => {
        state.recordingPending = false;
        await saveBinding(value, target, operation);
      },
    })
      .then((result) => {
        if (operation !== recordingOperation) return;
        if (result === 'restore-failed') {
          state.recordingPending = false;
          state.recording = true;
          state.recordingTarget = target;
          state.error =
            'Could not restore the active global shortcut. Try Escape and record again.';
          reportLocalError(state.error);
        } else if (result === 'cancelled') {
          state.recordingPending = false;
        } else if (!state.saving) {
          state.recordingTarget = null;
        }
      })
      .catch((error) => {
        if (operation !== recordingOperation) return;
        state.recordingPending = false;
        state.recording = true;
        state.recordingTarget = target;
        state.error = errorMessage(error);
        reportLocalError(state.error);
      });
  }

  function startBindingRecording(bindingId: string): void {
    if (!availability.desktopAvailable || state.saving || state.recording || state.recordingPending)
      return;
    if (!session.current.captureBindings.some((binding) => binding.id === bindingId)) return;
    state.error = null;
    requestRecordingStart({ kind: 'binding', bindingId });
  }

  function startCreateBindingRecording(profileId = state.newBindingProfileId): void {
    if (!availability.desktopAvailable || state.saving || state.recording || state.recordingPending)
      return;
    if (!session.current.processing.profiles.some((profile) => profile.id === profileId)) return;
    state.error = null;
    requestRecordingStart({ kind: 'create', profileId });
  }

  function startOpenShortcutRecording(): void {
    if (!availability.desktopAvailable || state.saving || state.recording || state.recordingPending)
      return;
    state.error = null;
    requestRecordingStart({ kind: 'open' });
  }

  function recordShortcut(shortcut: string): void {
    if (!state.recording || !state.recordingTarget) return;
    const target = state.recordingTarget;
    state.recording = false;
    restoreAndSaveRecordedShortcut(shortcut, target);
  }

  function cancelRecording(): void {
    if (
      !shouldHandleRecordingCancel(state.recordingTarget, state.recording, state.recordingPending)
    )
      return;
    const target = state.recordingTarget;
    state.recording = false;
    state.recordingPending = false;
    state.recordingTarget = null;
    restoreNativeRecording(target);
  }

  async function setBindingProfile(bindingId: string, profileId: string): Promise<boolean> {
    const config = session.current;
    if (
      !availability.desktopAvailable ||
      state.saving ||
      state.recording ||
      state.recordingPending ||
      bindingId === 'default' ||
      !config.captureBindings.some(
        (binding) => binding.id === bindingId && binding.profileId !== null,
      ) ||
      !config.processing.profiles.some((profile) => profile.id === profileId)
    )
      return false;
    const operation = ++recordingOperation;
    state.saving = true;
    state.error = null;
    try {
      await session.mutate((expectedRevision) =>
        commandApi.setCaptureBindingProfile(bindingId, profileId, expectedRevision),
      );
      return operation === recordingOperation;
    } catch (error) {
      if (operation === recordingOperation) {
        state.error =
          isConfigError(error) && error.kind === 'conflict'
            ? `${error.message} Choose another shortcut.`
            : errorMessage(error);
        if (!isConfigError(error))
          errors?.show(
            state.error,
            errorIdentity('shortcut', errorRevision(error, ++localErrorRevision)),
          );
      }
      return false;
    } finally {
      if (operation === recordingOperation) state.saving = false;
    }
  }

  async function deleteBinding(bindingId: string): Promise<boolean> {
    if (!availability.desktopAvailable || state.saving || state.recording || state.recordingPending)
      return false;
    if (!session.current.captureBindings.some((binding) => binding.id === bindingId)) return false;
    const operation = ++recordingOperation;
    state.saving = true;
    state.error = null;
    try {
      await session.mutate((expectedRevision) =>
        commandApi.deleteCaptureBinding(bindingId, expectedRevision),
      );
      return operation === recordingOperation;
    } catch (error) {
      if (operation === recordingOperation) {
        state.error =
          isConfigError(error) && error.kind === 'conflict'
            ? `${error.message} Choose another shortcut.`
            : errorMessage(error);
        if (!isConfigError(error))
          errors?.show(
            state.error,
            errorIdentity('shortcut', errorRevision(error, ++localErrorRevision)),
          );
      }
      return false;
    } finally {
      if (operation === recordingOperation) state.saving = false;
    }
  }

  function cleanupRecording(): void {
    recordingOperation += 1;
    state.recording = false;
    state.recordingPending = false;
    state.recordingTarget = null;
    void setNativeRecording(false).catch(() => undefined);
  }

  function dispose(): void {
    cleanupRecording();
    cleanupSession?.();
    cleanupSession = null;
  }

  function mount(): () => void {
    if (cleanupSession) return dispose;
    chooseNewBindingProfile();
    cleanupSession = session.subscribe(revalidateExternalConfig);
    return dispose;
  }

  return {
    state,
    startBindingRecording,
    startCreateBindingRecording,
    startOpenShortcutRecording,
    recordShortcut,
    cancelRecording,
    deleteBinding,
    setNewBindingProfile: (profileId: string) => {
      if (session.current.processing.profiles.some((profile) => profile.id === profileId))
        state.newBindingProfileId = profileId;
    },
    setBindingProfile,
    mount,
    dispose,
  };
}
