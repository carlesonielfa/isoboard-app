<script lang="ts">
  import type {
    DesktopAvailability,
    InferenceProfilesViewState,
    InferenceProviderDraft,
  } from '$lib/page-types';
  import type { InferenceBackend, ProviderSnapshot } from '$lib/generated/bindings';
  import ExpandableSettingsRow from '$lib/components/ExpandableSettingsRow.svelte';
  import InferenceProviderEditor from '$lib/components/InferenceProviderEditor.svelte';
  import ProviderLogo from '$lib/components/ProviderLogo.svelte';
  import InfoLink from '$lib/components/InfoLink.svelte';
  import { providerDocsLinks } from '$lib/docs-links';
  import { INFERENCE_BACKEND_LABELS, providerConnected } from '$lib/inference-profile-state';

  let {
    state: viewState,
    configLoaded,
    providers,
    availability,
    onSaveProvider,
    onClearCredential,
    onRefreshReadiness,
  }: {
    state: InferenceProfilesViewState;
    configLoaded: boolean;
    providers: ProviderSnapshot[];
    availability: DesktopAvailability;
    onSaveProvider: (
      provider: InferenceBackend,
      endpoint: string | null,
      credential: string,
    ) => Promise<boolean>;
    onClearCredential: (provider: InferenceBackend, label: string) => Promise<boolean>;
    onRefreshReadiness: (provider: string) => Promise<void>;
  } = $props();

  let expandedProvider = $state<InferenceBackend | null>(null);
  const providerOptions = $derived(providers);
  const busy = $derived(
    !availability.desktopAvailable ||
      !configLoaded ||
      !viewState.metadataLoaded ||
      viewState.pending !== null,
  );

  function toggleProvider(backend: InferenceBackend): void {
    if (busy) return;
    expandedProvider = expandedProvider === backend ? null : backend;
    if (expandedProvider && expandedProvider !== 'local') {
      void onRefreshReadiness(backend);
    }
  }

  function providerDraft(backend: InferenceBackend): InferenceProviderDraft {
    const configured = providers.find((provider) => provider.backend === backend);
    return configured ?? { backend, endpoint: null, credentialRef: null };
  }
</script>

<div class="flex flex-col gap-5">
  <ul class="m-0 flex list-none flex-col gap-2 p-0" aria-label="Providers">
    {#each viewState.metadata as item (item.id)}
      {@const provider = providerOptions.find((candidate) => candidate.backend === item.id)}
      {@const providerLabel = INFERENCE_BACKEND_LABELS[item.id as InferenceBackend]}
      {@const configured =
        item.id === 'local' || (provider !== undefined && providerConnected(provider))}
      <li class="flex min-w-0 flex-col gap-2">
        <ExpandableSettingsRow
          title={providerLabel}
          expanded={expandedProvider === item.id}
          {configured}
          disabled={busy}
          ariaLabel={configured ? `${providerLabel}, configured` : providerLabel}
          onToggle={() => toggleProvider(item.id as InferenceBackend)}
        >
          {#snippet leading()}
            <ProviderLogo backend={item.id as InferenceBackend} />
          {/snippet}
          {#snippet actions()}
            <InfoLink {...providerDocsLinks[item.id as InferenceBackend].about} size="sm" />
          {/snippet}
          <InferenceProviderEditor
            draft={providerDraft(item.id as InferenceBackend)}
            metadata={viewState.metadata}
            readiness={viewState.readiness[item.id]}
            disabled={busy}
            embedded
            onSave={(value, credential) =>
              onSaveProvider(value.backend, value.endpoint, credential)}
            onClearCredential={() => onClearCredential(item.id as InferenceBackend, providerLabel)}
          />
        </ExpandableSettingsRow>
      </li>
    {/each}
  </ul>
</div>
