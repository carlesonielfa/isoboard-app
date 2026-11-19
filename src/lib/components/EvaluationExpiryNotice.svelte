<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  let {
    busy,
    error,
    onDismiss,
    onLicense,
  }: { busy: boolean; error: string; onDismiss: () => void; onLicense: () => void } = $props();
</script>

<aside
  class="mx-4 mt-3 grid gap-2 rounded-control border border-border bg-surface p-4 text-sm text-ink"
  aria-label="Commercial evaluation ended"
>
  <p class="m-0">
    Your commercial evaluation has ended. Personal use remains free. A commercial license is
    required for continued work use.
  </p>
  <div class="flex gap-2">
    <Button variant="outline" size="sm" onclick={onLicense}>View license</Button>
    <Button variant="ghost" size="sm" disabled={busy} onclick={onDismiss}>Dismiss</Button>
  </div>
  {#if error}<p class="m-0" role="alert">{error}</p>{/if}
</aside>
