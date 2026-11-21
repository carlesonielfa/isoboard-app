<script lang="ts">
  import { tick, type Snippet } from 'svelte';
  import * as Combobox from '$lib/components/ui/combobox';
  import { modelPickerDisplayValue, resolveModelPickerModel } from '$lib/model-picker-state';
  import ModelTierIcon from '$lib/components/ModelTierIcon.svelte';
  import type { BuiltinProfileMetadata } from '$lib/generated/bindings';
  import {
    modelCatalogStatus,
    modelDisplayName,
    modelsForProvider,
    providerHasCatalog,
    type InferenceModelCatalogSnapshot,
    type ModelCapabilityFilter,
    type ModelCatalogStatus,
    type InferenceModel,
  } from '$lib/model-catalog-state';

  let {
    label = 'Model',
    id = 'inference-model-picker',
    value,
    provider,
    catalog = null,
    catalogLoading = false,
    catalogStatus,
    disabled = false,
    invalid = false,
    capability = 'vision-text',
    builtInModels = [],
    modelDescriptions = {},
    optionIndicator,
    optionFilter,
    showModelId = true,
    actions,
    onValueChange,
    onBlur,
  }: {
    label?: string;
    id?: string;
    value: string;
    provider: string;
    catalog?: InferenceModelCatalogSnapshot | null;
    catalogLoading?: boolean;
    catalogStatus?: ModelCatalogStatus;
    disabled?: boolean;
    invalid?: boolean;
    capability?: ModelCapabilityFilter;
    builtInModels?: BuiltinProfileMetadata[];
    modelDescriptions?: Record<string, string>;
    optionIndicator?: Snippet<[InferenceModel]>;
    optionFilter?: (model: InferenceModel) => boolean;
    showModelId?: boolean;
    actions?: Snippet;
    onValueChange: (value: string) => void;
    onBlur?: () => Promise<void> | void;
  } = $props();

  let open = $state(false);
  let editing = $state(false);
  let searchQuery = $state('');
  let inputDirty = $state(false);
  let pendingBlurSave: Promise<void> | null = null;
  let inputElement = $state<HTMLInputElement | null>(null);
  const fieldId = $derived(id);
  const status = $derived(catalogStatus ?? modelCatalogStatus(catalog, catalogLoading));
  const manualOnly = $derived(
    !catalogLoading &&
      status !== 'loading' &&
      (provider === 'openai_compatible' ||
        (!providerHasCatalog(catalog, provider) && builtInModels.length === 0)),
  );
  const normalizedSearch = $derived(searchQuery.trim().toLocaleLowerCase());
  // Trailing actions describe the committed model, so they would overlap the
  // placeholder or the query while nothing is selected or the user searches.
  const showActions = $derived(Boolean(actions) && Boolean(value.trim()) && !editing && !open);
  const options = $derived.by(() => {
    const catalogModels = modelsForProvider(catalog, provider, capability, value);
    const catalogById = new Map(catalogModels.map((model) => [model.id, model]));
    const featured = builtInModels.map(
      (profile): InferenceModel =>
        catalogById.get(profile.model) ?? {
          id: profile.model,
          name: profile.model,
          family: null,
          reasoning: null,
          temperature: null,
          reasoningOptions: null,
          releaseDate: null,
          lastUpdated: null,
          inputModalities: ['text', 'image'],
          outputModalities: ['text'],
          deprecated: false,
          provider,
        },
    );
    return [
      ...featured,
      ...catalogModels.filter((model) => !builtInModels.some((item) => item.model === model.id)),
    ];
  });
  const selectedModel = $derived(options.find((model) => model.id === value));
  const displayValue = $derived(
    editing || open
      ? searchQuery
      : showModelId
        ? modelPickerDisplayValue(selectedModel, value)
        : selectedModel
          ? modelDisplayName(selectedModel)
          : value,
  );
  const matchingOptions = $derived(
    options.filter(
      (model) =>
        (!optionFilter || optionFilter(model)) &&
        (!normalizedSearch ||
          modelDisplayName(model).toLocaleLowerCase().includes(normalizedSearch) ||
          model.id.toLocaleLowerCase().includes(normalizedSearch)),
    ),
  );

  function beginEditing(): void {
    if (disabled) return;
    editing = true;
    open = true;
    searchQuery = '';
    inputDirty = false;
  }

  async function choose(modelId: string): Promise<void> {
    if (pendingBlurSave) await pendingBlurSave;
    onValueChange(modelId);
    const model = options.find((candidate) => candidate.id === modelId);
    searchQuery = modelPickerDisplayValue(model, modelId);
    inputDirty = false;
    open = false;
    editing = false;
    const save = triggerBlurSave();
    await tick();
    inputElement?.focus();
    await save;
  }

  async function commitInput(force = false): Promise<void> {
    if (!editing && !open) return;
    if (!inputDirty) {
      open = false;
      editing = false;
      return;
    }
    const entered = searchQuery;
    const resolvedModelId = resolveModelPickerModel(entered, options);
    // A query with visible catalog matches stays transient on blur; Enter makes
    // committing an arbitrary value explicit.
    const directEntry =
      force || entered === '' || Boolean(resolvedModelId) || matchingOptions.length === 0;
    const nextValue = resolvedModelId ?? entered;
    const changed = directEntry && nextValue !== value;
    if (changed) onValueChange(nextValue);
    open = false;
    editing = false;
    const save = triggerBlurSave();
    await save;
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter' && open && !inputElement?.getAttribute('aria-activedescendant')) {
      event.preventDefault();
      void commitInput(true);
    } else if (event.key === 'Escape') {
      if (open) {
        event.preventDefault();
        searchQuery = value;
        inputDirty = false;
        open = false;
        editing = false;
      }
    }
  }

  function triggerBlurSave(): Promise<void> {
    const save = Promise.resolve(onBlur?.()).catch(() => undefined);
    pendingBlurSave = save;
    void save.finally(() => {
      if (pendingBlurSave === save) pendingBlurSave = null;
    });
    return save;
  }

  function handleBitsValueChange(modelId: string): void {
    void choose(modelId);
  }
