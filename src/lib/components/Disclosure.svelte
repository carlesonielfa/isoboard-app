<script lang="ts">
  import type { Snippet } from 'svelte';
  import { ChevronDown } from '@lucide/svelte';

  let {
    label,
    expanded = $bindable(false),
    children,
  }: {
    label: string;
    expanded?: boolean;
    children: Snippet;
  } = $props();

  const id = $props.id();
</script>

<div class="grid min-w-0">
  <div class="flex items-center">
    <button
      type="button"
      class="inline-flex items-center gap-1.5 rounded-sm text-sm font-semibold text-ink outline-none hover:text-ink/80 focus-visible:ring-2 focus-visible:ring-focus/30"
      aria-expanded={expanded}
      aria-controls={id}
      onclick={() => (expanded = !expanded)}
    >
      {label}
      <ChevronDown
        aria-hidden="true"
        class={`size-4 transition-transform duration-150 motion-reduce:transition-none ${expanded ? 'rotate-180' : ''}`}
      />
    </button>
  </div>
  {#if expanded}
    <div {id} class="mt-2 min-w-0 border-l-2 border-border pl-4">
      {@render children()}
    </div>
  {/if}
</div>
