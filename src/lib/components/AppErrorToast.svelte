<script lang="ts">
  import type { AppError } from '$lib/app-error';
  import Button from '$lib/components/Button.svelte';

  let {
    error,
    onDismiss,
  }: {
    error: AppError | null;
    onDismiss: (id: number) => void;
  } = $props();
</script>

{#if error}
  <div
    class="pointer-events-none fixed inset-x-0 top-0 z-50 flex justify-center px-4 pt-[max(1rem,env(safe-area-inset-top))]"
  >
    <div
      class="pointer-events-auto flex w-full max-w-[42rem] items-start gap-3 rounded-control border border-danger-border/55 bg-danger-surface px-4 py-3 text-danger shadow-glow"
      role="alert"
      aria-live="assertive"
    >
      <p class="m-0 min-w-0 flex-1 text-sm leading-6">{error.message}</p>
      <Button
        variant="ghost"
        class="min-h-9 shrink-0 px-2 text-sm"
        type="button"
        aria-label="Dismiss error"
        onclick={() => onDismiss(error.id)}
      >
        Dismiss
      </Button>
    </div>
  </div>
{/if}
