<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { LucideIcon } from '@lucide/svelte';

  let {
    title,
    icon: GroupIcon,
    children,
    footer,
  }: {
    title: string;
    /** Lucide icon shown before the title. */
    icon?: LucideIcon;
    children: Snippet;
    footer?: Snippet;
  } = $props();

  const id = $props.id();
</script>

<section class="grid min-w-0 gap-2" aria-labelledby={id}>
  <h3 {id} class="m-0 flex items-center gap-2 px-4 pb-1.5 font-display text-sm font-semibold">
    {#if GroupIcon}<GroupIcon class="size-4 shrink-0" strokeWidth={1.75} aria-hidden="true" />{/if}
    {title}
  </h3>
  <div
    class="grid min-w-0 divide-y divide-border rounded-control border border-border bg-surface-control"
  >
    {@render children()}
  </div>
  {#if footer}
    <div
      class="flex min-w-0 flex-wrap items-center gap-x-5 gap-y-1 px-4 [&_button]:text-xs [&_button]:font-medium"
    >
      {@render footer()}
    </div>
  {/if}
</section>
