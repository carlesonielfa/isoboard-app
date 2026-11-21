<script lang="ts">
  import { History, Info, SlidersHorizontal } from '@lucide/svelte';
  import LicenseSettings from '$lib/components/LicenseSettings.svelte';
  import type { LicensingController } from '$lib/licensing.svelte';
  import SettingsGroup from '$lib/components/SettingsGroup.svelte';
  import CaptureOverlaySettings from '$lib/components/CaptureOverlaySettings.svelte';
  import AppUpdates from '$lib/components/AppUpdates.svelte';
  import type { UpdatesController } from '$lib/updates-controller.svelte';
  import ConfigurationFile from '$lib/components/ConfigurationFile.svelte';
  import SettingsSwitch from '$lib/components/SettingsSwitch.svelte';
  import LoginStartupSetting from '$lib/components/LoginStartupSetting.svelte';
  import type { LoginStartupState } from '$lib/login-startup.svelte';
  import type { OverlayPosition } from '$lib/generated/bindings';
  import Button from '$lib/components/Button.svelte';
  import ThirdPartyNotices from '$lib/components/ThirdPartyNotices.svelte';
  import { applicationConfig } from '$lib/application-config';
  import { docsLinks } from '$lib/docs-links';
  import DocumentationLink from '$lib/components/DocumentationLink.svelte';

  let {
    licensing,
    path,
    soundsEnabled,
    soundsDisabled,
    onSoundsChange,
    keepHistoryOriginals,
    keepHistoryOriginalsDisabled,
    onKeepHistoryOriginalsChange,
    updates,
    automaticUpdateChecks,
    automaticUpdateChecksDisabled,
    onAutomaticUpdateChecksChange,
    loginStartup,
    desktopAvailable,
    configLoaded,
    overlayEnabled,
    overlayEnabledDisabled,
    onOverlayEnabledChange,
    overlayPosition,
    onOverlayPositionChange,
    onLoginStartupChange,
    onStartSetup,
  }: {
    licensing: LicensingController;
    path: string;
    soundsEnabled: boolean;
    soundsDisabled: boolean;
    onSoundsChange: (enabled: boolean) => void;
    keepHistoryOriginals: boolean;
    keepHistoryOriginalsDisabled: boolean;
    onKeepHistoryOriginalsChange: (enabled: boolean) => void;
    updates: UpdatesController;
    automaticUpdateChecks: boolean;
    automaticUpdateChecksDisabled: boolean;
    onAutomaticUpdateChecksChange: (enabled: boolean) => void;
    loginStartup: LoginStartupState;
    desktopAvailable: boolean;
    configLoaded: boolean;
    overlayEnabled: boolean;
    overlayEnabledDisabled: boolean;
    onOverlayEnabledChange: (enabled: boolean) => void;
    overlayPosition: OverlayPosition;
    onOverlayPositionChange: (position: OverlayPosition) => Promise<void>;
    onLoginStartupChange: (enabled: boolean) => void;
    onStartSetup?: () => void;
  } = $props();
</script>

<div class="mx-auto flex min-h-full w-full max-w-[40rem] flex-col gap-6">
  <SettingsGroup title="General" icon={SlidersHorizontal}>
    <SettingsSwitch
      variant="row"
      label="Play sounds"
      checked={soundsEnabled}
      disabled={soundsDisabled}
      onCheckedChange={onSoundsChange}
    />
    <LoginStartupSetting
      state={loginStartup}
      disabled={!desktopAvailable}
      onChange={onLoginStartupChange}
    />
  </SettingsGroup>
  {#if configLoaded}
    <CaptureOverlaySettings
      enabled={overlayEnabled}
      enabledDisabled={overlayEnabledDisabled}
      onEnabledChange={onOverlayEnabledChange}
      position={overlayPosition}
      disabled={!desktopAvailable}
      onPositionChange={onOverlayPositionChange}
    />
  {/if}
  <SettingsGroup title="History" icon={History}>
    <SettingsSwitch
      variant="row"
      label="Keep original screenshots"
      collapsible
      description={`Keeps the full-resolution image of your ${applicationConfig.history.maxOriginals} most recent captures on this computer. Turning this off deletes the ones already kept.`}
      checked={keepHistoryOriginals}
      disabled={keepHistoryOriginalsDisabled}
      onCheckedChange={onKeepHistoryOriginalsChange}
      infoLink={docsLinks.keepOriginals}
    />
  </SettingsGroup>
  <LicenseSettings {licensing} />
  <SettingsGroup title="About" icon={Info}>
    <AppUpdates
      {updates}
      disabled={!desktopAvailable}
      automaticChecks={automaticUpdateChecks}
      automaticChecksDisabled={automaticUpdateChecksDisabled}
      onAutomaticChecksChange={onAutomaticUpdateChecksChange}
    />
    <ConfigurationFile {path} disabled={!desktopAvailable} />
    {#snippet footer()}
      {#if onStartSetup}
        <Button variant="link" size="sm" type="button" onclick={onStartSetup}>
          Restart initial setup
        </Button>
      {/if}
      <DocumentationLink />
      <ThirdPartyNotices disabled={!desktopAvailable} />
    {/snippet}
  </SettingsGroup>
</div>
