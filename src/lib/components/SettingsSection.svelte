<script lang="ts">
  import type { Snippet } from 'svelte';
  import InfoLink, { type InfoLinkTarget } from '$lib/components/InfoLink.svelte';

  let {
    id,
    title,
    children,
    variant,
    infoLink,
  }: {
    id: string;
    title: string;
    children: Snippet;
    variant: 'panel' | 'nested';
    /** Adds an info button after the title that opens related documentation. */
    infoLink?: InfoLinkTarget;
  } = $props();
</script>

<section
  class={variant === 'panel'
    ? 'flex min-w-0 flex-col gap-3 rounded-control border border-border bg-surface-control p-3'
    : 'flex min-w-0 flex-col gap-3 border-l-2 border-border pl-3'}
  aria-labelledby={id}
>
  <h3 {id} class="m-0 flex items-center gap-1 text-sm font-semibold">
    {title}
    {#if infoLink}<InfoLink {...infoLink} />{/if}
  </h3>
  <div class="grid min-w-0 gap-3">
    {@render children()}
  </div>
</section>
