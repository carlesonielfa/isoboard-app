<script lang="ts">
  import type { Snippet } from 'svelte';

  type StatusTone = 'progress' | 'notice' | 'error';

  let {
    children,
    tone = 'progress',
    busy = false,
    role = tone === 'error' ? 'alert' : 'status',
  }: {
    children: Snippet;
    tone?: StatusTone;
    busy?: boolean;
    role?: 'alert' | 'status';
  } = $props();

  const toneClasses: Record<StatusTone, string> = {
    progress: 'text-ink',
    notice: 'box-border rounded-control border border-border bg-surface-control px-3 py-3 text-ink',
    error:
      'box-border rounded-control border border-danger-border/45 bg-danger-surface/70 px-3 py-2 text-danger',
  };
</script>

<div
  class={`flex min-h-6 items-center gap-3 text-xs ${toneClasses[tone]}`}
  {role}
  aria-live={tone === 'error' ? 'assertive' : 'polite'}
>
  {#if busy}
    <span
      class="inline-block size-[0.9rem] shrink-0 animate-[app-spin_700ms_linear_infinite] rounded-full border-2 border-border-strong border-t-ink motion-reduce:animate-[app-spin_1.4s_linear_infinite]"
      aria-hidden="true"
    ></span>
  {/if}
  <span>{@render children()}</span>
</div>
