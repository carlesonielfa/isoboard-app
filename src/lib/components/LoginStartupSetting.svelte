<script lang="ts">
  import SettingsSwitch from '$lib/components/SettingsSwitch.svelte';
  import { docsLinks } from '$lib/docs-links';
  import type { LoginStartupState } from '$lib/login-startup.svelte';

  let {
    state,
    disabled = false,
    onChange,
  }: {
    state: LoginStartupState;
    disabled?: boolean;
    onChange: (enabled: boolean) => void;
  } = $props();
</script>

<SettingsSwitch
  variant="row"
  label="Launch at login"
  description="Starts in the menu bar."
  checked={state.enabled}
  disabled={disabled || state.loading || state.saving}
  onCheckedChange={onChange}
  infoLink={docsLinks.launchAtLogin}
/>
