<script lang="ts">
  import ProgressBar from '$lib/components/ProgressBar.svelte';
  import type { CapturePhase } from '$lib/page-types';
  import type { CaptureReadiness } from '$lib/capture-readiness';

  let {
    phase,
    detail,
    readiness,
    progress = null,
  }: {
    phase: CapturePhase;
    detail: string;
    readiness: CaptureReadiness;
    progress?: number | null;
  } = $props();

  const statusLabels: Record<CapturePhase, string> = {
    idle: 'Ready',
    selecting: 'Select',
    processing: 'Processing',
    loading: 'Loading model',
    downloading: 'Downloading model',
    failed: 'Error',
  };

  const statusDescriptions: Record<CapturePhase, string> = {
    idle: 'Ready to capture an image.',
    selecting: 'Choose a screen region or image file.',
    processing: 'Processing the captured image.',
    loading: 'Loading local models.',
    downloading: 'Downloading the selected model.',
    failed: 'The last capture failed.',
  };

  const label = $derived(
    phase === 'idle' && !readiness.ready
      ? (readiness.label ?? 'Profile needs attention')
      : statusLabels[phase],
  );
  const description = $derived(
    phase === 'idle' ? readiness.detail : detail.trim() || statusDescriptions[phase],
  );
  const busy = $derived(
    phase === 'selecting' ||
      phase === 'processing' ||
      phase === 'loading' ||
      phase === 'downloading',
  );
  const spinnerClass = $derived(
    phase === 'loading'
      ? 'size-2.5 shrink-0 animate-[app-spin_900ms_linear_infinite] rounded-full border-2 border-warning/35 border-t-warning motion-reduce:animate-[app-spin_1.4s_linear_infinite]'
      : 'size-2.5 shrink-0 animate-[app-spin_900ms_linear_infinite] rounded-full border-2 border-success/35 border-t-success motion-reduce:animate-[app-spin_1.4s_linear_infinite]',
  );
  const dotClass = $derived(
    phase === 'failed'
      ? 'size-2.5 shrink-0 rounded-full bg-gradient-to-br from-danger to-danger/55'
      : phase === 'idle' && !readiness.ready
        ? `size-2.5 shrink-0 rounded-full ${readiness.checking || readiness.label === 'Setup incomplete' ? 'bg-warning' : 'bg-danger'}`
        : 'size-2.5 shrink-0 rounded-full bg-gradient-to-br from-success to-success/55',
  );
</script>

<!-- The status has a focus target so keyboard users can reveal its detail. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="group relative flex min-w-0 items-center gap-2 rounded-control px-1 py-0.5 text-xs font-medium text-ink outline-none focus-visible:ring-2 focus-visible:ring-focus/70"
  role="status"
  tabindex="0"
  aria-label={`${label}: ${description}`}
  aria-describedby="capture-status-detail"
  title={description}
>
  {#if busy}
    <span class={spinnerClass} aria-hidden="true"></span>
  {:else}
    <span class={dotClass} aria-hidden="true"></span>
  {/if}
  <span class="truncate">{label}</span>
  {#if phase === 'downloading' && progress !== null}<span>{progress}%</span><span class="w-16"
      ><ProgressBar value={progress} label="Model download" /></span
    >{/if}

  <span
    id="capture-status-detail"
    role="tooltip"
    class="pointer-events-none absolute bottom-full left-0 z-30 mb-2 w-max max-w-[18rem] rounded-control border border-border bg-overlay px-3 py-2 text-xs font-normal text-ink opacity-0 shadow-glow transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100 motion-reduce:transition-none"
  >
    {description}
  </span>
</div>
