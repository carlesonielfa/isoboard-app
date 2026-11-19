<script lang="ts">
  import type { ComponentProps, Snippet } from 'svelte';
  import { cn } from '$lib/utils.js';
  import * as Tabs from '$lib/components/ui/tabs/index.js';

  let {
    children,
    activeMarker,
    class: className,
    ...restProps
  }: ComponentProps<typeof Tabs.Trigger> & { children: Snippet; activeMarker?: boolean } = $props();
</script>

<Tabs.Trigger
  class={cn(
    'min-w-0 flex-1 justify-start gap-3 rounded-control border-0 px-2 py-2 text-left text-sm font-normal leading-snug hover:bg-transparent data-[state=active]:bg-transparent data-[state=active]:text-ink',
    className,
  )}
  {...restProps}
>
  {#if activeMarker !== undefined}
    <span
      class={`size-2 shrink-0 rounded-full border border-border-strong ${activeMarker ? 'bg-ink' : ''}`}
      aria-hidden="true"
    ></span>
  {/if}
  {@render children()}
</Tabs.Trigger>
