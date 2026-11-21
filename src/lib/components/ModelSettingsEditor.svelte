<script lang="ts">
  import { HardDrive } from '@lucide/svelte';
  import type { ModelSettingsDraft } from '$lib/page-types';
  import type { InferenceBackend, InferenceBackendMetadata } from '$lib/generated/bindings';
  import type { ProviderSnapshot } from '$lib/generated/bindings';
  import type { InferenceOptionsForm } from '$lib/inference-profile-state';
  import SettingsSelect from '$lib/components/SettingsSelect.svelte';
  import { useLocalModels } from '$lib/local-models-controller.svelte';
  import { modelSize } from '$lib/model-size';
  import LocalModelControls from '$lib/components/LocalModelControls.svelte';
  import { localModelSuggested } from '$lib/model-picker-state';
  import ModelPicker from '$lib/components/ModelPicker.svelte';
  import type {
    InferenceModelCatalogSnapshot,
    InferenceModel,
    ModelCapabilityFilter,
    ModelCatalogStatus,
  } from '$lib/model-catalog-state';
  import {
    INFERENCE_BACKEND_LABELS,
    inferenceOptionsFormFromValue,
    inferenceOptionsFromForm,
    providerConnected,
    reconcileModelSettingsDraft,
  } from '$lib/inference-profile-state';
  import { createInferenceProviderChangeController } from '$lib/inference-provider-state';
  import InferenceProfileOptions from '$lib/components/InferenceProfileOptions.svelte';
  import { modelsForProvider } from '$lib/model-catalog-state';
  import { builtInProfileMetadata } from '$lib/builtin-inference-profiles';

  let {
    draft: initialDraft,
    providers,
    backendMetadata = [],
    metadataLoaded = true,
    disabled = false,
    onSave,
    embedded = false,
    modelPickerId = 'inference-model-picker',
    modelCatalog = null,
    modelCatalogLoading = false,
    modelCatalogStatus,
    modelCapability = 'vision-text',
    requireTextSupport = false,
    advanced = false,
  }: {
    draft: ModelSettingsDraft;
    providers: ProviderSnapshot[];
    backendMetadata?: InferenceBackendMetadata[];
    metadataLoaded?: boolean;
    disabled?: boolean;
    onSave: (draft: ModelSettingsDraft) => Promise<boolean>;
    embedded?: boolean;
    modelPickerId?: string;
    modelCatalog?: InferenceModelCatalogSnapshot | null;
    modelCatalogLoading?: boolean;
    modelCatalogStatus?: ModelCatalogStatus;
    modelCapability?: ModelCapabilityFilter;
    /** Hide local models that only extract from images, such as isOCR. */
    requireTextSupport?: boolean;
    advanced?: boolean;
  } = $props();

  let draft = $state<ModelSettingsDraft>({
    provider: 'openai_api',
    model: '',
    options: null,
  });
  let optionsForm = $state<InferenceOptionsForm>(inferenceOptionsFormFromValue(null));
  let optionErrors = $state<Partial<Record<keyof InferenceOptionsForm, string>>>({});
  let lastInitialDraftKey = $state('');
  let previousInitialDraft = $state<ModelSettingsDraft | null>(null);
  let saveInFlight = $state(false);
  let queuedSaves: Array<{
    draft: ModelSettingsDraft;
    baseline: ModelSettingsDraft;
    waiters: Array<(saved: boolean) => void>;
  }> = [];
  $effect(() => {
    const incoming = { ...initialDraft, options: initialDraft.options ?? null };
    const key = JSON.stringify(incoming);
    if (key === lastInitialDraftKey) return;
    const previous = previousInitialDraft;
    const previousOptions = inferenceOptionsFormFromValue(previous?.options);
    const incomingOptions = inferenceOptionsFormFromValue(incoming.options);
    draft = previous ? reconcileModelSettingsDraft(draft, previous, incoming) : { ...incoming };
    if (!previous) {
      optionsForm = incomingOptions;
      optionErrors = {};
    } else {
      optionsForm = {
        temperature:
          optionsForm.temperature === previousOptions.temperature
            ? incomingOptions.temperature
            : optionsForm.temperature,
        maxOutputTokens:
          optionsForm.maxOutputTokens === previousOptions.maxOutputTokens
            ? incomingOptions.maxOutputTokens
            : optionsForm.maxOutputTokens,
        requestTimeoutSeconds:
          optionsForm.requestTimeoutSeconds === previousOptions.requestTimeoutSeconds
            ? incomingOptions.requestTimeoutSeconds
            : optionsForm.requestTimeoutSeconds,
        reasoning:
          optionsForm.reasoning === previousOptions.reasoning
            ? incomingOptions.reasoning
            : optionsForm.reasoning,
      };
    }
    previousInitialDraft = { ...incoming };
    providerChangeController.sync(draft.provider);
    lastInitialDraftKey = key;
  });

  const localModels = useLocalModels();
  function localModelVisible(model: InferenceModel): boolean {
    return localModelSuggested(
      localModels?.controller.state.snapshot.models.find((item) => item.id === model.id),
      localModels?.uiMode() ?? 'basic',
      requireTextSupport,
    );
  }

  const modelDescriptions = $derived(
    draft.provider === 'local'
      ? Object.fromEntries(
          (localModels?.controller.state.snapshot.models ?? [])
            .filter((model) => model.totalBytes > 0)
            .map((model) => [model.id, modelSize(model.totalBytes)]),
        )
      : {},
  );
  const downloadedModels = $derived(
    (localModels?.controller.state.snapshot.models ?? [])
      .filter((model) => model.downloaded)
      .map((model) => model.id),
  );
  const savingDisabled = $derived(disabled || !metadataLoaded);
  const configuredProviders = $derived(providers.filter(providerConnected));
  const providerConfigured = $derived(
    configuredProviders.some((provider) => provider.backend === draft.provider),
  );
  const selectedModel = $derived(
    modelsForProvider(modelCatalog, draft.provider, 'all', draft.model).find(
      (candidate) => candidate.id === draft.model,
    ) ?? null,
  );

  function updateDraft(changes: Partial<ModelSettingsDraft>): void {
    draft = { ...draft, ...changes };
  }

  function updateOptions(changes: Partial<InferenceOptionsForm>): void {
    optionsForm = { ...optionsForm, ...changes };
    optionErrors = {};
  }

  function draftForSave(): ModelSettingsDraft | null {
    const parsed = inferenceOptionsFromForm(optionsForm);
    if (parsed.error) {
      const nextErrors: Partial<Record<keyof InferenceOptionsForm, string>> = {};
      if (parsed.error.startsWith('Temperature')) nextErrors.temperature = parsed.error;
      else if (parsed.error.startsWith('Max output')) nextErrors.maxOutputTokens = parsed.error;
      else nextErrors.requestTimeoutSeconds = parsed.error;
      optionErrors = nextErrors;
      return null;
    }
    return { ...draft, options: parsed.options };
  }

  async function drainSaves(): Promise<void> {
    if (saveInFlight) return;
    saveInFlight = true;
    let lastResult = false;
    try {
      while (queuedSaves.length > 0) {
        const entry = queuedSaves.shift()!;
        const authoritative = previousInitialDraft ?? entry.baseline;
        const candidate = reconcileModelSettingsDraft(entry.draft, entry.baseline, authoritative);
        try {
          lastResult = await onSave({ ...candidate });
        } catch {
          lastResult = false;
        }
        if (lastResult) {
          const currentOptions = inferenceOptionsFromForm(optionsForm).options;
          if (JSON.stringify(currentOptions) === JSON.stringify(candidate.options ?? null)) {
            draft = { ...draft, options: candidate.options ?? null };
          }
        }
        entry.waiters.forEach((resolve) => resolve(lastResult));
      }
    } finally {
      saveInFlight = false;
    }
  }

  const providerChangeController = createInferenceProviderChangeController((provider) => {
    updateDraft({ provider });
  });

  function save(): Promise<boolean> {
    if (savingDisabled || !providerConfigured || !draft.model.trim()) {
      return Promise.resolve(false);
    }
    const candidate = draftForSave();
    if (!candidate) return Promise.resolve(false);
    const baseline = previousInitialDraft ?? {
      ...initialDraft,
      options: initialDraft.options ?? null,
    };
    const authoritative = { ...baseline, options: baseline.options ?? null };
    if (
      !saveInFlight &&
      queuedSaves.length === 0 &&
      JSON.stringify(candidate) === JSON.stringify(authoritative)
    ) {
      return Promise.resolve(true);
    }
    const existing = queuedSaves.at(-1);
    if (existing && JSON.stringify(existing.draft) === JSON.stringify(candidate)) {
      return new Promise<boolean>((resolve) => existing.waiters.push(resolve));
    }
    const result = new Promise<boolean>((resolve) =>
      queuedSaves.push({ draft: candidate, baseline: { ...baseline }, waiters: [resolve] }),
    );
    void drainSaves();
    return result;
  }

  async function saveProvider(provider: InferenceBackend): Promise<boolean> {
    if (disabled || !metadataLoaded || !provider || !draft.model.trim()) return false;
    updateDraft({ provider });
    return save();
  }
