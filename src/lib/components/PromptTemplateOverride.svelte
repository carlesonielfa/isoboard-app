<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  import ExpandableSettingsRow from '$lib/components/ExpandableSettingsRow.svelte';
  import { promptTemplateOverrideFromValue } from '$lib/processing-profile-state';

  let {
    label,
    value,
    defaultValue,
    disabled = false,
    onInput,
    onBlur,
    onReset,
  }: {
    label: string;
    value: string | null;
    defaultValue: string;
    disabled?: boolean;
    onInput: (value: string | null) => void;
    onBlur?: (event: FocusEvent) => void;
    onReset?: () => void;
  } = $props();

  const displayedValue = $derived(value ?? defaultValue);
  let expanded = $state(false);

  function update(value: string): void {
    onInput(promptTemplateOverrideFromValue(value, defaultValue));
  }

  function reset(): void {
    onInput(null);
    onReset?.();
  }
</script>

<ExpandableSettingsRow
  title={label}
  subtitle={value === null ? 'Using default' : 'Custom override'}
  {expanded}
  configured={value !== null}
  showChevron
  {disabled}
  ariaLabel={`${expanded ? 'Collapse' : 'Edit'} ${label}`}
  onToggle={() => (expanded = !expanded)}
>
  {#snippet actions()}
    {#if value !== null}
      <Button
        variant="ghost"
        size="default"
        type="button"
        class="h-7 min-h-7 px-2 py-1 text-xs font-normal"
        {disabled}
        onclick={reset}>Reset to default</Button
      >
    {/if}
  {/snippet}
  <textarea
    class="box-border min-h-48 min-w-0 resize-y rounded-control border border-border bg-surface px-3 py-2 font-mono text-xs font-normal leading-relaxed text-ink outline-none transition-[background-color,border-color,box-shadow] duration-150 ease-in-out focus:border-border-strong focus:ring-2 focus:ring-focus/30 disabled:cursor-not-allowed disabled:opacity-45"
    rows="10"
    {disabled}
    aria-label={label}
    value={displayedValue}
    oninput={(event) => update(event.currentTarget.value)}
    onblur={onBlur}></textarea>
</ExpandableSettingsRow>
