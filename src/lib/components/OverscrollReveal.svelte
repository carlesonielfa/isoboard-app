<!-- Fades its content in at the bottom of a scroll area while the user keeps scrolling past
     the end, so it sits in the gap the native rubber band opens. Place it in the positioned
     container that holds the scroll area. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  let { scroller, children }: { scroller: HTMLElement | null; children: Snippet } = $props();

  // Wheel distance past the end that reveals the content fully.
  const FULL_PULL = 160;
  const MAX_OPACITY = 0.16;

  let pull = $state(0);
  const strength = $derived(Math.min(pull / FULL_PULL, 1));

  $effect(() => {
    const element = scroller;
    if (!element) return;
    let frame = 0;
    let last = 0;
    let lastWheel = 0;

    const decay = (now: number) => {
      const seconds = (now - last) / 1000;
      last = now;
      if (now - lastWheel > 90) pull *= Math.exp(-seconds * 7);
      if (pull < 0.5) {
        pull = 0;
        frame = 0;
        return;
      }
      frame = requestAnimationFrame(decay);
    };

    const onWheel = (event: WheelEvent) => {
      const atBottom = element.scrollTop + element.clientHeight >= element.scrollHeight - 1;
      if (event.deltaY <= 0 || !atBottom) return;
      pull = Math.min(pull + event.deltaY, FULL_PULL * 1.5);
      lastWheel = performance.now();
      if (!frame) {
        last = lastWheel;
        frame = requestAnimationFrame(decay);
      }
    };

    element.addEventListener('wheel', onWheel, { passive: true });
    return () => {
      element.removeEventListener('wheel', onWheel);
      cancelAnimationFrame(frame);
      pull = 0;
    };
  });
</script>

{#if pull > 0}
  <div
    aria-hidden="true"
    class="pointer-events-none absolute inset-x-0 bottom-3 -z-10 flex justify-center"
    style:opacity={strength * MAX_OPACITY}
    style:transform={`translateY(${(1 - strength) * 8}px)`}
  >
    {@render children()}
  </div>
{/if}