</script>

<div
  class={embedded
    ? 'grid min-w-0 gap-3'
    : 'grid min-w-0 gap-3 rounded-control border border-border bg-surface-control p-3'}
>
  <div class="grid min-w-0 items-start gap-3 sm:grid-cols-[minmax(0,1fr)_minmax(0,2fr)]">
    <SettingsSelect
      label="Provider"
      value={providerConfigured ? draft.provider : ''}
      invalid={metadataLoaded && !providerConfigured}
      placeholder={configuredProviders.length === 0
        ? 'Set up a provider on the Providers tab'
        : 'Select a configured provider'}
      disabled={disabled || !metadataLoaded || configuredProviders.length === 0}
      options={[
        ...configuredProviders.map((item) => ({
          value: item.backend,
          label: INFERENCE_BACKEND_LABELS[item.backend],
        })),
        ...(configuredProviders.length > 0
          ? [
              {
                value: 'provider-access-instruction',
                label: 'Set up more providers in the Providers tab',
                disabled: true,
              },
            ]
          : []),
      ]}
      onValueChange={async (value) => {
        if (!configuredProviders.some((provider) => provider.backend === value)) return;
        if (!draft.model.trim()) {
          providerChangeController.sync(value as InferenceBackend);
          return;
        }
        await providerChangeController.select(
          value as ModelSettingsDraft['provider'],
          saveProvider,
        );
      }}
    />
    <div class="grid min-w-0 gap-1">
      {#snippet modelActions()}
        <LocalModelControls model={draft.model} disabled={savingDisabled} />
      {/snippet}
      {#snippet modelIndicator(model: InferenceModel)}
        {#if downloadedModels.includes(model.id)}
          <span
            title="Downloaded, available offline"
            role="img"
            aria-label="Downloaded, available offline"
            class="inline-flex text-foreground/50"
          >
            <HardDrive class="size-5" aria-hidden="true" />
          </span>
        {/if}
      {/snippet}
      <ModelPicker
        id={modelPickerId}
        value={draft.model}
        provider={draft.provider}
        {modelDescriptions}
        optionFilter={draft.provider === 'local' ? localModelVisible : undefined}
        optionIndicator={draft.provider === 'local' ? modelIndicator : undefined}
        actions={draft.provider === 'local' && draft.model.trim() ? modelActions : undefined}
        showModelId={draft.provider !== 'local'}
        builtInModels={builtInProfileMetadata(backendMetadata, draft.provider)}
        catalog={modelCatalog}
        catalogLoading={modelCatalogLoading}
        catalogStatus={modelCatalogStatus}
        disabled={savingDisabled}
        invalid={!draft.model.trim()}
        capability={modelCapability}
        onValueChange={(model) => updateDraft({ model })}
        onBlur={async () => {
          await save();
        }}
      />
      {#if draft.provider === 'local' && draft.model.trim()}
        <LocalModelControls model={draft.model} disabled={savingDisabled} feedbackOnly />
      {/if}
    </div>
  </div>

  <div class="grid min-w-0 gap-3 sm:grid-cols-2">
    <InferenceProfileOptions
      value={optionsForm}
      model={selectedModel}
      reasoningOnly
      disabled={savingDisabled}
      onChange={updateOptions}
      onCommit={async () => {
        await save();
      }}
    />
  </div>

  <InferenceProfileOptions
    value={optionsForm}
    model={selectedModel}
    {advanced}
    disabled={savingDisabled}
    errors={optionErrors}
    onChange={updateOptions}
    onCommit={async () => {
      await save();
    }}
  />
</div>
