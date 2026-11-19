<script lang="ts">
  import SettingsInput from '$lib/components/SettingsInput.svelte';
  import SettingsSelect from '$lib/components/SettingsSelect.svelte';
  import StatusMessage from '$lib/components/StatusMessage.svelte';
  import type { InferenceOptionsForm } from '$lib/inference-profile-state';
  import type { ReasoningLevel } from '$lib/generated/bindings';
  import { resolveModelReasoningControls, type InferenceModel } from '$lib/model-catalog-state';

  let {
    value,
    model = null,
    advanced = false,
    reasoningOnly = false,
    disabled = false,
    errors = {},
    onChange,
    onCommit,
  }: {
    value: InferenceOptionsForm;
    model?: InferenceModel | null;
    advanced?: boolean;
    reasoningOnly?: boolean;
    disabled?: boolean;
    errors?: Partial<Record<keyof InferenceOptionsForm, string>>;
    onChange: (changes: Partial<InferenceOptionsForm>) => void;
    onCommit: () => void | Promise<unknown>;
  } = $props();

  const controls = $derived(
    resolveModelReasoningControls(
      model ?? { reasoning: null, reasoningOptions: null, temperature: null },
    ),
  );

  function reasoningLabel(value: ReasoningLevel | null): string {
    if (value === null) return 'Not set (provider default)';
    if (value === 'on') return 'On';
    if (value === 'off') return 'Off';
    if (value === 'none') return 'None';
    return value.charAt(0).toUpperCase() + value.slice(1);
  }

  const reasoningValues: Array<ReasoningLevel | null> = $derived(controls.effortChoices);
  const reasoningOptions = $derived.by(() => {
    const options = reasoningValues.map((choice) => ({
      value: choice ?? '',
      label: reasoningLabel(choice),
    }));
    if (!options.some((option) => option.value === '')) {
      options.unshift({ value: '', label: reasoningLabel(null) });
    }
    if (value.reasoning !== null && !reasoningValues.includes(value.reasoning)) {
      options.push({
        value: value.reasoning,
        label: `${reasoningLabel(value.reasoning)} (unavailable for this model)`,
      });
    }
    return options;
  });
  const showReasoning = $derived(reasoningOnly && controls.visible);
  const hiddenOptionError = $derived.by(() => {
    const error = Object.values(errors).find((candidate) => Boolean(candidate));
    if (!error) return null;
    return !advanced || (!controls.temperatureAvailable && Boolean(errors.temperature))
      ? error
      : null;
  });

  function updateNumeric(
    key: keyof Pick<
      InferenceOptionsForm,
      'temperature' | 'maxOutputTokens' | 'requestTimeoutSeconds'
    >,
    event: Event,
  ): void {
    onChange({
      [key]: event.currentTarget instanceof HTMLInputElement ? event.currentTarget.value : '',
    });
  }
</script>

{#if showReasoning}
  <SettingsSelect
    label="Reasoning"
    value={value.reasoning ?? ''}
    disabled={disabled || !controls.visible}
    options={reasoningOptions}
    onValueChange={(next) => {
      onChange({ reasoning: next === '' ? null : (next as ReasoningLevel) });
      void onCommit();
    }}
  />
  {#if value.reasoning === 'on' || value.reasoning === 'off'}
    <StatusMessage tone="notice">
      The saved On/Off reasoning value is unavailable in the current runtime. Not set uses the
      provider default.
    </StatusMessage>
  {/if}
{/if}

{#if !reasoningOnly && advanced && controls.temperatureAvailable}
  <SettingsInput
    label="Temperature"
    type="text"
    value={value.temperature}
    {disabled}
    inputmode="decimal"
    placeholder="Provider default"
    errorId="inference-temperature-error"
    error={errors.temperature}
    oninput={(event) => updateNumeric('temperature', event)}
    onblur={() => void onCommit()}
  />
{/if}

{#if !reasoningOnly && advanced}
  <div class="grid min-w-0 gap-3 sm:grid-cols-2">
    <SettingsInput
      label="Max output tokens"
      type="text"
      value={value.maxOutputTokens}
      {disabled}
      inputmode="numeric"
      placeholder="Provider default"
      errorId="inference-max-output-error"
      error={errors.maxOutputTokens}
      oninput={(event) => updateNumeric('maxOutputTokens', event)}
      onblur={() => void onCommit()}
    />
    <SettingsInput
      label="Request timeout (seconds)"
      type="text"
      value={value.requestTimeoutSeconds}
      {disabled}
      inputmode="numeric"
      placeholder="120 (default)"
      errorId="inference-timeout-error"
      error={errors.requestTimeoutSeconds}
      oninput={(event) => updateNumeric('requestTimeoutSeconds', event)}
      onblur={() => void onCommit()}
    />
  </div>
{/if}

{#if !reasoningOnly && hiddenOptionError}
  <StatusMessage tone="error">
    {hiddenOptionError} Enable Advanced settings to edit this value.
  </StatusMessage>
{/if}
