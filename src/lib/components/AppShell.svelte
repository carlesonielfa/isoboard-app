<script lang="ts">
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';
  import { getWindowPresentationCapabilities } from '$lib/window-presentation';

  let { children, footer }: { children: Snippet; footer?: Snippet } = $props();
  let nativeBackdrop = $state(false);

  onMount(() => {
    getWindowPresentationCapabilities()
      .then((capabilities) => {
        nativeBackdrop = capabilities.nativeBackdrop;
      })
      .catch(() => (nativeBackdrop = false));
  });
</script>

<main
  class={`app-shell bg-prismatic relative isolate flex h-dvh min-h-0 w-full min-w-0 flex-col overflow-hidden ${nativeBackdrop ? 'bg-surface-shell backdrop-blur-md' : 'bg-canvas'}`}
>
  <div class="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
    {@render children()}
  </div>
  {@render footer?.()}
</main>
