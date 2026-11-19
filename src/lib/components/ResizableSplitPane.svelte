<script lang="ts">
  import type { Snippet } from 'svelte';
  import * as Resizable from '$lib/components/ui/resizable';

  let {
    primary,
    secondary,
    secondarySize = $bindable(null),
    defaultSecondaryHeight = 180,
    minSecondaryHeight = 60,
    minPrimaryHeight = 64,
    label = 'Resize panes',
  }: {
    primary: Snippet;
    secondary: Snippet;
    secondarySize?: number | null;
    defaultSecondaryHeight?: number;
    minSecondaryHeight?: number;
    minPrimaryHeight?: number;
    label?: string;
  } = $props();
  let height = $state(0);
  let dragging = $state(false);
  const available = $derived(Math.max(1, height - 1));
  const minimum = $derived(Math.min(100, (minSecondaryHeight / available) * 100));
  const primaryMinimum = $derived(Math.min(100 - minimum, (minPrimaryHeight / available) * 100));
  const initial = $derived(
    Math.max(
      minimum,
      Math.min(100 - primaryMinimum, secondarySize ?? (defaultSecondaryHeight / available) * 100),
    ),
  );
</script>

<div class="h-full min-h-0 overflow-hidden" bind:clientHeight={height}>
  {#if height > 0}
    <Resizable.PaneGroup
      direction="vertical"
      onLayoutChange={(sizes) => {
        secondarySize = sizes[1];
      }}
    >
      <Resizable.Pane minSize={primaryMinimum} defaultSize={100 - initial}>
        <div class="relative h-full min-h-0">
          {@render primary()}
          {#if dragging}<div class="absolute inset-0" aria-hidden="true"></div>{/if}
        </div>
      </Resizable.Pane>
      <Resizable.Handle
        withHandle
        aria-label={label}
        onDraggingChange={(value) => {
          dragging = value;
        }}
      />
      <Resizable.Pane minSize={minimum} defaultSize={initial}>
        {@render secondary()}
      </Resizable.Pane>
    </Resizable.PaneGroup>
  {/if}
</div>
