<script lang="ts">
  import LicenseSetupStep from '$lib/components/LicenseSetupStep.svelte';
  import EvaluationExpiryNotice from '$lib/components/EvaluationExpiryNotice.svelte';
  import { createLicensingController } from '$lib/licensing.svelte';
  import { onMount, tick } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import CaptureProfileButton from '$lib/components/CaptureProfileButton.svelte';
  import ContentSurface from '$lib/components/ContentSurface.svelte';
  import AppShell from '$lib/components/AppShell.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import SettingsHeader from '$lib/components/SettingsHeader.svelte';
  import GeneralSettings from '$lib/components/GeneralSettings.svelte';
  import AppErrorToast from '$lib/components/AppErrorToast.svelte';
  import HistorySettings from '$lib/components/HistorySettings.svelte';
  import ProcessingProfiles from '$lib/components/ProcessingProfiles.svelte';
  import ProviderAccess from '$lib/components/ProviderAccess.svelte';
  import SetupGuide, { type SetupGuideStep } from '$lib/components/SetupGuide.svelte';
  import LocalModelSetupStep from '$lib/components/LocalModelSetupStep.svelte';
  import ScreenCapturePermissionStep from '$lib/components/ScreenCapturePermissionStep.svelte';
  import SettingsTabs, { type SettingsSection } from '$lib/components/SettingsTabs.svelte';
  import ShortcutSettings from '$lib/components/ShortcutSettings.svelte';
  import { createCaptureController } from '$lib/capture-controller.svelte';
  import { createAppErrorStore } from '$lib/app-error-store.svelte';
  import { createHistoryController } from '$lib/history-controller.svelte';
  import { createHistoryNavigation } from '$lib/history-navigation';
  import {
    createLocalModelsController,
    provideLocalModels,
  } from '$lib/local-models-controller.svelte';
  import { createConfigSession } from '$lib/config-session.svelte';
  import { createProcessingProfilesController } from '$lib/processing-profiles-controller.svelte';
  import { createInferenceProfilesController } from '$lib/inference-profiles-controller.svelte';
  import { createModelCatalogController } from '$lib/model-catalog-controller.svelte';
  import { createShortcutController } from '$lib/shortcut-controller.svelte';
  import { processingProfileDraftFromSnapshot } from '$lib/processing-profile-state';
  import { providerConnected, synthesizeInferenceProviders } from '$lib/inference-profile-state';
  import {
    automaticBuiltInSetupSelection,
    deriveInitialSetupState,
    hasConnectedExternalProvider,
    profileConfigurationNotice,
    SETUP_LOCAL_MODEL,
    localSetupProfileName,
    type SetupPath,
  } from '$lib/setup-state';
  import { deriveCaptureReadiness } from '$lib/capture-readiness';
  import { createScreenCapturePermissionController } from '$lib/screen-capture-permission.svelte';
  import { createLoginStartupController } from '$lib/login-startup.svelte';
  import { createUpdatesController } from '$lib/updates-controller.svelte';
  import { createNotificationPermissionController } from '$lib/notification-permission.svelte';
  import type { UiMode } from '$lib/generated/bindings';

  const availability = $state({ desktopAvailable: true });
  const errors = createAppErrorStore();
  const capture = createCaptureController(availability, errors);
  const history = createHistoryController(availability, errors);
  const config = createConfigSession(availability, errors);
  const localModels = createLocalModelsController();
  const licensing = createLicensingController();
  const notifications = createNotificationPermissionController();
  provideLocalModels(
    localModels,
    (model) =>
      config.current.processing.profiles
        .flatMap((p) => [p.modelSettings, p.rulesModelSettings])
        .filter((settings) => settings?.provider === 'local' && settings.model === model).length,
    () => capture.state.busy,
    () => config.current.uiMode,
    () => notifications.state.status === 'granted',
  );
  const advanced = $derived(config.current.uiMode !== 'basic');
  const debug = $derived(config.current.uiMode === 'debug');
  let modeSaving = $state(false);
  let soundsSaving = $state(false);
  let overlayEnabledSaving = $state(false);
  let updateChecksSaving = $state(false);
  let keepOriginalsSaving = $state(false);
  const profiles = createProcessingProfilesController({ availability, session: config, errors });
  const inference = createInferenceProfilesController({ availability, session: config, errors });
  const modelCatalog = createModelCatalogController({ availability });
  const shortcut = createShortcutController({ availability, session: config, errors });
  const screenPermission = createScreenCapturePermissionController();
  const loginStartup = createLoginStartupController(errors);
  const updates = createUpdatesController();
  const inferenceProviders = $derived(
    synthesizeInferenceProviders(inference.state.metadata, config.current.inference.providers),
  );
  let activeSection = $state<SettingsSection>('history');
  let setupStep = $state<SetupGuideStep | 'permissions' | 'local' | 'license'>('permissions');
  let setupPath = $state<SetupPath>('local');
  let setupConnectedProvider = $state<string | null>(null);
  let localSetupBusy = $state(false);
  let localPreparation = $state<'pending' | 'ready' | 'failed' | null>(null);
  let localSetupError = $state<string | null>(null);
  let setupActive = $state(false);
  let setupDismissed = $state(false);
  let setupInitialized = $state(false);
  let setupRestarted = $state(false);
  const refreshedCredentials = new SvelteSet<string>();
  const historyNavigation = createHistoryNavigation(
    async (entryId) => {
      setupDismissed = true;
      activeSection = 'history';
      await tick();
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      document.getElementById('settings-tab-history')?.focus({ preventScroll: true });
      await history.open(entryId);
    },
    (error) => errors.show(error, { source: 'history-navigation' }),
  );

  onMount(() => licensing.mount());
  onMount(() => capture.mount());
  onMount(() => history.mount());
  onMount(() => config.mount());
  onMount(() => localModels.mount());
  onMount(() => inference.mount());
  onMount(() => modelCatalog.mount());
  onMount(() => shortcut.mount());
  onMount(() => screenPermission.mount());
  onMount(() => loginStartup.mount());
  onMount(() => updates.mount());
  onMount(() => notifications.mount());
  onMount(() => historyNavigation.mount());
  onMount(() => {
    const onFocus = () => {
      if (!config.state.loaded) return;
      for (const provider of inferenceProviders) {
        if (provider.backend !== 'local' && providerConnected(provider))
          void inference.refreshReadiness(provider.backend);
      }
    };
    window.addEventListener('focus', onFocus);
    return () => window.removeEventListener('focus', onFocus);
  });

  const setupState = $derived(
    deriveInitialSetupState(config.current, {
      metadata: inference.state.metadata,
      readiness: inference.state.readiness,
    }),
  );
  const setupAutomaticSelection = $derived(
    automaticBuiltInSetupSelection(
      config.current,
      {
        metadata: inference.state.metadata,
        readiness: inference.state.readiness,
      },
      true,
      setupConnectedProvider,
    ),
  );
  const setupLocalModel = $derived(
    localModels.state.snapshot.models.find((model) => model.id === SETUP_LOCAL_MODEL),
  );
  const activeProcessing = $derived(
    config.current.processing.profiles.find(
      (profile) => profile.id === config.current.processing.activeProfileId,
    ),
  );
  const activeInference = $derived(activeProcessing?.modelSettings);
  const externalProviderReady = $derived(
    hasConnectedExternalProvider(
      config.current,
      {
        metadata: inference.state.metadata,
        readiness: inference.state.readiness,
      },
      setupConnectedProvider,
    ),
  );
  const showModelSetupStep = $derived(
    setupStep === 'profile' ||
      (setupPath === 'external' && externalProviderReady && !setupAutomaticSelection),
  );
  const setupBusy = $derived(
    localSetupBusy ||
      localPreparation === 'pending' ||
      config.state.loading ||
      inference.state.pending !== null ||
      profiles.state.pending !== null,
  );
  const setupVisible = $derived(setupActive && !setupDismissed);
  const captureReadiness = $derived(
    deriveCaptureReadiness({
      desktopAvailable: availability.desktopAvailable,
      configLoaded: config.state.loaded,
      providersLoaded: inference.state.metadataLoaded,
      permission: screenPermission.state.status,
      config: config.current,
      credentials: { metadata: inference.state.metadata, readiness: inference.state.readiness },
      shortcutsRegistered: config.current.registered,
      shortcutError: config.current.registrationError,
    }),
  );
  const captureShortcut = $derived(
    config.current.captureBindings.find((binding) => binding.id === 'default')?.shortcut ??
      config.current.captureBindings[0]?.shortcut ??
      'CommandOrControl+Shift+2',
  );
  const setupExpectedSection = $derived(
    setupStep === 'provider'
      ? 'provider-access'
      : setupStep === 'profile'
        ? 'processing-profiles'
        : 'shortcuts',
  );

  $effect(() => {
    if (
      !config.state.loaded ||
      !inference.state.metadataLoaded ||
      !licensing.state.loaded ||
      setupInitialized
    )
      return;
    setupInitialized = true;
    if (
      (licensing.state.snapshot?.needsChoice || setupState.needsInitialSetup) &&
      !setupDismissed
    ) {
      setupActive = true;
      setupDismissed = false;
      setupStep = licensing.state.snapshot?.needsChoice ? 'license' : 'permissions';
      activeSection = 'provider-access';
    }
  });

  $effect(() => {
    if (!config.state.loaded) return;
    for (const provider of inferenceProviders) {
      if (provider.backend === 'local' || !providerConnected(provider)) continue;
      const key = `${provider.backend}:${provider.credentialRef ?? provider.endpoint}`;
      if (refreshedCredentials.has(key)) continue;
      refreshedCredentials.add(key);
      void inference.refreshReadiness(provider.backend);
    }
  });

  $effect(() => {
    if (
      setupVisible &&
      setupStep === 'local' &&
      setupLocalModel?.downloaded &&
      activeInference?.provider === 'local' &&
      activeInference.model === SETUP_LOCAL_MODEL &&
      !setupBusy &&
      !localModels.state.pending[SETUP_LOCAL_MODEL] &&
      localPreparation === null
    ) {
      void prepareLocalSetup();
    }
  });

  async function prepareLocalSetup(): Promise<boolean> {
    localPreparation = 'pending';
    const prepared = await localModels.prepare(SETUP_LOCAL_MODEL);
    localPreparation = prepared ? 'ready' : 'failed';
    return prepared;
  }

  function resumeSetup(): void {
    setupDismissed = false;
    setupActive = true;
    setupConnectedProvider = null;
    setupRestarted = true;
    setupStep = 'license';
  }

  async function continueLicenseSetup(): Promise<void> {
    if (!licensing.state.snapshot?.commercial.active && !(await licensing.continueWithoutKey()))
      return;
    finishLicenseSetup();
  }

  async function activateSetupLicense(key: string): Promise<string | null> {
    if (!(await licensing.activate(key))) {
      const error = licensing.state.error || 'Could not activate the license. Try again.';
      licensing.state.error = '';
      return error;
    }
    finishLicenseSetup();
    return null;
  }

  function finishLicenseSetup(): void {
    if (setupRestarted || setupState.needsInitialSetup) setupStep = 'permissions';
    else {
      setupActive = false;
      setupDismissed = true;
    }
  }

  function openSetupStep(): void {
    activeSection = setupExpectedSection;
  }

  function continuePermissionSetup(): void {
    if (screenPermission.state.status !== 'granted') return;
    setupPath = activeInference && activeInference.provider !== 'local' ? 'external' : 'local';
    setupStep =
      setupPath === 'local' && activeInference?.model === SETUP_LOCAL_MODEL
        ? 'local'
        : setupState.profileReady
          ? 'capture'
          : setupPath === 'external'
            ? 'provider'
            : activeInference && activeInference.model !== SETUP_LOCAL_MODEL
              ? 'profile'
              : 'local';
    activeSection = setupExpectedSection;
  }

  function useExternalSetup(): void {
    setupPath = 'external';
    setupStep = 'provider';
    activeSection = 'provider-access';
  }

  async function useLocalSetup(): Promise<void> {
    if (setupBusy || !setupLocalModel || screenPermission.state.status !== 'granted') return;
    localSetupBusy = true;
    localSetupError = null;
    try {
      if (!(
        activeProcessing &&
        (await profiles.saveProfile(activeProcessing.id, {
          ...processingProfileDraftFromSnapshot(activeProcessing),
          name: localSetupProfileName(config.current),
          modelSettings: { provider: 'local', model: SETUP_LOCAL_MODEL, options: null },
        }))
      )) {
        localSetupError = inference.state.error ?? 'Could not select isOCR™. Try again.';
        return;
      }
      if (!setupLocalModel.downloaded) {
        await localModels.act(SETUP_LOCAL_MODEL, 'download');
        return;
      }
      if (localPreparation !== 'ready' && !(await prepareLocalSetup())) return;
      setupStep = deriveInitialSetupState(config.current, inference.state).profileReady
        ? 'capture'
        : 'profile';
      activeSection = setupStep === 'capture' ? 'shortcuts' : 'processing-profiles';
    } finally {
      localSetupBusy = false;
    }
  }

  async function continueSetup(): Promise<void> {
    if (!setupVisible) return;
    if (setupStep === 'provider') {
      if (!externalProviderReady) {
        activeSection = 'provider-access';
        return;
      }
      const selection = setupAutomaticSelection;
      if (selection) {
        if (!(await profiles.selectProfile(selection.profileId))) return;
      } else {
        // Keep repair on the chosen provider; never silently use another provider.
        const candidate = config.current.processing.profiles.find(
          (p) => p.modelSettings?.provider === setupConnectedProvider,
        );
        if (candidate) {
          if (!(await profiles.selectProfile(candidate.id))) return;
        } else if (setupConnectedProvider && activeInference) {
          const id = await profiles.createProfile();
          if (!id || !(await profiles.selectProfile(id))) return;
        }
        setupStep = 'profile';
        activeSection = 'processing-profiles';
        return;
      }
      if (
        !deriveInitialSetupState(config.current, {
          metadata: inference.state.metadata,
          readiness: inference.state.readiness,
        }).profileReady
      ) {
        setupStep = 'profile';
        activeSection = 'processing-profiles';
        return;
      }
      setupStep = 'capture';
      activeSection = 'shortcuts';
      return;
    }
    if (setupStep === 'profile') {
      if (!setupState.profileReady) {
        activeSection = 'processing-profiles';
        return;
      }
      setupStep = 'capture';
      activeSection = 'shortcuts';
    }
  }

  function backSetup(): void {
    if (setupStep === 'provider') {
      setupPath = 'local';
      setupStep = 'local';
    } else if (setupStep === 'capture' || setupStep === 'profile') {
      setupStep = setupPath === 'local' ? 'local' : 'provider';
      activeSection = 'provider-access';
    }
  }

  function dismissSetup(): void {
    if (!setupState.hasConnectedProvider || screenPermission.state.status !== 'granted') return;
    setupDismissed = true;
    setupActive = false;
  }

  function setKeepHistoryOriginals(enabled: boolean): void {
    if (!config.state.loaded || keepOriginalsSaving) return;
    keepOriginalsSaving = true;
    void config
      .setKeepHistoryOriginals(enabled)
      .catch(() => undefined)
      .finally(() => (keepOriginalsSaving = false));
  }

  function setAutomaticUpdateChecks(enabled: boolean): void {
    if (!config.state.loaded || updateChecksSaving) return;
    updateChecksSaving = true;
    void config
      .setAutomaticUpdateChecks(enabled)
      .catch(() => undefined)
      .finally(() => (updateChecksSaving = false));
  }

  function setOverlayEnabled(enabled: boolean): void {
    if (!config.state.loaded || overlayEnabledSaving) return;
    overlayEnabledSaving = true;
    void config
      .setOverlayEnabled(enabled)
      .catch(() => undefined)
      .finally(() => (overlayEnabledSaving = false));
  }

  function setSounds(enabled: boolean): void {
    if (!config.state.loaded || soundsSaving) return;
    soundsSaving = true;
    void config
      .setSoundsEnabled(enabled)
      .catch(() => undefined)
      .finally(() => (soundsSaving = false));
  }

  function setMode(mode: UiMode): void {
    if (!config.state.loaded || modeSaving || config.current.uiMode === mode) return;
    modeSaving = true;
    void config
      .setUiMode(mode)
      .catch(() => undefined)
      .finally(() => (modeSaving = false));
  }
