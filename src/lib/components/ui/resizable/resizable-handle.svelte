<script lang="ts">
  import * as ResizablePrimitive from 'paneforge';
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    withHandle = false,
    ...restProps
  }: WithoutChildrenOrChild<ResizablePrimitive.PaneResizerProps> & {
    withHandle?: boolean;
  } = $props();
</script>

<ResizablePrimitive.PaneResizer
  bind:ref
  data-slot="resizable-handle"
  class={cn(
    'group relative flex w-px shrink-0 cursor-col-resize items-center justify-center bg-border/60 outline-none after:absolute after:inset-y-0 after:left-1/2 after:w-3 after:-translate-x-1/2 data-[direction=vertical]:h-px data-[direction=vertical]:w-full data-[direction=vertical]:cursor-row-resize data-[direction=vertical]:after:inset-x-0 data-[direction=vertical]:after:top-1/2 data-[direction=vertical]:after:left-0 data-[direction=vertical]:after:h-3 data-[direction=vertical]:after:w-full data-[direction=vertical]:after:translate-x-0 data-[direction=vertical]:after:-translate-y-1/2 [&[data-direction=vertical]>span]:h-1 [&[data-direction=vertical]>span]:w-5',
    className,
  )}
  {...restProps}
>
  {#if withHandle}
    <span
      aria-hidden="true"
      class="pointer-events-none relative z-10 h-5 w-1 rounded-full bg-ink/20 transition-colors group-hover:bg-ink/40 group-focus-visible:bg-ink/50 group-focus-visible:ring-2 group-focus-visible:ring-focus/60 group-data-[active]:bg-ink/50 motion-reduce:transition-none"
    ></span>
  {/if}
</ResizablePrimitive.PaneResizer>
