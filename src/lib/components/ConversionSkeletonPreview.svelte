<script lang="ts">
  import type { ProcessingConversionSettings } from '$lib/generated/bindings';

  let { conversion }: { conversion: ProcessingConversionSettings } = $props();

  const description = $derived(
    `Illustrative output: table content ${conversion.tables ? 'in cells' : 'flattened into lines'}, ` +
      `color styling ${conversion.colors ? 'retained' : 'removed'}, ` +
      `image ${conversion.images ? 'included' : 'omitted'} with its caption retained.`,
  );
</script>

<div class="relative min-h-0 min-w-0">
  <svg
    class="absolute inset-0 size-full"
    viewBox="0 0 208 104"
    preserveAspectRatio="xMinYMid meet"
    role="img"
    aria-label={description}
  >
    <rect x="0.5" y="0.5" width="207" height="103" rx="8" class="fill-surface stroke-border" />
    <g aria-hidden="true">
      <rect
        x="10"
        y="12"
        width="52"
        height="6"
        rx="3"
        class={conversion.colors ? 'fill-success' : 'fill-ink/70'}
      />
      <rect x="10" y="25" width="128" height="6" rx="3" class="fill-ink/60" />
      <rect x="10" y="38" width="102" height="6" rx="3" class="fill-ink/60" />
      {#if conversion.images}
        <rect
          x="154"
          y="10"
          width="44"
          height="32"
          rx="4"
          class="fill-surface-raised stroke-border"
        />
        <circle cx="189" cy="18" r="3" class="fill-ink/55" />
        <path d="M155 38 L168 23 L184 41 H158 Q155 41 155 38" class="fill-ink/45" />
        <path d="M176 41 L188 28 L197 38 Q197 41 194 41 Z" class="fill-ink/30" />
      {/if}
      <rect x="154" y="48" width="35" height="4" rx="2" class="fill-ink/60" />
      {#if conversion.tables}
        <rect
          x="10"
          y="60"
          width="188"
          height="34"
          rx="4"
          class="fill-surface-control stroke-border"
        />
        <path
          d="M14 60 H194 Q198 60 198 64 V77 H10 V64 Q10 60 14 60"
          class={conversion.colors ? 'fill-success/20' : 'fill-surface-control'}
        />
        <path d="M104 60 V94 M10 77 H198" class="stroke-border" />
        <rect
          x="16"
          y="66"
          width="70"
          height="5"
          rx="2.5"
          class={conversion.colors ? 'fill-success/80' : 'fill-ink/65'}
        />
        <rect
          x="110"
          y="66"
          width="58"
          height="5"
          rx="2.5"
          class={conversion.colors ? 'fill-success/80' : 'fill-ink/65'}
        />
        <rect x="16" y="83" width="52" height="5" rx="2.5" class="fill-ink/65" />
        <rect x="110" y="83" width="70" height="5" rx="2.5" class="fill-ink/65" />
      {:else}
        <rect
          x="10"
          y="64"
          width="62"
          height="6"
          rx="3"
          class={conversion.colors ? 'fill-success/80' : 'fill-ink/65'}
        />
        <rect x="80" y="64" width="47" height="6" rx="3" class="fill-ink/65" />
        <rect
          x="10"
          y="80"
          width="47"
          height="6"
          rx="3"
          class={conversion.colors ? 'fill-success/80' : 'fill-ink/65'}
        />
        <rect x="65" y="80" width="62" height="6" rx="3" class="fill-ink/65" />
      {/if}
    </g>
  </svg>
</div>
