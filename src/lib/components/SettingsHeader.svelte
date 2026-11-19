<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import {
    getTitlebarControlsCenterY,
    getWindowPresentationCapabilities,
  } from '$lib/window-presentation';
  import Wordmark from '$lib/components/Wordmark.svelte';

  let {
    navigation,
    trailing,
    sounds = false,
  }: { navigation?: Snippet; trailing?: Snippet; sounds?: boolean } = $props();
  let titlebarOverlay = $state(false);
  let brand: HTMLDivElement;
  let brandOffset = $state(0);
  onMount(() => {
    let mounted = true;
    let revision = 0;

    async function alignWordmark() {
      if (!titlebarOverlay) return;
      const current = ++revision;
      try {
        const centerY = await getTitlebarControlsCenterY();
        if (!mounted || current !== revision) return;
        const rect = brand.getBoundingClientRect();
        const unshiftedCenter = rect.top + rect.height / 2 - brandOffset;
        brandOffset = centerY === null ? 0 : centerY - unshiftedCenter;
      } catch {
        if (mounted && current === revision) brandOffset = 0;
      }
    }

    const observer = new ResizeObserver(() => void alignWordmark());
    void getWindowPresentationCapabilities()
      .then(async (capabilities) => {
        if (!mounted) return;
        titlebarOverlay = capabilities.shortcutPresentationPlatform === 'macos';
        if (!titlebarOverlay) return;
        observer.observe(brand);
        observer.observe(brand.closest('header')!);
        await alignWordmark();
      })
      .catch(() => {
        if (mounted) titlebarOverlay = false;
      });

    window.addEventListener('resize', alignWordmark);
    window.addEventListener('focus', alignWordmark);
    return () => {
      mounted = false;
      observer.disconnect();
      window.removeEventListener('resize', alignWordmark);
      window.removeEventListener('focus', alignWordmark);
    };
  });
</script>

<header
  data-tauri-drag-region={titlebarOverlay ? '' : undefined}
  data-titlebar-overlay={titlebarOverlay}
  class="settings-header app-chrome relative z-10 grid min-h-[3.25rem] shrink-0 grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-x-3 border-b border-border px-2 py-2 text-xs"
  aria-label="Settings navigation"
>
  <div
    bind:this={brand}
    data-tauri-drag-region={titlebarOverlay ? '' : undefined}
    class={`header-brand relative col-start-1 row-start-1 select-none ${titlebarOverlay ? 'pl-20' : ''}`}
    style:top={`${brandOffset}px`}
  >
    <Wordmark sound={sounds} />
  </div>
  {#if navigation}
    <div class="col-start-2 row-start-1 min-w-0 max-w-full justify-self-center">
      {@render navigation()}
    </div>
  {/if}
  {#if trailing}
    <div class="col-start-3 row-start-1 min-w-0 max-w-full justify-self-end">
      {@render trailing()}
    </div>
  {/if}
</header>
