<script lang="ts">
  import { Popover } from 'bits-ui';
  import { menuSurfaceClasses } from '$lib/components/ui/menu-styles';
  import { cn, type WithoutChild } from '$lib/utils';

  let {
    ref = $bindable(null),
    class: className,
    children,
    sideOffset = 4,
    ...restProps
  }: WithoutChild<Popover.ContentProps> = $props();
</script>

<Popover.Portal>
  <Popover.Content
    bind:ref
    {sideOffset}
    data-slot="popover-content"
    class={cn(
      menuSurfaceClasses,
      'w-fit p-2 outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=open]:fade-in-0 data-[state=closed]:fade-out-0 data-[state=open]:zoom-in-95 data-[state=closed]:zoom-out-95 duration-100 motion-reduce:animate-none',
      className,
    )}
    {...restProps}
  >
    {@render children?.()}
  </Popover.Content>
</Popover.Portal>
