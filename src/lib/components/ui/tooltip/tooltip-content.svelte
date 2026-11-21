<script lang="ts">
  import { Tooltip as TooltipPrimitive } from 'bits-ui';
  import { menuSurfaceClasses } from '$lib/components/ui/menu-styles';
  import { cn, type WithoutChild } from '$lib/utils';

  let {
    ref = $bindable(null),
    class: className,
    children,
    sideOffset = 6,
    ...restProps
  }: WithoutChild<TooltipPrimitive.ContentProps> = $props();
</script>

<TooltipPrimitive.Portal>
  <TooltipPrimitive.Content
    bind:ref
    {sideOffset}
    data-slot="tooltip-content"
    class={cn(
      menuSurfaceClasses,
      'max-w-64 px-2.5 py-1.5 text-xs text-ink outline-none data-[state=delayed-open]:animate-in data-[state=instant-open]:animate-in data-[state=closed]:animate-out data-[state=delayed-open]:fade-in-0 data-[state=closed]:fade-out-0 duration-100 motion-reduce:animate-none',
      className,
    )}
    {...restProps}
  >
    {@render children?.()}
  </TooltipPrimitive.Content>
</TooltipPrimitive.Portal>
