<script lang="ts">
  import type { Snippet } from 'svelte';

  type BadgeTone = 'neutral' | 'highlight';
  type BadgeSize = 'xs' | 'sm';

  let {
    children,
    tone = 'neutral',
    size = 'xs',
  }: {
    children: Snippet;
    tone?: BadgeTone;
    /** `sm` matches the size of surrounding `text-sm` copy. */
    size?: BadgeSize;
  } = $props();

  const toneClasses: Record<BadgeTone, string> = {
    neutral: 'border-border bg-surface-control text-ink',
    highlight: 'border-highlight bg-highlight text-canvas',
  };

  const sizeClasses: Record<BadgeSize, string> = {
    xs: 'h-5 px-2 text-xs',
    sm: 'h-6 px-2.5 text-sm',
  };
</script>

<span
  class={`inline-flex shrink-0 items-center rounded-full border font-medium ${sizeClasses[size]} ${toneClasses[tone]}`}
>
  {@render children()}
</span>
