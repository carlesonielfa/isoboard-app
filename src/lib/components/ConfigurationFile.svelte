<script lang="ts">
  let { path }: { path: string } = $props();
</script>

{#if path}
  <div class="flex min-w-0 flex-col gap-1 py-2 text-ink">
    <p class="m-0 text-sm">Configuration file</p>
    <p class="m-0 select-text text-xs [overflow-wrap:anywhere]">{path}</p>
  </div>
{/if}
