<script lang="ts">
  import { Tabs as TabsPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    ...restProps
  }: TabsPrimitive.TriggerProps = $props();
</script>

<TabsPrimitive.Trigger
  bind:ref
  data-slot="tabs-trigger"
  class={cn(
    "relative z-10 inline-flex h-auto w-16 shrink-0 items-center justify-center whitespace-nowrap rounded-[var(--tabs-inner-radius)] border border-transparent px-1 py-1 text-center text-xs font-semibold leading-tight text-ink transition-colors duration-150 ease-in-out hover:bg-ink/7 focus-visible:border-focus focus-visible:ring-2 focus-visible:ring-focus/30 focus-visible:outline-none data-[state=active]:text-canvas motion-reduce:transition-none [&_svg:not([class*='size-'])]:size-4 [&_svg]:pointer-events-none [&_svg]:shrink-0",
    'group-data-[variant=small]/tabs-list:w-16 group-data-[variant=small]/tabs-list:px-1.5 group-data-[variant=small]/tabs-list:py-0.5 group-data-[variant=small]/tabs-list:text-[0.6875rem] group-data-[variant=small]/tabs-list:leading-none',
    'group-data-[variant=line]/tabs-list:bg-transparent group-data-[variant=line]/tabs-list:data-[state=active]:bg-transparent',
    className,
  )}
  {...restProps}
/>
