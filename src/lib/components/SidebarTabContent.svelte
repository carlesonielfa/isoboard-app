<script lang="ts">
  import type { ComponentProps, Snippet } from 'svelte';
  import * as Tabs from '$lib/components/ui/tabs/index.js';

  let {
    children,
    class: className,
    ...restProps
  }: ComponentProps<typeof Tabs.Content> & { children: Snippet } = $props();
</script>

<Tabs.Content class={className} {...restProps}>
  {@render children()}
</Tabs.Content>
