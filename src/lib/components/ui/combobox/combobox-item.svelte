<script lang="ts">
  import { Combobox as ComboboxPrimitive } from 'bits-ui';
  import { cn, type WithoutChild } from '$lib/utils.js';
  import { menuItemClasses } from '$lib/components/ui/menu-styles';

  let {
    ref = $bindable(null),
    class: className,
    children: childrenProp,
    ...restProps
  }: WithoutChild<ComboboxPrimitive.ItemProps> = $props();
</script>

<ComboboxPrimitive.Item
  bind:ref
  data-slot="combobox-item"
  class={cn(
    menuItemClasses,
    'relative flex w-full select-none items-start px-2 py-1.5 text-left font-normal data-disabled:pointer-events-none',
    className,
  )}
  {...restProps}
>
  {#snippet children({ selected, highlighted })}
    {@render childrenProp?.({ selected, highlighted })}
  {/snippet}
</ComboboxPrimitive.Item>
