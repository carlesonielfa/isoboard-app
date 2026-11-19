<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils';

  let {
    label,
    value,
    children,
    class: className = '',
  }: {
    label: string;
    value?: string;
    children?: Snippet;
    class?: string;
  } = $props();
</script>

<div
  class={cn(
    'grid min-h-9 grid-cols-[minmax(8rem,1fr)_minmax(0,1.4fr)] items-center gap-4',
    className,
  )}
>
  <dt class="font-semibold">{label}</dt>
  <dd class="m-0 min-w-0 justify-self-end text-right">
    {#if children}
      {@render children()}
    {:else}
      {value}
    {/if}
  </dd>
</div>
