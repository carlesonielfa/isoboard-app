<script lang="ts">
  import DebugResourceUsage from '$lib/components/DebugResourceUsage.svelte';
  import CaptureStatusIndicator from '$lib/components/CaptureStatusIndicator.svelte';
  import SegmentedControl from '$lib/components/SegmentedControl.svelte';
  import InfoLink from '$lib/components/InfoLink.svelte';
  import { docsLinks } from '$lib/docs-links';
  import type { UiMode } from '$lib/generated/bindings';
  import type { CaptureViewState } from '$lib/page-types';
  import type { CaptureReadiness } from '$lib/capture-readiness';

  const modeOptions = [
    { value: 'basic', label: 'Basic' },
    { value: 'advanced', label: 'Advanced' },
    { value: 'debug', label: 'Debug' },
  ] as const;

  let {
    mode,
    captureState,
    readiness,
    disabled = false,
    onModeChange,
  }: {
    mode: UiMode;
    captureState: CaptureViewState;
    readiness: CaptureReadiness;
    disabled?: boolean;
    onModeChange: (mode: UiMode) => void;
  } = $props();

  const detail = $derived(
    captureState.phase === 'failed'
      ? captureState.errorMessage || captureState.statusMessage
      : captureState.statusMessage,
  );
</script>

<footer
  class="app-chrome relative z-20 shrink-0 border-t border-border text-xs pb-[1px]"
  aria-label="Application controls"
>
  <div class="mx-auto flex min-h-8 w-full max-w-[58rem] items-center justify-between gap-1.5 px-4">
    <div class="flex min-w-0 items-center gap-3">
      <CaptureStatusIndicator
        phase={captureState.phase}
        progress={captureState.downloadProgress}
        {detail}
        {readiness}
      />
      {#if mode === 'debug'}<DebugResourceUsage />{/if}
    </div>

    <div class="flex shrink-0 items-center gap-1">
      <InfoLink {...docsLinks.displayMode} />
      <SegmentedControl
        options={modeOptions}
        value={mode}
        onChange={(value) => onModeChange(value as UiMode)}
        aria-label="Display mode"
        {disabled}
        class="text-xs"
      />
    </div>
  </div>
</footer>
