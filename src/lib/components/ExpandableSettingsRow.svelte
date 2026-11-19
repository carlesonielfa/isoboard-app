<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';

  let {
    title,
    subtitle,
    expanded,
    active,
    configured = false,
    disabled = false,
    ariaLabel,
    onToggle,
    leading,
    actions,
    children,
    showChevron = false,
  }: {
    title: string;
    subtitle?: string;
    expanded: boolean;
    active?: boolean;
    configured?: boolean;
    disabled?: boolean;
    ariaLabel?: string;
    onToggle: () => void;
    leading?: Snippet;
    actions?: Snippet;
    children: Snippet;
    showChevron?: boolean;
  } = $props();

  const stateClasses = $derived(
    active
      ? 'border-border-strong bg-ink/10'
      : configured
        ? 'border-success bg-surface-control'
        : 'border-border bg-surface-control',
  );
</script>

<div class={`grid min-w-0 rounded-control border ${stateClasses}`}>
  <div class="flex min-w-0 items-center gap-2 px-1">
    <button
      type="button"
      class="flex min-h-10 min-w-0 flex-1 items-center gap-3 rounded-control px-2 py-2 text-left text-sm font-semibold text-ink transition-[background-color,border-color] duration-150 ease-in-out motion-reduce:transition-none enabled:cursor-pointer disabled:cursor-not-allowed disabled:opacity-45"
      {disabled}
      aria-expanded={expanded}
      aria-pressed={active}
      aria-label={ariaLabel ?? title}
      onclick={onToggle}
    >
      {#if active !== undefined}
        <span
          class={`size-2 shrink-0 rounded-full border border-border-strong ${active ? 'bg-ink' : ''}`}
          aria-hidden="true"
        ></span>
      {/if}
      {#if leading}{@render leading()}{/if}
      <span class="min-w-0 truncate">{title}</span>
      {#if subtitle}<span class="min-w-0 truncate text-xs font-normal">{subtitle}</span>{/if}
      {#if showChevron}
        <Icon
          name="chevron"
          class={`ml-auto transition-transform duration-150 motion-reduce:transition-none ${expanded ? 'rotate-180' : ''}`}
        />
      {/if}
    </button>

    {#if actions}{@render actions()}{/if}
  </div>

  {#if expanded}
    <div class="grid min-w-0 gap-3 px-3 py-3">
      {@render children()}
    </div>
  {/if}
</div>
