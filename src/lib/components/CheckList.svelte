<script lang="ts">
  import { Check } from '@lucide/svelte';
  let { items }: { items: string[] } = $props();
</script>

<ul class="m-0 grid list-none gap-2.5 p-0 text-sm leading-relaxed text-ink">
  {#each items as item (item)}
    <li class="flex items-start gap-2.5">
      <Check class="mt-0.5 size-4 shrink-0 text-focus" strokeWidth={2} aria-hidden="true" />
      <span>{item}</span>
    </li>
  {/each}
</ul>
