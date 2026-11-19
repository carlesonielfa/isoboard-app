<script lang="ts">
  import { ArrowRight, LoaderCircle, RotateCw, Settings2, ShieldCheck } from '@lucide/svelte';
  import CaptureIcon from '$lib/components/CaptureIcon.svelte';
  import ActionCard from '$lib/components/ActionCard.svelte';
  import Button from '$lib/components/Button.svelte';
  import StatusIndicator from '$lib/components/StatusIndicator.svelte';
  import SetupProgress from '$lib/components/SetupProgress.svelte';
  import NotificationPermissionControl from '$lib/components/NotificationPermissionControl.svelte';
  import type { NotificationPermissionState } from '$lib/notification-permission.svelte';
  import type { ScreenCapturePermissionViewState } from '$lib/screen-capture-permission.svelte';

  let {
    state,
    notifications,
    onEnableNotifications,
    onOpenNotificationSettings,
    onRequest,
    onOpenSettings,
    onRestart,
    onContinue,
  }: {
    state: ScreenCapturePermissionViewState;
    notifications: NotificationPermissionState;
    onEnableNotifications: () => void | Promise<void>;
    onOpenNotificationSettings: () => void | Promise<void>;
    onRequest: () => void | Promise<void>;
    onOpenSettings: () => void | Promise<void>;
    onRestart: () => void | Promise<void>;
    onContinue: () => void;
  } = $props();

  const granted = $derived(state.status === 'granted');
  const needsRestart = $derived(state.status === 'restartRequired');
</script>

<section class="min-h-0 flex-1 overflow-y-auto px-6 py-6" aria-label="Initial setup: permissions">
  <div class="mx-auto flex w-full max-w-[44rem] flex-col gap-6">
    <SetupProgress step="permissions" />
    <div class="flex items-center gap-4">
      <div
        class="flex size-12 shrink-0 items-center justify-center rounded-control border border-border bg-surface-control text-ink"
      >
        <ShieldCheck class="size-6 shrink-0" strokeWidth={1.75} aria-hidden="true" />
      </div>
      <h1 class="m-0 text-2xl font-semibold text-ink">Allow permissions</h1>
    </div>

    <ActionCard
      title="Screen Recording"
      description="Allow isoboard to capture the screen region you select."
      error={state.error}
      actionsVisible={!granted && state.status !== 'unsupported'}
      feedback={state.status === 'unsupported'
        ? 'Open the isoboard desktop app to grant access.'
        : needsRestart
          ? 'Access is enabled. Restart isoboard to finish.'
          : state.requested && !granted && !state.busy
            ? 'Enable Screen Recording for isoboard in System Settings. We’ll detect the change when you return.'
            : null}
    >
      {#snippet icon()}<CaptureIcon />{/snippet}
      {#snippet status()}
        <StatusIndicator
          status={state.busy || state.status === 'checking'
            ? 'busy'
            : granted
              ? 'success'
              : 'locked'}
          label={state.busy
            ? 'Waiting for macOS…'
            : state.status === 'checking'
              ? 'Checking…'
              : granted
                ? 'Allowed'
                : needsRestart
                  ? 'Restart needed'
                  : state.status === 'unsupported'
                    ? 'Desktop app required'
                    : state.requested
                      ? 'Waiting for access'
                      : 'Not allowed'}
        />
      {/snippet}
      {#snippet actions()}
        {#if !granted && state.status !== 'unsupported'}
          {#if needsRestart}
            <Button
              variant="primary"
              type="button"
              disabled={state.busy}
              onclick={() => void onRestart()}
            >
              <RotateCw class="size-4" aria-hidden="true" />Restart isoboard
            </Button>
          {:else if !state.requested || state.busy}
            <Button
              variant="primary"
              type="button"
              disabled={state.busy || state.status === 'checking'}
              onclick={() => void onRequest()}
            >
              {#if state.busy}<LoaderCircle
                  class="size-4 animate-spin motion-reduce:animate-none"
                  aria-hidden="true"
                />
              {:else}<ShieldCheck class="size-4" aria-hidden="true" />{/if}
              {state.busy ? 'Waiting for macOS…' : 'Ask macOS for access'}
            </Button>
          {/if}
          <Button
            variant={state.requested && !needsRestart && !state.busy ? 'primary' : 'outline'}
            type="button"
            disabled={state.busy}
            onclick={() => void onOpenSettings()}
          >
            <Settings2 class="size-4" aria-hidden="true" />Open System Settings
          </Button>
        {/if}
      {/snippet}
    </ActionCard>

    <NotificationPermissionControl
      state={notifications}
      onRequest={onEnableNotifications}
      onOpenSettings={onOpenNotificationSettings}
    />
    <div class="flex justify-end">
      <Button variant="primary" type="button" disabled={!granted} onclick={onContinue}>
        Continue<ArrowRight class="size-4" aria-hidden="true" />
      </Button>
    </div>
  </div>
</section>
