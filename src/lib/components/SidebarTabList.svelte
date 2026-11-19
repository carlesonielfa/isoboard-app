<script lang="ts">
  import type { ComponentProps, Snippet } from 'svelte';
  import * as Tabs from '$lib/components/ui/tabs/index.js';

  let {
    children,
    class: className,
    ...restProps
  }: ComponentProps<typeof Tabs.List> & { children: Snippet } = $props();
</script>

<Tabs.List variant="line" class={className} {...restProps}>
  {@render children()}
</Tabs.List>
