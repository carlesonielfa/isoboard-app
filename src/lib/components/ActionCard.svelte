<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    title,
    description,
    icon,
    status,
    actions,
    feedback = null,
    actionsVisible = true,
    error = null,
  }: {
    title: string;
    description: string;
    icon: Snippet;
    status?: Snippet;
    actions?: Snippet;
    feedback?: string | null;
    actionsVisible?: boolean;
    error?: string | null;
  } = $props();
</script>

<section
  class="grid gap-4 rounded-surface border border-border bg-surface-control p-5"
  aria-label={title}
>
  <div class="flex flex-wrap items-center justify-between gap-3">
    <h2 class="m-0 flex items-center gap-2 text-base font-semibold text-ink">
      {@render icon()}
      {title}
    </h2>
    {#if status}{@render status()}{/if}
  </div>
  <p class="m-0 text-sm leading-relaxed text-ink">{description}</p>
  {#if feedback}
    <div class="text-sm leading-relaxed text-ink" role="status">{feedback}</div>
  {/if}
  {#if actions && actionsVisible}
    <div class="flex flex-wrap gap-2">{@render actions()}</div>
  {/if}
  {#if error}<p class="m-0 text-xs text-danger" role="alert">{error}</p>{/if}
</section>
