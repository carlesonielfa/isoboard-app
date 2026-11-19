<script lang="ts">
  export type PositionGridOption = { value: string; label: string };

  let {
    label,
    value,
    options,
    disabled = false,
    onValueChange,
    onPreviewChange,
  }: {
    label: string;
    value: string;
    options: readonly PositionGridOption[];
    disabled?: boolean;
    onValueChange: (value: string) => void;
    onPreviewChange?: (value: string | null) => void;
  } = $props();

  let buttons = $state<HTMLButtonElement[]>([]);
  let focusedIndex = $state(0);

  export function focusSelected(): void {
    focusedIndex = Math.max(
      0,
      options.findIndex((option) => option.value === value),
    );
    buttons[focusedIndex]?.focus();
  }

  function navigate(event: KeyboardEvent, index: number): void {
    let next = index;
    switch (event.key) {
      case 'ArrowLeft':
        if (index % 3 > 0) next--;
        break;
      case 'ArrowRight':
        if (index % 3 < 2) next++;
        break;
      case 'ArrowUp':
        if (index >= 3) next -= 3;
        break;
      case 'ArrowDown':
        if (index < 6) next += 3;
        break;
      case 'Home':
        next = 0;
        break;
      case 'End':
        next = 8;
        break;
      default:
        return;
    }
    event.preventDefault();
    if (disabled) return;
    focusedIndex = next;
    buttons[next]?.focus();
  }
</script>

<div
  role="grid"
  tabindex="-1"
  aria-label={label}
  aria-disabled={disabled}
  class="grid aspect-[112/67] w-full gap-px overflow-hidden rounded-control"
  onpointerleave={() => onPreviewChange?.(null)}
  onfocusout={(event) => {
    if (!event.currentTarget.contains(event.relatedTarget as Node | null)) onPreviewChange?.(null);
  }}
>
  {#each [0, 1, 2] as row (row)}
    <div role="row" class="grid grid-cols-3 gap-px">
      {#each options.slice(row * 3, row * 3 + 3) as option, column (option.value)}
        {@const index = row * 3 + column}
        <div role="gridcell">
          <button
            bind:this={buttons[index]}
            type="button"
            {disabled}
            aria-label={option.label}
            aria-pressed={value === option.value}
            tabindex={focusedIndex === index ? 0 : -1}
            class="flex h-full min-h-11 w-full items-center justify-center rounded-none border-0 bg-surface outline-none transition-colors duration-150 enabled:cursor-pointer enabled:hover:bg-surface-raised aria-pressed:bg-highlight-surface focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus disabled:cursor-not-allowed disabled:opacity-45 motion-reduce:transition-none"
            onpointerenter={() => {
              if (!disabled) onPreviewChange?.(option.value);
            }}
            onfocus={() => {
              focusedIndex = index;
              onPreviewChange?.(option.value);
            }}
            onclick={() => onValueChange(option.value)}
            onkeydown={(event) => navigate(event, index)}
          >
            <span
              aria-hidden="true"
              class={value === option.value
                ? 'h-2.5 w-7 rounded-sm bg-highlight'
                : 'h-1.5 w-5 rounded-sm bg-highlight/40'}
            ></span>
          </button>
        </div>
      {/each}
    </div>
  {/each}
</div>
