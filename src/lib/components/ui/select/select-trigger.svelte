<script lang="ts">
  import { Select as SelectPrimitive } from 'bits-ui';
  import { selectionTriggerClasses } from '$lib/components/ui/menu-styles';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import { cn, type WithoutChild } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    children,
    size = 'default',
    ...restProps
  }: WithoutChild<SelectPrimitive.TriggerProps> & {
    size?: 'sm' | 'default';
  } = $props();
</script>

<SelectPrimitive.Trigger
  bind:ref
  data-slot="select-trigger"
  data-size={size}
  class={cn(selectionTriggerClasses, className)}
  {...restProps}
>
  {@render children?.()}
  <ChevronDownIcon class="text-ink size-4 pointer-events-none" />
</SelectPrimitive.Trigger>
