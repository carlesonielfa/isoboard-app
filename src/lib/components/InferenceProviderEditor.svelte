<script lang="ts">
  import type { InferenceProviderDraft } from '$lib/page-types';
  import type {
    InferenceBackendMetadata,
    InferenceCredentialReadinessSnapshot,
  } from '$lib/generated/bindings';
  import SettingsInput from '$lib/components/SettingsInput.svelte';
  import { backendMetadataFor, inferenceProviderIsValid } from '$lib/inference-profile-state';

  let {
    draft: initialDraft,
    metadata,
    readiness,
    disabled,
    onSave,
    embedded = false,
  }: {
    draft: InferenceProviderDraft;
    metadata: InferenceBackendMetadata[];
    readiness?: InferenceCredentialReadinessSnapshot;
    disabled: boolean;
    onSave: (draft: InferenceProviderDraft, credential: string) => Promise<boolean>;
    embedded?: boolean;
  } = $props();

  let draft = $state<InferenceProviderDraft>({
    backend: 'openai_api',
    endpoint: null,
    credentialRef: null,
  });
  let credential = $state('');
  let saving = $state(false);
  let lastInitialDraftKey = $state('');

  $effect(() => {
    const key = `${initialDraft.backend}\u0000${initialDraft.endpoint ?? ''}`;
    if (!saving && key !== lastInitialDraftKey) {
      draft = { ...initialDraft };
      lastInitialDraftKey = key;
    }
  });

  const backend = $derived(backendMetadataFor(draft.backend, metadata));
  const valid = $derived(inferenceProviderIsValid(draft, metadata));
  const savingDisabled = $derived(disabled || saving);
  const readinessLabels: Record<string, string> = {
    missing: 'Missing',
    ready: 'Ready',
    locked: 'Locked',
    corrupt: 'Unavailable',
    unavailable: 'Unavailable',
    unknown: 'Configured',
  };

  function updateDraft(changes: Partial<InferenceProviderDraft>): void {
    draft = { ...draft, ...changes };
  }

  async function save(credentialValue = ''): Promise<void> {
    if (savingDisabled || !valid) return;
    saving = true;
    try {
      const saved = await onSave({ ...draft }, credentialValue);
      if (saved) credential = '';
    } finally {
      saving = false;
    }
  }
</script>

<div
  class={embedded
    ? 'grid min-w-0 gap-3'
    : 'grid min-w-0 gap-3 rounded-control border border-border bg-surface-control p-3'}
>
  {#if backend.supportsCustomEndpoint}
    <SettingsInput
      label="Endpoint"
      value={draft.endpoint ?? ''}
      disabled={savingDisabled}
      oninput={(event) => updateDraft({ endpoint: event.currentTarget.value, credentialRef: null })}
      onblur={() => void save()}
    />
  {/if}

  {#if backend.credentialKind === 'none'}
    <p class="m-0">Available. Choose a model in Profiles.</p>
  {:else}
    <div class="grid min-w-0 gap-1">
      <SettingsInput
        label={backend.credentialOptional ? 'API key (optional)' : 'API key'}
        type="password"
        value={credential}
        disabled={savingDisabled}
        autocomplete="off"
        oninput={(event) => (credential = event.currentTarget.value)}
        onblur={() => void save(credential)}
      />
      {#if readiness}<span class="text-xs font-normal"
          >{readinessLabels[readiness.readiness] ?? 'Unavailable'}</span
        >{/if}
    </div>
  {/if}
</div>
