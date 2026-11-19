<script lang="ts">
  import SettingsSwitch from '$lib/components/SettingsSwitch.svelte';
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

<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
  <SettingsSwitch
    label="Open at login"
    checked={state.enabled}
    disabled={disabled || state.loading || state.saving}
    onCheckedChange={onChange}
  />
  <span class="text-xs text-ink">Starts in the menu bar.</span>
</div>
