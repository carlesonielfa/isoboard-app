<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    title,
    icon,
    children,
  }: {
    title: string;
    icon?: Snippet;
    children?: Snippet;
  } = $props();
</script>

<div class="flex h-full min-h-0 items-center justify-center p-6">
  <div class="grid max-w-sm justify-items-center gap-3 text-center text-ink">
    {#if icon}
      <div class="flex items-center justify-center text-highlight">
        {@render icon()}
      </div>
    {/if}
    <h2 class="m-0 font-display text-base font-semibold">{title}</h2>
    {@render children?.()}
  </div>
</div>
