<script lang="ts">
  import type { Snippet } from 'svelte';

  type BadgeTone = 'neutral' | 'highlight';

  let { children, tone = 'neutral' }: { children: Snippet; tone?: BadgeTone } = $props();

  const toneClasses: Record<BadgeTone, string> = {
    neutral: 'border-border bg-surface-control text-ink',
    highlight: 'border-highlight bg-highlight text-canvas',
  };
</script>

<span
  class={`inline-flex h-5 shrink-0 items-center rounded-full border px-2 text-xs font-medium ${toneClasses[tone]}`}
>
  {@render children()}
</span>