</script>

<div class="grid min-w-0 gap-1 text-xs font-semibold">
  <label for={fieldId}>{label}</label>
  <Combobox.Root
    type="single"
    {value}
    inputValue={displayValue}
    bind:open
    {disabled}
    allowDeselect={false}
    onValueChange={handleBitsValueChange}
  >
    <div class="relative min-w-0">
      <Combobox.Input
        bind:ref={inputElement}
        id={fieldId}
        type="text"
        class={`h-10 w-full rounded-control border border-border bg-surface-control px-3 text-sm font-normal text-foreground outline-none placeholder:text-foreground focus:border-border-strong focus:ring-2 focus:ring-focus aria-invalid:border-danger-border aria-invalid:ring-2 aria-invalid:ring-danger/30 disabled:cursor-not-allowed disabled:opacity-60 ${showActions ? 'pr-12' : ''}`}
        {disabled}
        style={!editing && !open && modelDescriptions[value] ? 'color: transparent' : undefined}
        aria-describedby={modelDescriptions[value] ? `${fieldId}-size` : undefined}
        aria-invalid={invalid || undefined}
        aria-controls={`${fieldId}-options`}
        aria-haspopup="listbox"
        autocomplete="off"
        spellcheck="false"
        placeholder="Search model name or input manual model ID"
        onfocus={beginEditing}
        onclick={() => {
          if (!open) beginEditing();
        }}
        oninput={(event) => {
          if (!editing || !open) {
            editing = true;
            open = true;
          }
          searchQuery = event.currentTarget.value;
          inputDirty = true;
        }}
        onkeydown={handleKeydown}
        onblur={() => {
          // Allow a pointer click on an option to run before closing the list.
          void commitInput();
        }}
      />

      {#if !editing && !open && modelDescriptions[value]}
        <span
          aria-hidden="true"
          class={`pointer-events-none absolute inset-0 flex min-w-0 items-center gap-1 overflow-hidden px-3 text-sm font-normal ${showActions ? 'pr-12' : ''} ${disabled ? 'opacity-60' : ''}`}
        >
          <span class="min-w-0 truncate"
            >{selectedModel ? modelDisplayName(selectedModel) : value}</span
          >
          <span class="shrink-0 text-foreground/60">– {modelDescriptions[value]}</span>
        </span>
      {/if}
      {#if modelDescriptions[value]}<span id={`${fieldId}-size`} class="sr-only"
          >Download size: {modelDescriptions[value]}</span
        >{/if}
      {#if showActions && actions}<div class="absolute inset-y-0 right-1 flex items-center">
          {@render actions()}
        </div>{/if}
    </div>

    <Combobox.Content
      id={`${fieldId}-options`}
      aria-label={`${label} options`}
      side="bottom"
      align="start"
      collisionPadding={8}
      strategy="fixed"
    >
      {#if matchingOptions.length > 0}
        {#each matchingOptions as model (model.id)}
          {@const builtIn = builtInModels.find((item) => item.model === model.id)}
          <Combobox.Item
            value={model.id}
            label={modelDisplayName(model)}
            class="items-stretch gap-2"
          >
            <span class="grid min-w-0 flex-1 gap-0.5">
              <span
                class="flex w-full min-w-0 items-center gap-1.5 text-sm font-semibold text-foreground"
              >
                {#if builtIn}
                  <ModelTierIcon tier={builtIn.tier} explanation={builtIn.explanation} />
                {/if}
                <span class="min-w-0 truncate">{modelDisplayName(model)}</span>
                {#if modelDescriptions[model.id]}<span
                    class="shrink-0 font-normal text-foreground/60"
                    >– {modelDescriptions[model.id]}</span
                  >{/if}
              </span>
              <span class="block w-full min-w-0 truncate text-xs font-normal text-foreground/70"
                >{model.id}</span
              >
            </span>
            {#if optionIndicator}
              <span class="flex w-5 shrink-0 items-center justify-center"
                >{@render optionIndicator(model)}</span
              >
            {/if}
          </Combobox.Item>
        {/each}
      {:else}
        <div class="px-2 py-2 text-sm font-normal text-foreground" role="status">
          {#if status === 'loading'}Loading model catalog…{:else if manualOnly}Manual model ID for
            this provider.{:else}No matching catalog models. Enter an ID directly.{/if}
        </div>
      {/if}
    </Combobox.Content>
  </Combobox.Root>
</div>
