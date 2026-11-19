<script lang="ts">
  import { applicationConfig } from '$lib/application-config';
  import { Check, ChevronRight, TriangleAlert } from '@lucide/svelte';
  import Button from '$lib/components/Button.svelte';
  import ContentSurface from '$lib/components/ContentSurface.svelte';
  import ProgressBar from '$lib/components/ProgressBar.svelte';
  import { onMount } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { shouldApplyCaptureStatus } from '$lib/capture-status';
  import { commands, events } from '$lib/ipc';
  import type { CaptureOverlayPresentation, CaptureStatus } from '$lib/generated/bindings';
  import { getWindowPresentationCapabilities } from '$lib/window-presentation';

  type OverlayView = {
    title: string;
    detail: string;
    phase: 'processing' | 'success' | 'failure';
    progress?: number;
  };

  let view = $state<OverlayView | null>(null);
  let leaving = $state(false);
  let nativeBackdrop = $state(false);
  let sequence = -1;
  let revision = 0;
  let status: CaptureStatus['status'] | null = null;
  let presentationGeneration = -1;
  let opening = $state(false);

  function applyPresentation(next: CaptureOverlayPresentation): void {
    if (next.sequence !== sequence || next.generation < presentationGeneration) return;
    presentationGeneration = next.generation;
    leaving = next.leaving;
  }

  function applyStatus(next: CaptureStatus): void {
    if (!shouldApplyCaptureStatus(next, { sequence, status, revision })) return;
    if (next.sequence !== sequence) presentationGeneration = -1;
    sequence = next.sequence;
    status = next.status;
    revision = 'revision' in next ? next.revision : 0;
    leaving = false;

    if (next.status === 'idle' || next.status === 'started' || next.status === 'cancelled') {
      view = null;
    } else if (next.status === 'imageReady') {
      view = {
        title: next.source === 'region' ? 'Region captured' : 'Image selected',
        detail: 'Processing image',
        phase: 'processing',
      };
    } else if (next.status === 'downloadingModel') {
      const progress = Math.min(
        100,
        Math.floor((100 * next.downloadedBytes) / Math.max(1, next.totalBytes)),
      );
      view = {
        title: 'Downloading model',
        detail: `${progress}% · ${(next.downloadedBytes / 1e9).toFixed(2)} / ${(next.totalBytes / 1e9).toFixed(2)} GB`,
        phase: 'processing',
        progress,
      };
    } else if (next.status === 'loadingModel') {
      view = { title: 'Loading model', detail: next.model, phase: 'processing' };
    } else if (next.status === 'processing') {
      view = {
        title: 'Processing capture',
        detail: 'Converting to editable content',
        phase: 'processing',
      };
    } else if (next.status === 'completed') {
      view = {
        title: 'Copied to clipboard',
        detail: 'Ready to paste in your desired app',
        phase: 'success',
      };
    } else {
      view = { title: 'Capture failed', detail: next.message, phase: 'failure' };
    }
  }

  async function openHistory(): Promise<void> {
    if (opening || sequence < 0) return;
    opening = true;
    try {
      await commands.openCaptureOverlayHistory(sequence);
    } catch (error) {
      console.error('Could not open capture History:', error);
    } finally {
      opening = false;
    }
  }

  onMount(() => {
    void getWindowPresentationCapabilities()
      .then((capabilities) => (nativeBackdrop = capabilities.nativeBackdrop))
      .catch(() => (nativeBackdrop = false));
    if (!isTauri()) return;
    const overlayWindow = getCurrentWindow();
    let disposed = false;
    const cleanups: Array<() => void> = [];

    async function subscribe(listener: Promise<() => void>): Promise<void> {
      try {
        const cleanup = await listener;
        if (disposed) cleanup();
        else cleanups.push(cleanup);
      } catch (error) {
        console.error('Could not subscribe to capture overlay updates:', error);
      }
    }

    async function syncVisibleStatus(): Promise<void> {
      try {
        if (!(await overlayWindow.isVisible())) return;
        const snapshot = await commands.getCaptureSnapshot();
        if (!disposed) applyStatus(snapshot);
      } catch (error) {
        console.error('Could not load capture overlay status:', error);
      }
    }

    function onVisibilityChange(): void {
      if (!document.hidden) void syncVisibleStatus();
    }

    document.addEventListener('visibilitychange', onVisibilityChange);
    void Promise.all([
      subscribe(
        events.captureOverlayPresentation.listen((event) => {
          if (!disposed) applyPresentation(event.payload);
        }),
      ),
      subscribe(
        events.captureStatus.listen((event) => {
          if (!disposed) applyStatus(event.payload);
        }),
      ),
    ]).then(() => {
      if (!disposed) void syncVisibleStatus();
    });

    return () => {
      disposed = true;
      cleanups.forEach((cleanup) => cleanup());
      document.removeEventListener('visibilitychange', onVisibilityChange);
    };
  });
