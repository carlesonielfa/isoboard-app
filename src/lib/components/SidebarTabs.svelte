<script lang="ts">
  import type { ComponentProps, Snippet } from 'svelte';
  import * as Tabs from '$lib/components/ui/tabs/index.js';

  let {
    value = $bindable(''),
    onValueChange,
    children,
    ...restProps
  }: ComponentProps<typeof Tabs.Root> & { children: Snippet } = $props();
</script>

<Tabs.Root bind:value {onValueChange} orientation="vertical" {...restProps}>
  {@render children()}
</Tabs.Root>
