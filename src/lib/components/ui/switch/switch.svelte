<script lang="ts">
  import { Switch as SwitchPrimitive } from 'bits-ui';
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    checked = $bindable(false),
    class: className,
    ...restProps
  }: WithoutChildrenOrChild<SwitchPrimitive.RootProps> = $props();
</script>

<SwitchPrimitive.Root
  bind:ref
  bind:checked
  data-slot="switch"
  class={cn(
    'inline-flex h-5 w-9 shrink-0 items-center rounded-full border border-border bg-surface-control p-0.5 outline-none transition-[background-color,border-color,box-shadow] duration-150 focus-visible:border-focus focus-visible:ring-2 focus-visible:ring-focus/30 disabled:cursor-not-allowed data-[state=checked]:border-highlight data-[state=checked]:bg-highlight',
    className,
  )}
  {...restProps}
>
  <SwitchPrimitive.Thumb
    data-slot="switch-thumb"
    class="pointer-events-none block size-3.5 rounded-full bg-ink transition-[transform,background-color] duration-150 data-[state=checked]:translate-x-4 data-[state=checked]:bg-canvas motion-reduce:transition-none"
  />
</SwitchPrimitive.Root>
