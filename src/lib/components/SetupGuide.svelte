<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  import SetupProgress from '$lib/components/SetupProgress.svelte';
  import type { SetupPath, SetupStep } from '$lib/setup-state';
  import { presentShortcut } from '$lib/shortcut-presentation';

  export type SetupGuideStep = 'provider' | 'profile' | 'capture';

  export type SetupGuideProgress = {
    permissions: boolean;
    provider: boolean;
    profile: boolean;
    capture: boolean;
  };

  let {
    step,
    showModelStep = false,
    path = 'external',
    progress,
    captureShortcut,
    captureShortcutError = null,
    configurationNotice = null,
    currentSection,
    expectedSection,
    busy = false,
    onContinue,
    onBack,
    onLater,
    onOpenStep,
  }: {
    step: SetupGuideStep;
    showModelStep?: boolean;
    path?: SetupPath;
    progress: SetupGuideProgress;
    captureShortcut: string;
    captureShortcutError?: string | null;
    configurationNotice?: string | null;
    currentSection: string;
    expectedSection: string;
    busy?: boolean;
    onContinue: () => void | Promise<void>;
    onBack: () => void;
    onLater: () => void;
    onOpenStep: () => void;
  } = $props();

  const label = $derived(
    step === 'provider'
      ? 'Connect a provider'
      : step === 'profile'
        ? 'Complete profile setup'
        : 'Start capturing',
  );
  const complete: SetupStep[] = $derived([
    ...(progress.permissions ? ['permissions' as const] : []),
    ...(progress.provider ? ['provider' as const] : []),
    ...(progress.profile ? ['local' as const, 'profile' as const] : []),
  ]);
  const shortcut = $derived(presentShortcut(captureShortcut, 'macos'));
  const onStepSection = $derived(currentSection === expectedSection);
  const stepReady = $derived(step === 'provider' ? progress.provider : progress.profile);
</script>

<section
  class="relative z-10 border-b border-border bg-surface/70 px-4 py-2 shadow-md shadow-canvas/35"
  aria-label="Initial setup"
>
  <div class="mx-auto grid w-full max-w-[58rem] gap-2">
    <SetupProgress {step} {path} {showModelStep} {complete} />

    <div
      tabindex="-1"
      class="flex items-center justify-between gap-3 outline-none"
      aria-live="polite"
    >
      <div class="min-w-0">
        <h2 class="m-0 text-sm font-semibold text-ink">{label}</h2>
        {#if step === 'provider'}
          <p class="m-0 mt-0.5 text-xs text-ink">
            {progress.profile
              ? 'Continue to capture.'
              : progress.provider
                ? showModelStep
                  ? 'Continue to complete profile setup.'
                  : 'Continue with the selected provider’s profile.'
                : 'Choose a provider below, then add your API key.'}
          </p>
        {:else if step === 'profile'}
          <p class="m-0 mt-0.5 text-xs text-ink">
            {configurationNotice ?? 'No usable default is available. Configure a profile below.'}
          </p>
        {:else}
          <p class="m-0 mt-0.5 text-xs text-ink">
            Press your keyboard shortcut (<span class="font-mono text-ink"
              >{shortcut.display || captureShortcut}</span
            >) or the top-right Capture button to start a capture.
          </p>
          {#if captureShortcutError}
            <p class="m-0 mt-1 text-xs text-danger">{captureShortcutError}</p>
          {/if}
        {/if}
      </div>
      <div class="flex shrink-0 items-center gap-2">
        {#if progress.permissions && progress.provider}
          <Button variant="ghost" class="h-8 px-2 text-xs" type="button" onclick={onLater}
            >Set up later</Button
          >
        {/if}
        <Button
          variant="ghost"
          class="h-8 px-2 text-xs"
          type="button"
          disabled={busy}
          onclick={onBack}>Back</Button
        >
        {#if step !== 'capture'}
          <Button
            variant="primary"
            class="h-8 px-3 text-xs"
            type="button"
            disabled={busy || (onStepSection && !stepReady)}
            onclick={onStepSection ? () => void onContinue() : onOpenStep}
            >{onStepSection
              ? 'Continue'
              : `Open ${step === 'provider' ? 'Providers' : 'Profiles'}`}</Button
          >
        {:else}
          <Button
            variant="primary"
            class="h-8 px-3 text-xs"
            type="button"
            disabled={!progress.permissions || !progress.provider || !progress.profile}
            onclick={onLater}>Done</Button
          >
        {/if}
      </div>
    </div>
  </div>
</section>
