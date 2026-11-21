<script lang="ts">
  import type { Snippet } from 'svelte';
  let {
    title,
    description,
    featured = false,
    icon,
    children,
    actions,
  }: {
    title: string;
    description: string;
    /** Emphasizes the card's border, for the option an equal-height group recommends. */
    featured?: boolean;
    icon: Snippet;
    children?: Snippet;
    actions: Snippet;
  } = $props();
</script>

<section
  class={`flex min-w-0 flex-col rounded-surface border bg-surface-control p-5 ${featured ? 'border-border-strong' : 'border-border'}`}
  aria-label={title}
>
  <div class="mb-4 flex items-center gap-3">
    <div
      class="flex size-12 shrink-0 items-center justify-center rounded-surface border border-border bg-surface text-ink"
    >
      {@render icon()}
    </div>
    <h2 class="m-0 font-display text-xl font-semibold tracking-tight text-ink">{title}</h2>
  </div>
  <p class="m-0 text-sm leading-relaxed text-ink">{description}</p>
  <div class="flex-1 pt-4">
    {#if children}{@render children()}{/if}
  </div>
  <div class="mt-5">{@render actions()}</div>
</section>
