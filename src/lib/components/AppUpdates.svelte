<script lang="ts">
  import AppChangelog from '$lib/components/AppChangelog.svelte';
  import AppVersion from '$lib/components/AppVersion.svelte';
  import BrandLogo from '$lib/components/BrandLogo.svelte';
  import Button from '$lib/components/Button.svelte';
  import SettingsSwitch from '$lib/components/SettingsSwitch.svelte';
  import StatusIndicator from '$lib/components/StatusIndicator.svelte';
  import type { UpdatesController } from '$lib/updates-controller.svelte';

  let {
    updates,
    disabled = false,
    automaticChecks,
    automaticChecksDisabled = false,
    onAutomaticChecksChange,
  }: {
    updates: UpdatesController;
    disabled?: boolean;
    automaticChecks: boolean;
    automaticChecksDisabled?: boolean;
    onAutomaticChecksChange: (enabled: boolean) => void;
  } = $props();

  const status = $derived(updates.state.status);
  const indicator = $derived.by(
    (): { status: 'success' | 'busy' | 'error'; label: string } | null => {
      if (updates.state.error) return { status: 'error', label: updates.state.error };
      switch (status.status) {
        case 'checking':
          return { status: 'busy', label: 'Checking for updates…' };
        case 'downloading':
          return { status: 'busy', label: `Downloading ${status.version}…` };
        case 'installing':
          return { status: 'busy', label: `Installing ${status.version}…` };
        case 'ready':
          return { status: 'success', label: `Version ${status.version} is ready to install` };
        case 'upToDate':
          return { status: 'success', label: 'Up to date' };
        case 'failed':
          return { status: 'error', label: status.message };
        default:
          return null;
      }
    },
  );
  const busy = $derived(indicator?.status === 'busy');
</script>

<div class="grid min-w-0">
  <div class="flex min-w-0 flex-wrap items-center gap-x-5 gap-y-2 py-2">
    <div class="flex min-w-0 items-center gap-3">
      <BrandLogo />
      <div class="grid min-w-0 gap-0.5">
        <p class="m-0 text-sm font-semibold text-ink">isoboard <AppVersion /></p>
        <div class="min-h-5">
          {#if indicator}
            <StatusIndicator size="xs" status={indicator.status} label={indicator.label} />
          {:else}
            <span class="text-xs text-ink">Not checked yet</span>
          {/if}
        </div>
      </div>
    </div>
    {#if status.status === 'ready' || status.status === 'installing'}
      <Button
        variant="primary"
        size="sm"
        type="button"
        disabled={disabled || status.status === 'installing'}
        onclick={() => void updates.install()}
      >
        Restart to update
      </Button>
    {:else}
      <Button
        variant="outline"
        size="sm"
        type="button"
        disabled={disabled || busy}
        onclick={() => void updates.check()}
      >
        Check for updates
      </Button>
    {/if}
  </div>
  <div class="flex min-h-10 items-center">
    <SettingsSwitch
      label="Check for updates automatically"
      checked={automaticChecks}
      disabled={automaticChecksDisabled}
      onCheckedChange={onAutomaticChecksChange}
    />
  </div>
  <AppChangelog entries={updates.state.changelog} />
</div>
