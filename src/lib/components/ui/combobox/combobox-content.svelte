<script lang="ts">
  import { Combobox as ComboboxPrimitive } from 'bits-ui';
  import { cn, type WithoutChild, type WithoutChildrenOrChild } from '$lib/utils.js';
  import ComboboxPortal from './combobox-portal.svelte';
  import type { ComponentProps } from 'svelte';
  import { menuSurfaceClasses } from '$lib/components/ui/menu-styles';

  let {
    ref = $bindable(null),
    class: className,
    sideOffset = 4,
    portalProps,
    children,
    preventScroll = true,
    ...restProps
  }: WithoutChild<ComboboxPrimitive.ContentProps> & {
    portalProps?: WithoutChildrenOrChild<ComponentProps<typeof ComboboxPortal>>;
  } = $props();
</script>

<ComboboxPortal {...portalProps}>
  <ComboboxPrimitive.Content
    bind:ref
    {sideOffset}
    {preventScroll}
    data-slot="combobox-content"
    class={cn(
      menuSurfaceClasses,
      'w-(--bits-combobox-anchor-width) max-w-[calc(100vw-1rem)] border border-border bg-popover/90 p-1 shadow-glow',
      className,
    )}
    {...restProps}
  >
    <ComboboxPrimitive.Viewport
      class="min-h-0 max-h-(--bits-combobox-content-available-height) w-full overflow-y-auto"
    >
      {@render children?.()}
    </ComboboxPrimitive.Viewport>
  </ComboboxPrimitive.Content>
</ComboboxPortal>
