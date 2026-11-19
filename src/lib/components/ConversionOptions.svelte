<script lang="ts">
  import SettingsSwitch from '$lib/components/SettingsSwitch.svelte';
  import type { ProcessingConversionSettings } from '$lib/generated/bindings';
  let {
    value,
    disabled = false,
    disabledOptions = {},
    layout = 'horizontal',
    onChange,
  }: {
    value: ProcessingConversionSettings;
    disabled?: boolean;
    disabledOptions?: Partial<ProcessingConversionSettings>;
    layout?: 'horizontal' | 'vertical';
    onChange: (value: ProcessingConversionSettings) => void;
  } = $props();
  const options = ['tables', 'colors', 'images'] as const;
</script>

<div
  class={layout === 'vertical'
    ? 'flex flex-col items-start justify-self-center gap-6 py-2'
    : 'flex flex-wrap items-center gap-x-5 gap-y-2'}
>
  {#each options as option (option)}
    <SettingsSwitch
      label={option.charAt(0).toUpperCase() + option.slice(1)}
      checked={value[option]}
      disabled={disabled || disabledOptions[option]}
      onCheckedChange={(checked) => onChange({ ...value, [option]: checked })}
    />
  {/each}
</div>
