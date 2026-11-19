<script lang="ts">
  import type { Snippet } from 'svelte';
  import * as Tabs from '$lib/components/ui/tabs/index.js';
  import { cn } from '$lib/utils';

  export type SegmentedControlOption = {
    value: string;
    label: string;
    disabled?: boolean;
  };

  type Props = {
    options: readonly SegmentedControlOption[];
    value: string;
    onChange: (value: string) => void;
    'aria-label': string;
    disabled?: boolean;
    fullWidth?: boolean;
    optionContent?: Snippet<[SegmentedControlOption]>;
    class?: string;
  };

  let {
    options,
    value,
    onChange,
    'aria-label': ariaLabel,
    disabled = false,
    fullWidth = false,
    optionContent,
    class: className = '',
  }: Props = $props();

  let selectedValue = $derived(value);
  let selectedIndex = $derived(
    Math.max(
      0,
      options.findIndex((option) => option.value === value),
    ),
  );

  function handleValueChange(nextValue: string): void {
    if (
      !disabled &&
      !options.find((option) => option.value === nextValue)?.disabled &&
      nextValue !== value
    )
      onChange(nextValue);
  }
</script>

<Tabs.Root
  bind:value={selectedValue}
  orientation="horizontal"
  loop
  {disabled}
  class={cn(fullWidth && 'flex w-full', className)}
  aria-label={ariaLabel}
  onValueChange={handleValueChange}
>
  <Tabs.List
    class={fullWidth
      ? 'relative isolate grid w-full grid-cols-[repeat(var(--segment-count),minmax(0,1fr))] overflow-hidden'
      : 'relative isolate overflow-hidden'}
    aria-label={ariaLabel}
    aria-disabled={disabled}
    style={`--segment-count: ${options.length}; --segment-index: ${selectedIndex};`}
    variant="small"
  >
    <span
      aria-hidden="true"
      class="pointer-events-none absolute inset-y-[var(--tabs-inset)] left-[var(--tabs-inset)] z-0 w-[calc((100%-var(--tabs-total-inset))/var(--segment-count))] rounded-[var(--tabs-inner-radius)] bg-highlight transition-transform duration-150 ease-in-out motion-reduce:transition-none"
      style="transform: translateX(calc(var(--segment-index) * 100%));"
    ></span>

    {#each options as option (option.value)}
      <Tabs.Trigger
        value={option.value}
        disabled={disabled || option.disabled}
        class={fullWidth
          ? 'min-w-0 whitespace-normal break-words disabled:cursor-not-allowed disabled:opacity-45 group-data-[variant=small]/tabs-list:w-full group-data-[variant=small]/tabs-list:px-3 group-data-[variant=small]/tabs-list:py-2 group-data-[variant=small]/tabs-list:text-xs group-data-[variant=small]/tabs-list:leading-snug'
          : undefined}
      >
        {#if optionContent}
          {@render optionContent(option)}
        {:else}
          {option.label}
        {/if}
      </Tabs.Trigger>
    {/each}
  </Tabs.List>
</Tabs.Root>
