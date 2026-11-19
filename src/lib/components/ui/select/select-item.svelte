<script lang="ts">
  import { Select as SelectPrimitive } from 'bits-ui';
  import CheckIcon from '@lucide/svelte/icons/check';
  import { cn, type WithoutChild } from '$lib/utils.js';
  import { menuItemClasses } from '$lib/components/ui/menu-styles';

  let {
    ref = $bindable(null),
    class: className,
    value,
    label,
    children: childrenProp,
    ...restProps
  }: WithoutChild<SelectPrimitive.ItemProps> = $props();
</script>

<SelectPrimitive.Item
  bind:ref
  {value}
  {label}
  data-slot="select-item"
  class={cn(
    menuItemClasses,
    "focus:bg-accent focus:text-accent-foreground not-data-[variant=destructive]:focus:**:text-accent-foreground gap-1.5 py-1 pr-8 pl-1.5 [&_svg:not([class*='size-'])]:size-4 *:[span]:last:flex *:[span]:last:items-center *:[span]:last:gap-2 relative flex w-full items-center select-none data-disabled:pointer-events-none [&_svg]:pointer-events-none [&_svg]:shrink-0",
    className,
  )}
  {...restProps}
>
  {#snippet children({ selected, highlighted })}
    <span class="pointer-events-none absolute right-2 flex size-4 items-center justify-center">
      {#if selected}
        <CheckIcon class="pointer-events-none" />
      {/if}
    </span>
    <span class="flex flex-1 gap-2 shrink-0 whitespace-nowrap">
      {#if childrenProp}
        {@render childrenProp({ selected, highlighted })}
      {:else}
        {label || value}
      {/if}
    </span>
  {/snippet}
</SelectPrimitive.Item>
