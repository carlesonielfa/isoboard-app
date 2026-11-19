<script lang="ts" module>
  import { tv, type VariantProps } from 'tailwind-variants';

  export const tabsListVariants = tv({
    base: 'group/tabs-list inline-flex min-w-0 items-center justify-center aria-disabled:cursor-not-allowed aria-disabled:opacity-45',
    variants: {
      variant: {
        default:
          'rounded-control border border-border bg-surface-control p-0.5 text-xs font-semibold text-ink [--tabs-inset:0.125rem] [--tabs-total-inset:0.25rem] [--tabs-inner-radius:calc(var(--radius-control)-0.1875rem)]',
        small:
          'rounded-[calc(var(--radius-control)-0.25rem)] border border-border bg-surface-control p-px text-xs font-semibold text-ink [--tabs-inset:0.0625rem] [--tabs-total-inset:0.125rem] [--tabs-inner-radius:calc(var(--radius-control)-0.375rem)]',
        line: 'gap-1 bg-transparent',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  });

  export type TabsListVariant = VariantProps<typeof tabsListVariants>['variant'];
</script>

<script lang="ts">
  import { Tabs as TabsPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    variant = 'default',
    class: className,
    ...restProps
  }: TabsPrimitive.ListProps & {
    variant?: TabsListVariant;
  } = $props();
</script>

<TabsPrimitive.List
  bind:ref
  data-slot="tabs-list"
  data-variant={variant}
  class={cn(tabsListVariants({ variant }), className)}
  {...restProps}
/>
