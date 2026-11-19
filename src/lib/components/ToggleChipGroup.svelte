<script lang="ts">
  import * as ToggleGroup from '$lib/components/ui/toggle-group/index.js';

  export type ToggleChipOption = { value: string; label: string };

  let {
    options,
    value,
    disabled = false,
    fullWidth = false,
    'aria-label': ariaLabel,
    onChange,
  }: {
    options: readonly ToggleChipOption[];
    /** The values switched on, in any order. */
    value: readonly string[];
    disabled?: boolean;
    /** Spreads the chips evenly across the row instead of wrapping them. */
    fullWidth?: boolean;
    'aria-label': string;
    /** Receives the switched-on values in `options` order. */
    onChange: (value: string[]) => void;
  } = $props();

  const selected = $derived([...value]);

  function handleChange(next: string[]): void {
    onChange(options.map((option) => option.value).filter((option) => next.includes(option)));
  }
</script>

<ToggleGroup.Root
  type="multiple"
  value={selected}
  onValueChange={handleChange}
  {disabled}
  aria-label={ariaLabel}
  class={fullWidth ? 'flex gap-1' : 'flex flex-wrap gap-1'}
>
  {#each options as option (option.value)}
    <ToggleGroup.Item
      value={option.value}
      class="{fullWidth
        ? 'min-w-0 flex-1 px-1'
        : 'px-2.5'} min-h-7 rounded-control border border-border bg-surface text-xs text-ink outline-none transition-colors duration-150 enabled:hover:bg-surface-raised focus-visible:ring-2 focus-visible:ring-focus/50 disabled:cursor-not-allowed disabled:opacity-45 data-[state=on]:border-highlight data-[state=on]:bg-highlight data-[state=on]:text-canvas motion-reduce:transition-none"
    >
      {option.label}
    </ToggleGroup.Item>
  {/each}
</ToggleGroup.Root>
