<script lang="ts">
  import type { Snippet } from 'svelte';
  let {
    title,
    description,
    icon,
    children,
    actions,
  }: {
    title: string;
    description: string;
    icon: Snippet;
    children?: Snippet;
    actions: Snippet;
  } = $props();
</script>

<section
  class="flex min-w-0 flex-col rounded-surface border border-border bg-surface-control p-5"
  aria-label={title}
>
  <div
    class="mb-5 flex size-16 items-center justify-center rounded-surface border border-border bg-surface text-ink"
  >
    {@render icon()}
  </div>
  <h2 class="m-0 font-display text-xl font-semibold tracking-tight text-ink">{title}</h2>
  <p class="mt-2 mb-0 text-sm leading-relaxed text-ink">{description}</p>
  <div class="flex-1 pt-5">
    {#if children}{@render children()}{/if}
  </div>
  <div class="mt-5">{@render actions()}</div>
</section>
