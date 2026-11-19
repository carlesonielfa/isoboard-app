<script lang="ts">
  import { CircleAlert } from '@lucide/svelte';
  import Button from '$lib/components/Button.svelte';
  let {
    message,
    action,
    onAction,
    checking = false,
  }: {
    message: string;
    action?: string;
    onAction?: () => void;
    checking?: boolean;
  } = $props();
</script>

<div
  class={`flex min-w-0 flex-wrap items-center gap-2 rounded-control border px-3 py-2 text-sm ${checking ? 'border-border bg-surface-solid text-ink' : 'border-danger-border bg-danger-surface text-danger'}`}
  role="status"
  aria-live="polite"
>
  <CircleAlert class="size-4 shrink-0" aria-hidden="true" />
  <span class="min-w-0 flex-1">{message}</span>
  {#if action && onAction && !checking}
    <Button size="sm" variant="ghost" class="shrink-0 text-inherit" onclick={onAction}
      >{action}</Button
    >
  {/if}
</div>
