<script lang="ts">
  import { untrack } from 'svelte';
  import type {
    ModelSettingsDraft,
    ProcessingProfileDraft,
    ProcessingPromptTemplateDefaults,
  } from '$lib/page-types';
  import type {
    InferenceBackendMetadata,
    ProcessingProfileSnapshot,
    ProviderSnapshot,
  } from '$lib/generated/bindings';
  import ModelSettingsEditor from '$lib/components/ModelSettingsEditor.svelte';
  import SettingsSection from '$lib/components/SettingsSection.svelte';
  import SettingsInput from '$lib/components/SettingsInput.svelte';
  import SegmentedControl from '$lib/components/SegmentedControl.svelte';
  import ProcessingConversionSettings from '$lib/components/ProcessingConversionSettings.svelte';
  import ProcessingAdditionalRules from '$lib/components/ProcessingAdditionalRules.svelte';
  import PromptTemplateOverride from '$lib/components/PromptTemplateOverride.svelte';
  import DiagnosticBanner from '$lib/components/DiagnosticBanner.svelte';
  import type { CredentialReadiness, ProfileIssue } from '$lib/processing-profile-state';
  import { useLocalModels } from '$lib/local-models-controller.svelte';
  import {
    defaultProcessingProfileDraft,
    normalizeProcessingProfileDraft,
    processingProfileDraftFromSnapshot,
    processingProfileDraftValidationError,
    assessProfile,
    reconcileProcessingProfileDraft,
  } from '$lib/processing-profile-state';
  import type { ModelCatalogState } from '$lib/model-catalog-controller.svelte';
  import { modelDisplayName, modelsForProvider } from '$lib/model-catalog-state';

  let {
    profile,
    backendMetadata = [],
    inferenceProviders = [],
    inferenceReady = true,
    credentials,
    onOpenProviders,
    disabled = false,
    onSaveProfile,
    embedded = false,
    modelPickerId = 'profile-model',
    modelCatalog,
    advanced = false,
    promptTemplateDefaults,
  }: {
    profile: ProcessingProfileSnapshot;
    backendMetadata?: InferenceBackendMetadata[];
    inferenceProviders?: ProviderSnapshot[];
    inferenceReady?: boolean;
    credentials?: CredentialReadiness;
    onOpenProviders?: () => void;
    disabled?: boolean;
    onSaveProfile: (id: string, draft: ProcessingProfileDraft) => Promise<boolean>;
    embedded?: boolean;
    modelPickerId?: string;
    modelCatalog: ModelCatalogState;
    advanced?: boolean;
    promptTemplateDefaults?: ProcessingPromptTemplateDefaults;
  } = $props();
  let draft = $state(defaultProcessingProfileDraft(''));
  let baseline = $state(defaultProcessingProfileDraft(''));
  let initialized = false;
  let error = $state<string | null>(null);
  let queue: Promise<boolean> = Promise.resolve(true);
  const localModels = useLocalModels();
  const requiresRulesModel = $derived(
    draft.modelSettings?.provider === 'local' &&
      localModels?.controller.state.snapshot.models.find((m) => m.id === draft.modelSettings?.model)
        ?.supportsText === false,
  );
  const executionNotice = $derived(
    inferenceReady
      ? assessProfile(
          draft,
          inferenceProviders,
          localModels && !localModels.controller.state.loading
            ? localModels.controller.state.snapshot.models
            : undefined,
          credentials,
        )
      : null,
  );
  const emptyModel: ModelSettingsDraft = { provider: 'local', model: '', options: null };
  const primaryModelName = $derived.by(() => {
    const settings = draft.modelSettings;
    if (!settings?.model.trim()) return 'No model selected';
    const model = modelsForProvider(
      modelCatalog.snapshot,
      settings.provider,
      'all',
      settings.model,
    ).find((model) => model.id === settings.model);
    return model ? modelDisplayName(model) : settings.model;
  });

  $effect(() => {
    const incoming = processingProfileDraftFromSnapshot(profile);
    untrack(() => {
      draft = initialized ? reconcileProcessingProfileDraft(draft, baseline, incoming) : incoming;
      baseline = incoming;
      initialized = true;
    });
  });

  function recover(issue: ProfileIssue): void {
    if (issue.target === 'providers') {
      onOpenProviders?.();
      return;
    }
    const headingId =
      issue.target === 'model'
        ? `${modelPickerId}-section`
        : `${modelPickerId}-${issue.target}-section`;
    const section = document.getElementById(headingId)?.closest('section');
    const control =
      (issue.target === 'model'
        ? document.getElementById(modelPickerId)
        : issue.target === 'rules'
          ? document.getElementById(`${modelPickerId}-rules`)
          : null) ?? section?.querySelector<HTMLElement>('input, button, select, textarea');
    (control ?? section)?.scrollIntoView({ block: 'center', behavior: 'instant' });
    control?.focus({ preventScroll: true });
  }

  function update(changes: Partial<ProcessingProfileDraft>): void {
    draft = { ...draft, ...changes };
    const requiresSeparate =
      draft.additionalRules.trim() &&
      draft.modelSettings?.provider === 'local' &&
      localModels?.controller.state.snapshot.models.find((m) => m.id === draft.modelSettings?.model)
        ?.supportsText === false;
    if (requiresSeparate)
      draft = { ...draft, additionalRulesSeparatePass: true, rulesModelMode: 'independent' };
  }

  function save(): Promise<boolean> {
    error = processingProfileDraftValidationError(draft);
    if (error || disabled) return Promise.resolve(false);
    const candidate = structuredClone($state.snapshot(draft));
    const previous = structuredClone($state.snapshot(baseline));
    queue = queue
      .then(async () => {
        const next = normalizeProcessingProfileDraft(
          reconcileProcessingProfileDraft(candidate, previous, baseline),
        );
        const saved = await onSaveProfile(profile.id, next);
        if (!saved) error = 'Could not save this profile. Check the settings and try again.';
        return saved;
      })
      .catch(() => {
        error = 'Could not save this profile. Try again.';
        return false;
      });
    return queue;
  }

  function saveModel(
    field: 'modelSettings' | 'rulesModelSettings',
    value: ModelSettingsDraft,
  ): Promise<boolean> {
    update({ [field]: value });
    return save();
  }
