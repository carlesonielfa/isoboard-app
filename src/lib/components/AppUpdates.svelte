<script lang="ts">
  import { RefreshCw } from '@lucide/svelte';
  import AppChangelog from '$lib/components/AppChangelog.svelte';
  import AppVersion from '$lib/components/AppVersion.svelte';
  import BrandLogo from '$lib/components/BrandLogo.svelte';
  import Button from '$lib/components/Button.svelte';
  import SettingsRow from '$lib/components/SettingsRow.svelte';
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

{#snippet changelog()}
  <AppChangelog entries={updates.state.changelog} />
{/snippet}

<SettingsRow label="isoboard" below={updates.state.changelog.length > 0 ? changelog : undefined}>
  {#snippet leading()}<BrandLogo />{/snippet}
  {#snippet labelContent()}<span class="font-display font-semibold">isoboard</span>{/snippet}
  {#snippet details()}
    <span class="flex flex-wrap items-center gap-x-2 gap-y-0.5">
      <AppVersion />
      <span aria-hidden="true">·</span>
      {#if indicator}
        <StatusIndicator size="xs" status={indicator.status} label={indicator.label} />
      {:else}
        <span>Not checked yet</span>
      {/if}
    </span>
  {/snippet}
  {#snippet control()}
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
        <RefreshCw
          class={`size-4 ${busy ? 'animate-spin motion-reduce:animate-none' : ''}`}
          strokeWidth={1.75}
          aria-hidden="true"
        />
        Check for updates
      </Button>
    {/if}
  {/snippet}
</SettingsRow>
<SettingsSwitch
  variant="row"
  label="Check for updates automatically"
  checked={automaticChecks}
  disabled={automaticChecksDisabled}
  onCheckedChange={onAutomaticChecksChange}
/>
