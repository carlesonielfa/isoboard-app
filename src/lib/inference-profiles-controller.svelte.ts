import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import * as defaultBridge from '$lib/inference-profile-bridge';
import {
  DEFAULT_INFERENCE_BACKEND_METADATA,
  synthesizeInferenceProviders,
} from '$lib/inference-profile-state';
import { errorIdentity, errorMessage, errorRevision, type AppErrorStore } from '$lib/app-error';
import {
  isConfigError,
  type ConfigMutationResponse,
  type ConfigSession,
} from '$lib/config-session.svelte';
import type { DesktopAvailability, InferenceProfilesViewState } from '$lib/page-types';
import type { ConfigSnapshot, InferenceBackend, ProviderSnapshot } from '$lib/generated/bindings';

type ConfigOperation = NonNullable<InferenceProfilesViewState['pending']>;

function createReactiveState(initialState: InferenceProfilesViewState): InferenceProfilesViewState {
  const state = $state(initialState);
  return state;
}

export type InferenceProfilesControllerOptions = {
  availability: DesktopAvailability;
  session: ConfigSession;
  errors?: AppErrorStore;
  bridge?: typeof defaultBridge;
  stateFactory?: <T>(value: T) => T;
};

export function createInferenceProfilesController({
  availability,
  session,
  errors,
  bridge = defaultBridge,
  stateFactory,
}: InferenceProfilesControllerOptions) {
  const initialState: InferenceProfilesViewState = {
    metadata: DEFAULT_INFERENCE_BACKEND_METADATA,
    metadataLoaded: false,
    pending: null,
    readiness: {},
    error: null,
  };
  const state = stateFactory ? stateFactory(initialState) : createReactiveState(initialState);
  let disposed = false;
  let cleanups: Array<() => void> = [];
  let mutationCount = 0;
  let localErrorRevision = 0;
  const readinessSequences = new SvelteMap<string, number>();
  const providerSignatures = new SvelteMap<string, string>();

  function showError(error: unknown, source = 'inference'): void {
    state.error = errorMessage(error);
    errors?.show(state.error, errorIdentity(source, errorRevision(error, ++localErrorRevision)));
  }

  function currentProviders(): ProviderSnapshot[] {
    return synthesizeInferenceProviders(state.metadata, session.current.inference.providers);
  }

  function invalidateProviderAuthState(snapshot: ConfigSnapshot): void {
    const next = synthesizeInferenceProviders(state.metadata, snapshot.inference.providers);
    const nextSignatures = new SvelteMap(
      next.map((provider) => [provider.id, `${provider.backend}:${provider.credentialRef ?? ''}`]),
    );
    const ids = new SvelteSet([...providerSignatures.keys(), ...nextSignatures.keys()]);
    for (const id of ids) {
      if (providerSignatures.get(id) === nextSignatures.get(id)) continue;
      delete state.readiness[id];
      readinessSequences.set(id, (readinessSequences.get(id) ?? 0) + 1);
    }
    providerSignatures.clear();
    nextSignatures.forEach((signature, id) => providerSignatures.set(id, signature));
  }

  function currentProviderMatches(
    providerId: string,
    backend: InferenceBackend,
    credentialRef: string | null,
  ): boolean {
    return currentProviders().some(
      (provider) =>
        (provider.id === providerId || provider.backend === providerId) &&
        provider.backend === backend &&
        provider.credentialRef === credentialRef,
    );
  }

  function enqueueMutation(
    operation: ConfigOperation,
    run: (expectedRevision: number) => Promise<ConfigMutationResponse>,
    onSnapshot?: (snapshot: ConfigSnapshot, response: ConfigMutationResponse) => void,
  ): Promise<boolean> {
    if (!availability.desktopAvailable || disposed) return Promise.resolve(false);
    mutationCount += 1;
    state.pending = operation;
    state.error = null;
    return session
      .mutate(run)
      .then((response) => {
        if (disposed) return false;
        const snapshot = 'snapshot' in response ? response.snapshot : response;
        onSnapshot?.(snapshot, response);
        return true;
      })
      .catch((error) => {
        if (!disposed && !isConfigError(error)) showError(error);
        return false;
      })
      .finally(() => {
        mutationCount -= 1;
        if (!disposed && mutationCount === 0) state.pending = null;
      });
  }

  function saveProvider(
    provider: InferenceBackend,
    endpoint: string | null,
    credential: string,
  ): Promise<boolean> {
    const metadata = state.metadata.find((item) => item.id === provider);
    if (!metadata || (metadata.supportsCustomEndpoint && !endpoint?.trim()))
      return Promise.resolve(false);
    if (!credential.trim()) {
      // A blank key field saves a keyless connection only when the endpoint
      // changes; an unchanged endpoint keeps any stored key.
      const current = currentProviders().find((candidate) => candidate.backend === provider);
      if (!metadata.credentialOptional || current?.endpoint === endpoint?.trim())
        return Promise.resolve(false);
    }
    return enqueueMutation('provider-credential', (expectedRevision) =>
      bridge.saveInferenceProviderWithCredential(
        { backend: provider, endpoint: endpoint?.trim() || null, credential },
        expectedRevision,
      ),
    ).then((saved) => {
      if (saved) {
        delete state.readiness[provider];
        readinessSequences.set(provider, (readinessSequences.get(provider) ?? 0) + 1);
        void refreshReadiness(provider);
      }
      return saved;
    });
  }

  /** Removes the provider's stored API key after native confirmation. */
  function clearProviderCredential(provider: InferenceBackend, label: string): Promise<boolean> {
    return enqueueMutation('provider-credential', (expectedRevision) =>
      bridge.clearProviderCredential(provider, label, expectedRevision),
    ).then((cleared) => {
      if (cleared) void refreshReadiness(provider);
      return cleared;
    });
  }

  async function refreshReadiness(providerId: string): Promise<void> {
    if (!availability.desktopAvailable || disposed) return;
    const provider = currentProviders().find(
      (candidate) => candidate.id === providerId || candidate.backend === providerId,
    );
    if (!provider) return;
    const sequence = (readinessSequences.get(providerId) ?? 0) + 1;
    readinessSequences.set(providerId, sequence);
    delete state.readiness[providerId];
    try {
      const readiness = await bridge.getInferenceCredentialReadiness(provider.backend);
      if (
        disposed ||
        readinessSequences.get(providerId) !== sequence ||
        !currentProviderMatches(providerId, provider.backend, provider.credentialRef)
      )
        return;
      state.readiness[providerId] = readiness;
    } catch (error) {
      if (
        !disposed &&
        readinessSequences.get(providerId) === sequence &&
        currentProviderMatches(providerId, provider.backend, provider.credentialRef)
      )
        showError(error);
    }
  }

  function mount(): () => void {
    if (cleanups.length > 0) return dispose;
    const unsubscribe = session.subscribe((snapshot) => invalidateProviderAuthState(snapshot));
    cleanups = [unsubscribe];
    if (!availability.desktopAvailable) {
      state.metadataLoaded = true;
      return dispose;
    }
    void bridge
      .getInferenceBackendMetadata()
      .then((metadata) => {
        if (!disposed) {
          state.metadata = metadata;
        }
      })
      .catch((error) => {
        if (!disposed) showError(error, 'inference');
      })
      .finally(() => {
        if (!disposed) state.metadataLoaded = true;
      });
    return dispose;
  }

  function dispose(): void {
    if (disposed) return;
    disposed = true;
    cleanups.forEach((cleanup) => cleanup());
    cleanups = [];
    readinessSequences.clear();
  }

  return {
    state,
    saveProvider,
    clearProviderCredential,
    refreshReadiness,
    mount,
    dispose,
  };
}