</script>

{#if view}
  <section
    style:--capture-exit-fade={`${applicationConfig.feedback.captureFadeMilliseconds}ms`}
    aria-label="Capture status"
    aria-live={view.phase === 'failure' ? 'assertive' : 'polite'}
    class={`capture-overlay bg-prismatic relative isolate flex h-full overflow-hidden rounded-capture border border-border text-ink ${nativeBackdrop ? 'bg-surface-shell backdrop-blur-md' : 'bg-canvas'}`}
    class:leaving
    class:native-backdrop={nativeBackdrop}
  >
    <Button
      type="button"
      variant="ghost"
      size="icon"
      class="absolute inset-0 z-10 h-full w-full cursor-pointer rounded-capture p-0 focus-visible:ring-inset"
      disabled={opening}
      aria-label={`Open capture in History: ${view.title}`}
      title="Open capture in History"
      onclick={() => void openHistory()}
      ><span class="sr-only">Open capture in History</span></Button
    >
    <ContentSurface>
      <div class="flex h-full min-w-0 items-center gap-2 px-3 py-2">
        <span
          aria-hidden="true"
          class={`flex size-8 shrink-0 items-center justify-center rounded-control ${view.phase === 'failure' ? 'bg-danger/15 text-danger' : view.phase === 'success' ? 'bg-success/15 text-success' : 'bg-highlight/15 text-highlight'}`}
        >
          {#if view.phase === 'processing'}
            <span class="flex items-center gap-0.5">
              <span class="capture-dot size-1 rounded-full bg-current"></span>
              <span class="capture-dot size-1 rounded-full bg-current"></span>
              <span class="capture-dot size-1 rounded-full bg-current"></span>
            </span>
          {:else if view.phase === 'failure'}
            <TriangleAlert class="size-[1.125rem]" strokeWidth={2} />
          {:else}
            <Check class="size-[1.125rem]" strokeWidth={3} />
          {/if}
        </span>
        <div class="min-w-0 flex-1">
          <p class="m-0 font-display text-sm leading-4 font-semibold">{view.title}</p>
          <p class="m-0 mt-0.5 truncate text-xs leading-4 text-ink" title={view.detail}>
            {view.detail}
          </p>
          {#if view.progress !== undefined}<ProgressBar
              value={view.progress}
              label="Model download"
            />{/if}
        </div>
        <ChevronRight aria-hidden="true" class="relative size-3 shrink-0" strokeWidth={2} />
      </div>
    </ContentSurface>
  </section>
{/if}

<style>
  .capture-overlay {
    transition:
      opacity var(--capture-exit-fade) ease-out,
      transform var(--capture-exit-fade) ease-out;
  }

  .capture-overlay.leaving {
    opacity: 0;
    transform: translateY(4px);
  }

  .capture-overlay.native-backdrop.leaving {
    opacity: 1;
    transform: none;
  }

  .capture-dot {
    animation: capture-dot-hop 1.1s ease-in-out infinite;
  }

  .capture-dot:nth-child(2) {
    animation-delay: 110ms;
  }

  .capture-dot:nth-child(3) {
    animation-delay: 220ms;
  }

  @keyframes capture-dot-hop {
    0%,
    60%,
    100% {
      transform: translateY(0);
    }

    30% {
      transform: translateY(-3px);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .capture-overlay {
      transition: none;
    }

    .capture-dot {
      animation: none;
    }
  }
</style>
