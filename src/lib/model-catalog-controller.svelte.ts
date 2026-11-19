import { events } from '$lib/ipc';
import * as defaultBridge from '$lib/inference-profile-bridge';
import {
  EMPTY_MODEL_CATALOG,
  modelCatalogStatus,
  type InferenceModelCatalogSnapshot,
  type ModelCatalogStatus,
  projectModelCatalogSnapshot,
} from '$lib/model-catalog-state';
import type { ModelCatalogSnapshot } from '$lib/generated/bindings';
import type { DesktopAvailability } from '$lib/page-types';

export type ModelCatalogState = {
  snapshot: InferenceModelCatalogSnapshot | null;
  loading: boolean;
  status: ModelCatalogStatus;
  error: string | null;
};

export type ModelCatalogControllerOptions = {
  availability: DesktopAvailability;
  bridge?: { getInferenceModelCatalog: () => Promise<ModelCatalogSnapshot> };
  events?: {
    inferenceModelCatalogChanged: Pick<typeof events.inferenceModelCatalogChanged, 'listen'>;
  };
  stateFactory?: <T>(value: T) => T;
};

function createReactiveState(initialState: ModelCatalogState): ModelCatalogState {
  const state = $state(initialState);
  return state;
}

export function createModelCatalogController({
  availability,
  bridge = defaultBridge,
  stateFactory,
  events: eventApi,
}: ModelCatalogControllerOptions) {
  const resolvedEvents = eventApi ?? events;
  const state = stateFactory
    ? stateFactory<ModelCatalogState>({
        snapshot: null,
        loading: false,
        status: 'custom',
        error: null,
      })
    : createReactiveState({ snapshot: null, loading: false, status: 'custom', error: null });
  let disposed = false;
  let mounted = false;
  let cleanup: (() => void) | null = null;
  let sequence = 0;
  let latestRevision = -1;

  function apply(value: ModelCatalogSnapshot): boolean {
    if (disposed) return false;
    const snapshot = projectModelCatalogSnapshot(value);
    if (snapshot.revision < latestRevision) return false;
    latestRevision = snapshot.revision;
    state.snapshot = snapshot;
    state.status = modelCatalogStatus(snapshot);
    state.error = snapshot.error;
    return true;
  }

  function mount(): () => void {
    if (mounted) return dispose;
    mounted = true;
    if (!availability.desktopAvailable) {
      state.snapshot = { ...EMPTY_MODEL_CATALOG, offline: true };
      state.status = 'offline';
      return dispose;
    }
    const currentSequence = ++sequence;
    state.loading = true;
    state.status = 'loading';
    const listenCatalog = resolvedEvents.inferenceModelCatalogChanged.listen((event) =>
      apply(event.payload),
    );
    void listenCatalog
      .then((unsubscribe) => {
        if (disposed || currentSequence !== sequence) unsubscribe();
        else cleanup = unsubscribe;
      })
      .catch((error) => {
        if (disposed || currentSequence !== sequence) return;
        state.error = error instanceof Error ? error.message : String(error);
        state.snapshot = state.snapshot ?? { ...EMPTY_MODEL_CATALOG, offline: true };
        state.status = state.snapshot.stale ? 'stale' : 'offline';
      });
    void bridge
      .getInferenceModelCatalog()
      .then((snapshot) => {
        if (!disposed && currentSequence === sequence) apply(snapshot);
      })
      .catch((error) => {
        if (disposed || currentSequence !== sequence) return;
        state.error = error instanceof Error ? error.message : String(error);
        state.snapshot = state.snapshot ?? { ...EMPTY_MODEL_CATALOG, offline: true };
        state.status = state.snapshot.stale ? 'stale' : 'offline';
      })
      .finally(() => {
        if (!disposed && currentSequence === sequence) state.loading = false;
      });
    return dispose;
  }

  function dispose(): void {
    if (disposed) return;
    disposed = true;
    sequence += 1;
    cleanup?.();
    cleanup = null;
  }

  return { state, mount, dispose };
}
