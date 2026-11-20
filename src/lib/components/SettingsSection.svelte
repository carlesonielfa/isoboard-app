<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    id,
    title,
    children,
    variant,
  }: {
    id: string;
    title: string;
    children: Snippet;
    variant: 'panel' | 'nested';
  } = $props();
</script>

<section
  class={variant === 'panel'
    ? 'flex min-w-0 flex-col gap-3 rounded-control border border-border bg-surface-control p-3'
    : 'flex min-w-0 flex-col gap-3 border-l-2 border-border pl-3'}
  aria-labelledby={id}
>
  <h3 {id} class="m-0 text-sm font-semibold">{title}</h3>
  <div class="grid min-w-0 gap-3">
    {@render children()}
  </div>
</section>