</script>

<div
  class={embedded
    ? 'grid min-w-0 gap-4'
    : 'grid min-w-0 gap-4 rounded-control border border-border bg-surface-control p-3'}
>
  {#if error || executionNotice}
    <div class="sticky top-0 z-20 grid gap-2">
      {#if error}<DiagnosticBanner message={error} />{/if}
      {#if executionNotice}
        <DiagnosticBanner
          message={executionNotice.message}
          action={executionNotice.action}
          checking={executionNotice.checking}
          onAction={() => executionNotice && recover(executionNotice)}
        />
      {/if}
    </div>
  {/if}
  <SettingsInput
    label="Name"
    value={draft.name}
    {disabled}
    oninput={(event) => update({ name: event.currentTarget.value })}
    onblur={() => void save()}
  />
  <SettingsSection id={`${modelPickerId}-conversion-section`} title="Conversion" variant="panel">
    <ProcessingConversionSettings
      conversion={draft.conversion}
      applicationModeOverride={draft.applicationModeOverride}
      {advanced}
      {disabled}
      onConversionChange={(conversion) => {
        update({ conversion });
        void save();
      }}
      onApplicationModeChange={(applicationModeOverride) => {
        update({ applicationModeOverride });
        void save();
      }}
    />
    {#if advanced && promptTemplateDefaults}
      <PromptTemplateOverride
        label="Conversion prompt template"
        value={draft.conversionPromptTemplate}
        defaultValue={promptTemplateDefaults.conversion}
        {disabled}
        onInput={(conversionPromptTemplate) => update({ conversionPromptTemplate })}
        onBlur={() => void save()}
        onReset={() => void save()}
      />
    {/if}
  </SettingsSection>
  <SettingsSection id={`${modelPickerId}-section`} title="Model" variant="panel">
    <ModelSettingsEditor
      draft={draft.modelSettings ?? emptyModel}
      providers={inferenceProviders}
      {backendMetadata}
      metadataLoaded={inferenceReady}
      {disabled}
      embedded
      {modelPickerId}
      {advanced}
      modelCatalog={modelCatalog.snapshot}
      modelCatalogLoading={modelCatalog.loading}
      modelCatalogStatus={modelCatalog.status}
      onSave={(value) => saveModel('modelSettings', value)}
    />
  </SettingsSection>
  <SettingsSection id={`${modelPickerId}-rules-section`} title="Rules" variant="panel">
    <ProcessingAdditionalRules
      value={draft.additionalRules}
      separatePass={draft.additionalRulesSeparatePass}
      {disabled}
      automaticSeparatePass={requiresRulesModel}
      onInput={(additionalRules) => update({ additionalRules })}
      onBlur={() => void save()}
      onSeparatePassChange={(additionalRulesSeparatePass) => {
        update({ additionalRulesSeparatePass });
        void save();
      }}
    />
    {#if draft.additionalRulesSeparatePass && (!requiresRulesModel || draft.additionalRules.trim())}
      <SettingsSection
        id={`${modelPickerId}-rules-model-section`}
        title="Rules model"
        variant="nested"
      >
        <fieldset class="w-full min-w-0" {disabled}>
          <legend class="mb-1 text-xs font-semibold">Model usage</legend>
          <SegmentedControl
            aria-label="Model usage"
            value={draft.rulesModelMode}
            {disabled}
            fullWidth
            options={[
              {
                value: 'same',
                label: 'Use the same model',
                disabled: requiresRulesModel,
              },
              { value: 'independent', label: 'Use a different model' },
            ]}
            onChange={(value) => {
              update({ rulesModelMode: value === 'same' ? 'same' : 'independent' });
              void save();
            }}
          >
            {#snippet optionContent(option)}
              <span class="grid min-w-0 gap-1">
                <span>{option.label}</span>
                {#if option.value === 'same'}
                  <span class="font-mono font-normal">{primaryModelName}</span>
                {/if}
              </span>
            {/snippet}
          </SegmentedControl>
        </fieldset>
        {#if draft.rulesModelMode === 'independent'}
          <ModelSettingsEditor
            draft={draft.rulesModelSettings ?? emptyModel}
            providers={inferenceProviders}
            {backendMetadata}
            metadataLoaded={inferenceReady}
            {disabled}
            embedded
            modelPickerId={`${modelPickerId}-rules`}
            {advanced}
            modelCapability="text-output"
            modelCatalog={modelCatalog.snapshot}
            modelCatalogLoading={modelCatalog.loading}
            modelCatalogStatus={modelCatalog.status}
            onSave={(value) => saveModel('rulesModelSettings', value)}
          />
        {/if}
        {#if advanced && promptTemplateDefaults}
          <PromptTemplateOverride
            label="Postprocessing prompt template"
            value={draft.postprocessingPromptTemplate}
            defaultValue={promptTemplateDefaults.postprocessing}
            {disabled}
            onInput={(postprocessingPromptTemplate) => update({ postprocessingPromptTemplate })}
            onBlur={() => void save()}
            onReset={() => void save()}
          />
        {/if}
      </SettingsSection>
    {/if}
  </SettingsSection>
</div>
