<script lang="ts">
  import LicenseSettings from '$lib/components/LicenseSettings.svelte';
  import type { LicensingController } from '$lib/licensing.svelte';
  import SettingsSection from '$lib/components/SettingsSection.svelte';
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
    overlayPosition: OverlayPosition;
    onOverlayPositionChange: (position: OverlayPosition) => Promise<void>;
    onLoginStartupChange: (enabled: boolean) => void;
    onStartSetup?: () => void;
  } = $props();
</script>

<div class="flex min-h-full flex-col gap-4">
  <SettingsSection id="general-heading" title="General">
    <div class="flex min-h-10 items-center">
      <SettingsSwitch
        label="Play sounds"
        checked={soundsEnabled}
        disabled={soundsDisabled}
        onCheckedChange={onSoundsChange}
      />
    </div>
    <div class="flex min-h-10 items-center">
      <LoginStartupSetting
        state={loginStartup}
        disabled={!desktopAvailable}
        onChange={onLoginStartupChange}
      />
    </div>
    {#if onStartSetup}
      <div class="flex min-h-10 items-center">
        <Button variant="outline" size="sm" type="button" onclick={onStartSetup}>
          Restart initial setup
        </Button>
      </div>
    {/if}
  </SettingsSection>
  <SettingsSection id="history-storage-heading" title="History">
    <div class="flex min-h-10 items-center">
      <SettingsSwitch
        label="Keep original screenshots"
        checked={keepHistoryOriginals}
        disabled={keepHistoryOriginalsDisabled}
        onCheckedChange={onKeepHistoryOriginalsChange}
      />
    </div>
    <p class="m-0 text-xs">
      Keeps the full-resolution image of your {applicationConfig.history.maxOriginals} most recent captures
      on this computer. Turning this off deletes the ones already kept.
    </p>
  </SettingsSection>
  {#if configLoaded}
    <CaptureOverlaySettings
      position={overlayPosition}
      disabled={!desktopAvailable}
      onPositionChange={onOverlayPositionChange}
    />
  {/if}
  <LicenseSettings {licensing} />
  <SettingsSection id="about-heading" title="About">
    <AppUpdates
      {updates}
      disabled={!desktopAvailable}
      automaticChecks={automaticUpdateChecks}
      automaticChecksDisabled={automaticUpdateChecksDisabled}
      onAutomaticChecksChange={onAutomaticUpdateChecksChange}
    />
    <ConfigurationFile {path} />
    <ThirdPartyNotices disabled={!desktopAvailable} />
  </SettingsSection>
</div>
