<script lang="ts">
  import type {
    ConversionSettingsApplicationMode,
    ProcessingConversionSettings,
  } from '$lib/generated/bindings';
  import SettingsSelect from '$lib/components/SettingsSelect.svelte';
  import ConversionOptions from '$lib/components/ConversionOptions.svelte';
  import ConversionSkeletonPreview from '$lib/components/ConversionSkeletonPreview.svelte';

  let {
    conversion,
    applicationModeOverride,
    advanced = false,
    disabled = false,
    onConversionChange,
    onApplicationModeChange,
  }: {
    conversion: ProcessingConversionSettings;
    applicationModeOverride: ConversionSettingsApplicationMode | null;
    advanced?: boolean;
    disabled?: boolean;
    onConversionChange: (conversion: ProcessingConversionSettings) => void;
    onApplicationModeChange: (mode: ConversionSettingsApplicationMode | null) => void;
  } = $props();
</script>

<div class="grid min-w-0 gap-3">
  <fieldset class="min-w-0" {disabled}>
    <legend class="mb-3 text-xs font-semibold">
      Choose which elements to preserve in the converted content.
    </legend>
    <div
      class="mx-auto grid w-full max-w-[23rem] min-w-0 grid-cols-[minmax(0,1fr)_max-content] items-stretch gap-6 pb-2"
    >
      <ConversionSkeletonPreview {conversion} />
      <ConversionOptions
        layout="vertical"
        value={conversion}
        {disabled}
        onChange={onConversionChange}
      />
    </div>
  </fieldset>

  {#if advanced}
    <SettingsSelect
      label="Conversion settings application"
      value={applicationModeOverride ?? ''}
      {disabled}
      options={[
        { value: '', label: 'Automatic — Prompt + cleanup' },
        { value: 'prompt', label: 'Prompt only' },
        { value: 'html_cleanup', label: 'Cleanup only' },
        { value: 'prompt_and_html_cleanup', label: 'Prompt + cleanup' },
      ]}
      onValueChange={(value) =>
        onApplicationModeChange(value ? (value as ConversionSettingsApplicationMode) : null)}
    />
  {/if}
</div>