</script>

<svelte:head>
  <title>isoboard</title>
  <meta name="description" content="Next generation clipboard OCR at your fingertips." />
</svelte:head>

<AppShell>
  <AppErrorToast error={errors.current} onDismiss={errors.dismiss} />
  {#if setupVisible && (setupStep === 'license' || setupStep === 'permissions' || setupStep === 'local')}
    <SettingsHeader sounds={config.current.soundsEnabled} />
  {/if}
  {#if setupVisible && setupStep === 'license'}
    <ContentSurface>
      <LicenseSetupStep
        snapshot={licensing.state.snapshot}
        busy={licensing.state.busy}
        error={licensing.state.error}
        onContinue={() => void continueLicenseSetup()}
        onActivate={activateSetupLicense}
      />
    </ContentSurface>
  {/if}
  {#if setupVisible && setupStep === 'permissions'}
    <ContentSurface>
      <ScreenCapturePermissionStep
        state={screenPermission.state}
        notifications={notifications.state}
        onEnableNotifications={notifications.request}
        onOpenNotificationSettings={notifications.openSettings}
        onRequest={screenPermission.request}
        onOpenSettings={screenPermission.openSettings}
        onRestart={screenPermission.restart}
        onContinue={continuePermissionSetup}
      />
    </ContentSurface>
  {/if}
  {#if setupVisible && setupStep === 'local'}
    <ContentSurface>
      <LocalModelSetupStep
        model={setupLocalModel}
        canNotify={notifications.state.status === 'granted'}
        loading={localModels.state.loading}
        preparing={localPreparation === 'pending'}
        busy={setupBusy || localModels.state.pending[SETUP_LOCAL_MODEL]}
        configured={activeInference?.provider === 'local' &&
          activeInference.model === SETUP_LOCAL_MODEL}
        error={localSetupError ??
          localModels.state.errors[SETUP_LOCAL_MODEL] ??
          setupLocalModel?.error ??
          localModels.state.error}
        onUse={useLocalSetup}
        onCancel={() => localModels.act(SETUP_LOCAL_MODEL, 'cancel')}
        onExternal={useExternalSetup}
        onBack={() => (setupStep = 'permissions')}
      />
    </ContentSurface>
  {/if}
  <SettingsTabs
    value={activeSection}
    sounds={config.current.soundsEnabled}
    onChange={(section) => (activeSection = section)}
    hidden={setupVisible &&
      (setupStep === 'license' || setupStep === 'permissions' || setupStep === 'local')}
  >
    {#snippet captureActions()}
      <CaptureProfileButton
        profiles={config.current.processing.profiles}
        activeProfileId={config.current.processing.activeProfileId}
        readiness={captureReadiness}
        highlighted={setupVisible && setupStep === 'capture' && !capture.state.busy}
        disabled={!availability.desktopAvailable ||
          !config.state.loaded ||
          config.state.loading ||
          capture.state.busy ||
          setupBusy}
        onCapture={() => {
          void capture.startCapture('region', { hideWindow: true });
        }}
        onSelectProfile={profiles.selectProfile}
      />
    {/snippet}
    {#snippet setupContent()}
      {#if licensing.state.snapshot?.showExpiryNotice && !capture.state.busy && !setupVisible}
        <EvaluationExpiryNotice
          busy={licensing.state.busy}
          error={licensing.state.error}
          onDismiss={() => void licensing.dismissNotice()}
          onLicense={() => {
            activeSection = 'settings';
          }}
        />
      {/if}
      {#if setupVisible && setupStep !== 'license' && setupStep !== 'permissions' && setupStep !== 'local'}
        <SetupGuide
          step={setupStep}
          showModelStep={showModelSetupStep}
          configurationNotice={profileConfigurationNotice(config.current, inference.state)}
          path={setupPath}
          progress={{
            permissions: screenPermission.state.status === 'granted',
            provider:
              setupPath === 'local' ? setupState.hasConnectedProvider : externalProviderReady,
            profile:
              setupStep === 'provider' ? Boolean(setupAutomaticSelection) : setupState.profileReady,
            capture: false,
          }}
          {captureShortcut}
          captureShortcutError={config.current.registrationError}
          currentSection={activeSection}
          expectedSection={setupExpectedSection}
          busy={setupBusy}
          onContinue={continueSetup}
          onBack={backSetup}
          onLater={dismissSetup}
          onOpenStep={openSetupStep}
        />
      {/if}
    {/snippet}
    {#snippet historyContent()}
      <HistorySettings
        state={history.state}
        {availability}
        {advanced}
        {debug}
        onSelect={history.select}
        onRestore={history.restore}
        onCopyPlainText={history.copyPlainText}
        onFormattingChange={history.changeFormatting}
        onCopyFailureDetails={history.copyFailureDetails}
        onCopyDebugHtml={history.copyDebugHtml}
        onDelete={history.deleteEntry}
        onLoadMore={history.loadMore}
        onFilterChange={history.setFilter}
      />
    {/snippet}
    {#snippet shortcutsContent()}
      <ShortcutSettings
        state={shortcut.state}
        config={config.current}
        configLoaded={config.state.loaded}
        {availability}
        onStartBindingRecording={shortcut.startBindingRecording}
        onStartCreateBindingRecording={shortcut.startCreateBindingRecording}
        onStartOpenShortcutRecording={shortcut.startOpenShortcutRecording}
        onRecordShortcut={shortcut.recordShortcut}
        onCancelRecording={shortcut.cancelRecording}
        onDeleteBinding={shortcut.deleteBinding}
        onSelectNewBindingProfile={shortcut.setNewBindingProfile}
        onSelectBindingProfile={shortcut.setBindingProfile}
      />
    {/snippet}
    {#snippet processingProfilesContent()}
      <ProcessingProfiles
        state={profiles.state}
        config={config.current}
        configLoaded={config.state.loaded}
        {availability}
        onCreate={profiles.createProfile}
        onDuplicate={profiles.duplicateProfile}
        onSave={profiles.saveProfile}
        onDelete={profiles.deleteProfile}
        onSelect={profiles.selectProfile}
        backendMetadata={inference.state.metadata}
        {inferenceProviders}
        credentials={inference.state.readiness}
        onOpenProviders={() => (activeSection = 'provider-access')}
        inferenceReady={config.state.loaded && inference.state.metadataLoaded}
        modelCatalog={modelCatalog.state}
        {advanced}
        promptTemplateDefaults={config.current.processing.promptTemplateDefaults}
      />
    {/snippet}
    {#snippet providerAccessContent()}
      <ProviderAccess
        state={inference.state}
        configLoaded={config.state.loaded}
        providers={inferenceProviders}
        {availability}
        onSaveProvider={async (provider, endpoint, credential) => {
          const saved = await inference.saveProvider(provider, endpoint, credential);
          if (saved && setupVisible && setupStep === 'provider') setupConnectedProvider = provider;
          return saved;
        }}
        onRefreshReadiness={inference.refreshReadiness}
      />
    {/snippet}
    {#snippet settingsContent()}
      <GeneralSettings
        {licensing}
        path={config.current.configPath}
        configLoaded={config.state.loaded}
        overlayEnabled={config.current.overlayEnabled}
        overlayEnabledDisabled={!availability.desktopAvailable ||
          !config.state.loaded ||
          overlayEnabledSaving}
        onOverlayEnabledChange={setOverlayEnabled}
        overlayPosition={config.current.overlayPosition}
        onOverlayPositionChange={(position) => config.setOverlayPosition(position).then(() => {})}
        onStartSetup={resumeSetup}
        soundsEnabled={config.current.soundsEnabled}
        soundsDisabled={!availability.desktopAvailable || !config.state.loaded || soundsSaving}
        onSoundsChange={setSounds}
        keepHistoryOriginals={config.current.keepHistoryOriginals}
        keepHistoryOriginalsDisabled={!availability.desktopAvailable ||
          !config.state.loaded ||
          keepOriginalsSaving}
        onKeepHistoryOriginalsChange={setKeepHistoryOriginals}
        {updates}
        automaticUpdateChecks={config.current.automaticUpdateChecks}
        automaticUpdateChecksDisabled={!availability.desktopAvailable ||
          !config.state.loaded ||
          updateChecksSaving}
        onAutomaticUpdateChecksChange={setAutomaticUpdateChecks}
        loginStartup={loginStartup.state}
        desktopAvailable={availability.desktopAvailable}
        onLoginStartupChange={(enabled) => void loginStartup.setEnabled(enabled)}
      />
    {/snippet}
  </SettingsTabs>
  {#snippet footer()}
    <AppFooter
      mode={config.current.uiMode}
      captureState={capture.state}
      readiness={setupVisible &&
      !setupState.profileReady &&
      (setupStep === 'local' || captureReadiness.ready)
        ? {
            ready: false,
            label: 'Setup incomplete',
            detail: 'Download the selected local model to finish setup.',
          }
        : setupVisible && !captureReadiness.ready && !captureReadiness.checking
          ? { ...captureReadiness, label: 'Setup incomplete' }
          : captureReadiness}
      disabled={!availability.desktopAvailable ||
        !config.state.loaded ||
        history.state.loading ||
        config.state.loading ||
        modeSaving}
      onModeChange={setMode}
    />
  {/snippet}
</AppShell>
