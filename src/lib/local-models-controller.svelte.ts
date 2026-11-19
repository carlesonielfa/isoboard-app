import { isTauri } from '@tauri-apps/api/core';
import { getContext, setContext } from 'svelte';
import { commands, events } from '$lib/ipc';
import type { LocalModelsChangedEvent, UiMode } from '$lib/generated/bindings';

export type LocalModelsState = {
  snapshot: LocalModelsChangedEvent;
  loading: boolean;
  error: string | null;
  pending: Record<string, boolean>;
  errors: Record<string, string | null>;
};
export type ModelAction = 'download' | 'cancel' | 'remove';
type Bridge = Pick<
  typeof commands,
  | 'prepareLocalModel'
  | 'getLocalModelsState'
  | 'downloadLocalModel'
  | 'cancelLocalModelDownload'
  | 'removeLocalModelDownload'
>;
type Environment = {
  isDesktop?: () => boolean;
  commands?: Bridge;
  listen?: typeof events.localModelsChanged.listen;
  stateFactory?: (value: LocalModelsState) => LocalModelsState;
};
function reactive(value: LocalModelsState): LocalModelsState {
  const state = $state(value);
  return state;
}

export function createLocalModelsController(environment: Environment = {}) {
  const api = environment.commands ?? commands;
  const state = (environment.stateFactory ?? reactive)({
    snapshot: { revision: 0, models: [] },
    loading: true,
    error: null,
    pending: {},
    errors: {},
  });
  let disposed = false;
  let mounted = false;
  let cleanup: (() => void) | undefined;
  function accept(snapshot: LocalModelsChangedEvent) {
    if (disposed || snapshot.revision < state.snapshot.revision) return;
    state.snapshot = snapshot;
    state.loading = false;
    state.error = null;
  }
  function mount() {
    if (mounted) return () => undefined;
    mounted = true;
    if (!(environment.isDesktop ?? isTauri)()) {
      state.loading = false;
      return dispose;
    }
    // Subscribe before reading so an older initial response cannot hide an update.
    void (environment.listen ?? events.localModelsChanged.listen)((event) => accept(event.payload))
      .then(async (unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        cleanup = unlisten;
        accept(await api.getLocalModelsState());
      })
      .catch((error: unknown) => {
        if (!disposed) {
          state.error = String(error);
          state.loading = false;
        }
      });
    return dispose;
  }
  function dispose() {
    disposed = true;
    cleanup?.();
  }
  async function runAction(model: string, action: ModelAction | 'prepare'): Promise<boolean> {
    if (disposed || state.pending[model]) return false;
    state.pending = { ...state.pending, [model]: true };
    state.errors = { ...state.errors, [model]: null };
    const operations = {
      prepare: api.prepareLocalModel,
      download: api.downloadLocalModel,
      cancel: api.cancelLocalModelDownload,
      remove: api.removeLocalModelDownload,
    };
    try {
      accept(await operations[action](model));
      return !disposed;
    } catch (error) {
      if (!disposed) state.errors = { ...state.errors, [model]: String(error) };
      return false;
    } finally {
      if (!disposed) state.pending = { ...state.pending, [model]: false };
    }
  }
  async function act(model: string, action: ModelAction): Promise<void> {
    await runAction(model, action);
  }
  return { state, mount, act, prepare: (model: string) => runAction(model, 'prepare') };
}
export type LocalModelsController = ReturnType<typeof createLocalModelsController>;
const context = Symbol('local-models');
export function provideLocalModels(
  controller: LocalModelsController,
  profileCount: (id: string) => number,
  isCapturing: () => boolean = () => false,
  uiMode: () => UiMode = () => 'basic',
  canNotify: () => boolean = () => false,
) {
  setContext(context, { controller, profileCount, isCapturing, uiMode, canNotify });
}
export function useLocalModels() {
  return getContext<
    | {
        controller: LocalModelsController;
        profileCount: (id: string) => number;
        isCapturing: () => boolean;
        uiMode: () => UiMode;
        canNotify: () => boolean;
      }
    | undefined
  >(context);
}
