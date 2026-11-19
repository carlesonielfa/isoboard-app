<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();
</script>

<div
  class="relative isolate flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden"
  data-slot="content-surface"
>
  <div class="content-dots pointer-events-none absolute inset-0 -z-10" aria-hidden="true"></div>
  {@render children()}
</div>

<style>
  .content-dots {
    background-image: radial-gradient(var(--color-grid-dot) 1px, transparent 1px);
    background-size: 28px 28px;
    mask-image: radial-gradient(ellipse at center, var(--color-ink) 18%, transparent 85%);
    -webkit-mask-image: radial-gradient(ellipse at center, var(--color-ink) 18%, transparent 85%);
  }
</style>
