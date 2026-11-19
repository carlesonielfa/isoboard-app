<script lang="ts">
  import { Bell, LoaderCircle, Settings2, ShieldCheck } from '@lucide/svelte';
  import ActionCard from '$lib/components/ActionCard.svelte';
  import Button from '$lib/components/Button.svelte';
  import StatusIndicator from '$lib/components/StatusIndicator.svelte';
  import type { NotificationPermissionState } from '$lib/notification-permission.svelte';

  let {
    state,
    onRequest,
    onOpenSettings,
  }: {
    state: NotificationPermissionState;
    onRequest: () => void | Promise<void>;
    onOpenSettings: () => void | Promise<void>;
  } = $props();
  const granted = $derived(state.status === 'granted');
</script>

<ActionCard
  title="Notifications"
  description="You’ll only be notified about model downloads."
  error={state.error}
  actionsVisible={!granted && state.status !== 'unsupported'}
  feedback={state.status === 'denied'
    ? 'Enable notifications for isoboard in System Settings. We’ll detect the change when you return.'
    : state.requestPending && !state.busy
      ? 'Allow notifications in macOS. We’ll detect the change when you return.'
      : null}
>
  {#snippet icon()}<Bell class="size-4" aria-hidden="true" />{/snippet}
  {#snippet status()}
    <StatusIndicator
      status={state.busy || state.status === 'checking' ? 'busy' : granted ? 'success' : 'locked'}
      label={state.busy
        ? 'Waiting for macOS…'
        : state.status === 'checking'
          ? 'Checking…'
          : granted
            ? 'Allowed'
            : state.status === 'unsupported'
              ? 'Desktop app required'
              : state.requestPending
                ? 'Waiting for access'
                : 'Not allowed'}
    />
  {/snippet}
  {#snippet actions()}
    {#if !granted && state.status !== 'unsupported'}
      {#if state.status !== 'denied'}
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
          {state.busy ? 'Waiting for macOS…' : 'Enable notifications'}
        </Button>
      {/if}
      <Button
        variant={state.status === 'denied' ? 'primary' : 'outline'}
        type="button"
        disabled={state.busy}
        onclick={() => void onOpenSettings()}
      >
        <Settings2 class="size-4" aria-hidden="true" />Open System Settings
      </Button>
    {/if}
  {/snippet}
</ActionCard>
