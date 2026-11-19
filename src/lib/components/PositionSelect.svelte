<script lang="ts">
  import { ChevronDown } from '@lucide/svelte';
  import * as Popover from '$lib/components/ui/popover';
  import { selectionTriggerClasses } from '$lib/components/ui/menu-styles';
  import PositionGrid, { type PositionGridOption } from '$lib/components/PositionGrid.svelte';

  let {
    label,
    value,
    options,
    disabled = false,
    onValueChange,
  }: {
    label: string;
    value: string;
    options: readonly PositionGridOption[];
    disabled?: boolean;
    onValueChange: (value: string) => void | Promise<void>;
  } = $props();

  let open = $state(false);
  let saving = $state(false);
  let previewValue = $state<string | null>(null);
  let grid = $state<PositionGrid>();
  // svelte-ignore state_referenced_locally
  let selectedValue = $state(value);
  const displayValue = $derived(open ? (previewValue ?? selectedValue) : selectedValue);
  const displayIndex = $derived(
    Math.max(
      0,
      options.findIndex((option) => option.value === displayValue),
    ),
  );
  const displayLabel = $derived(options[displayIndex]?.label ?? 'Select a position');

  $effect(() => {
    if (!open) previewValue = null;
  });

  $effect(() => {
    const authoritativeValue = value;
    if (!saving) selectedValue = authoritativeValue;
  });

  async function select(nextValue: string): Promise<void> {
    if (disabled || saving) return;
    open = false;
    if (nextValue === selectedValue) return;
    selectedValue = nextValue;
    saving = true;
    try {
      await onValueChange(nextValue);
    } catch {
      // The caller reports errors; restore its authoritative value after saving.
    } finally {
      selectedValue = value;
      saving = false;
    }
  }
</script>

<Popover.Root bind:open>
  <Popover.Trigger
    {disabled}
    aria-label={`${label}: ${displayLabel}`}
    aria-busy={saving}
    data-size="default"
    class={selectionTriggerClasses}
    onkeydown={(event) => {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        open = true;
      }
    }}
  >
    <span class="flex min-w-0 items-center gap-2">
      <svg
        aria-hidden="true"
        class="size-5"
        viewBox="0 0 24 20"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
      >
        <rect x="1" y="1" width="22" height="18" rx="2" />
        <rect
          x={4 + (displayIndex % 3) * 5}
          y={4 + Math.floor(displayIndex / 3) * 4}
          width="6"
          height="4"
          rx="1"
          fill="currentColor"
          stroke="none"
        />
      </svg>
      <span class="truncate">{displayLabel}</span>
    </span>
    <ChevronDown class="size-4" />
  </Popover.Trigger>
  <Popover.Content
    class="w-(--bits-popover-anchor-width) overflow-hidden p-0"
    align="start"
    collisionPadding={8}
    aria-label={label}
    onOpenAutoFocus={(event) => {
      event.preventDefault();
      grid?.focusSelected();
    }}
  >
    <PositionGrid
      bind:this={grid}
      {label}
      value={selectedValue}
      {options}
      disabled={disabled || saving}
      onValueChange={(nextValue) => void select(nextValue)}
      onPreviewChange={(value) => (previewValue = value)}
    />
  </Popover.Content>
</Popover.Root>
