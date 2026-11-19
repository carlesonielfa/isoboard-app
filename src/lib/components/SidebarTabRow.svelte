<script lang="ts">
  import type { Snippet } from 'svelte';
  let { selected, children }: { selected: boolean; children: Snippet } = $props();
</script>

<div
  class={`flex min-w-0 items-center rounded-control transition-colors duration-150 ease-in-out motion-reduce:transition-none ${selected ? 'bg-surface-raised' : 'hover:bg-ink/5'}`}
>
  {@render children()}
</div>
