<script lang="ts">
  import { menuItemClasses } from '$lib/components/ui/menu-styles';
  import { cn } from '$lib/utils.js';
  import { DropdownMenu as DropdownMenuPrimitive } from 'bits-ui';

  let {
    ref = $bindable(null),
    class: className,
    inset,
    variant = 'default',
    ...restProps
  }: DropdownMenuPrimitive.ItemProps & {
    inset?: boolean;
    variant?: 'default' | 'destructive';
  } = $props();
</script>

<DropdownMenuPrimitive.Item
  bind:ref
  data-slot="dropdown-menu-item"
  data-inset={inset}
  data-variant={variant}
  class={cn(
    menuItemClasses,
    "relative flex items-center gap-2 px-3 py-2 select-none data-[disabled]:pointer-events-none data-[inset]:ps-8 data-[variant=destructive]:text-danger data-[variant=destructive]:data-highlighted:bg-danger-surface [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
    className,
  )}
  {...restProps}
/>
