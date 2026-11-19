<script lang="ts">
  import { Lightbulb, Zap } from '@lucide/svelte';
  import type { BuiltinModelTier } from '$lib/generated/bindings';

  let {
    tier,
    explanation,
  }: {
    tier: BuiltinModelTier;
    explanation: string;
  } = $props();

  const label = $derived(tier === 'efficient' ? 'Efficient model' : 'Accurate model');
</script>

<span
  class="inline-flex size-5 shrink-0 items-center justify-center text-foreground"
  role="img"
  aria-label={`${label}: ${explanation}`}
  title={`${label}: ${explanation}`}
>
  {#if tier === 'efficient'}
    <Zap class="size-4" aria-hidden="true" />
  {:else}
    <Lightbulb class="size-4" aria-hidden="true" />
  {/if}
</span>
