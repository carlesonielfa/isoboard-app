<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';
  import InfoLink, { type InfoLinkTarget } from '$lib/components/InfoLink.svelte';

  let {
    label,
    value,
    children,
    class: className = '',
    infoLink,
  }: {
    label: string;
    value?: string;
    children?: Snippet;
    class?: string;
    /** Adds an info button after the label that opens related documentation. */
    infoLink?: InfoLinkTarget;
  } = $props();
</script>

<div
  class={cn(
    'grid min-h-9 grid-cols-[minmax(8rem,1fr)_minmax(0,1.4fr)] items-center gap-4',
    className,
  )}
>
  <dt class="flex min-w-0 items-center gap-1 font-semibold">
    {label}
    {#if infoLink}<InfoLink {...infoLink} />{/if}
  </dt>
  <dd class="m-0 min-w-0 justify-self-end text-right">
    {#if children}
      {@render children()}
    {:else}
      {value}
    {/if}
  </dd>
</div>
