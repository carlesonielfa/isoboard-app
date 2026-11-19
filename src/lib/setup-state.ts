import { modelDefaults } from '$lib/application-config';
import type {
  ConfigSnapshot,
  InferenceCredentialReadinessSnapshot,
  ProviderSnapshot,
} from '$lib/generated/bindings';
import {
  DEFAULT_INFERENCE_BACKEND_METADATA,
  providerConnected,
  synthesizeInferenceProviders,
} from '$lib/inference-profile-state';
import {
  processingProfileReferenceReadinessNotice,
  processingProfileUsesSeparatePass,
} from '$lib/processing-profile-state';
import type { InferenceProfilesViewState } from '$lib/page-types';

export type SetupReadinessState = Pick<InferenceProfilesViewState, 'metadata' | 'readiness'>;

export const SETUP_LOCAL_MODEL = modelDefaults.setupLocalModel;
export type SetupStep = 'permissions' | 'local' | 'provider' | 'profile' | 'capture';
export type SetupPath = 'local' | 'external';

export type InitialSetupState = {
  hasConnectedProvider: boolean;
  profileConfigured: boolean;
  profileReady: boolean;
  needsInitialSetup: boolean;
};

function providersFor(config: ConfigSnapshot, state: SetupReadinessState): ProviderSnapshot[] {
  return synthesizeInferenceProviders(
    state.metadata.length > 0 ? state.metadata : DEFAULT_INFERENCE_BACKEND_METADATA,
    config.inference.providers,
  );
}

function matchingReadiness(
  provider: ProviderSnapshot,
  state: SetupReadinessState,
): InferenceCredentialReadinessSnapshot | null {
  if (provider.backend === 'local')
    return { provider: 'local', credentialRef: null, readiness: 'ready' };
  if (!providerConnected(provider)) return null;
  const readiness = state.readiness[provider.id] ?? state.readiness[provider.backend];
  if (
    !readiness ||
    readiness.provider !== provider.backend ||
    readiness.credentialRef !== provider.credentialRef
  ) {
    return null;
  }
  return readiness;
}

function activeProcessingProfile(config: ConfigSnapshot) {
  return config.processing.profiles.find(
    (profile) => profile.id === config.processing.activeProfileId,
  );
}

export function localSetupProfileName(config: ConfigSnapshot): string {
  const active = activeProcessingProfile(config);
  if (!active) return 'isOCR';
  if (active.id !== 'default' || active.name !== 'Default') return active.name;
  const names = new Set(
    config.processing.profiles
      .filter((profile) => profile.id !== active.id)
      .map((profile) => profile.name.toLowerCase()),
  );
  let name = 'isOCR';
  let suffix = 2;
  while (names.has(name.toLowerCase())) name = `isOCR ${suffix++}`;
  return name;
}

export function automaticBuiltInSetupSelection(
  config: ConfigSnapshot,
  state: SetupReadinessState,
  replaceLocalSelection = false,
  preferredProvider: string | null = null,
): { profileId: string } | null {
  const active = activeProcessingProfile(config);
  if (
    !preferredProvider &&
    active?.modelSettings &&
    !(replaceLocalSelection && active.modelSettings.provider === 'local') &&
    deriveInitialSetupState(config, state).profileReady
  )
    return { profileId: active.id };
  const ready = providersFor(config, state).filter(
    (p) =>
      p.backend !== 'local' &&
      (!preferredProvider || p.backend === preferredProvider) &&
      matchingReadiness(p, state)?.readiness === 'ready',
  );
  for (const provider of ready) {
    const seed = state.metadata
      .find((m) => m.id === provider.backend)
      ?.builtInProfiles.find((p) => p.tier === 'efficient');
    const candidates = config.processing.profiles.filter(
      (p) => p.modelSettings?.provider === provider.backend,
    );
    const ordered = [
      ...candidates.filter((p) => seed && p.name === seed.name),
      ...candidates.filter((p) => seed && p.modelSettings?.model === seed.model),
      ...candidates,
    ];
    const profile = ordered.find(
      (p) =>
        deriveInitialSetupState(
          {
            ...config,
            processing: { ...config.processing, activeProfileId: p.id },
          },
          state,
        ).profileReady,
    );
    if (profile) return { profileId: profile.id };
  }
  return null;
}

function activeModels(config: ConfigSnapshot) {
  const profile = activeProcessingProfile(config);
  if (!profile) return [];
  const models = [profile.modelSettings];
  if (processingProfileUsesSeparatePass(profile))
    models.push(
      profile.rulesModelMode === 'same' ? profile.modelSettings : profile.rulesModelSettings,
    );
  return models;
}

export function hasConnectedProvider(config: ConfigSnapshot, state: SetupReadinessState): boolean {
  return providersFor(config, state).some(
    (provider) => matchingReadiness(provider, state)?.readiness === 'ready',
  );
}

export function hasConnectedExternalProvider(
  config: ConfigSnapshot,
  state: SetupReadinessState,
  preferredProvider: string | null = null,
): boolean {
  return providersFor(config, state).some(
    (provider) =>
      provider.backend !== 'local' &&
      (!preferredProvider || provider.backend === preferredProvider) &&
      matchingReadiness(provider, state)?.readiness === 'ready',
  );
}

export function pendingSetupDownloads(config: ConfigSnapshot): boolean {
  return activeModels(config).some(
    (settings) => settings?.provider === 'local' && settings.localModelDownloaded !== true,
  );
}

export function profileConfigured(config: ConfigSnapshot, state: SetupReadinessState): boolean {
  return profileConfigurationNotice(config, state) === null;
}

export function profileConfigurationNotice(
  config: ConfigSnapshot,
  state: SetupReadinessState,
): string | null {
  const profile = activeProcessingProfile(config);
  if (!profile) return 'Choose an active Profile before capture.';
  return processingProfileReferenceReadinessNotice(profile, providersFor(config, state));
}

export function profileReady(config: ConfigSnapshot, state: SetupReadinessState): boolean {
  if (!profileConfigured(config, state)) return false;

  const providers = providersFor(config, state);
  return activeModels(config).every((settings) => {
    if (!settings) return false;
    if (settings.provider === 'local')
      return (
        settings.localModelDownloaded === true || settings.localModelDownloadAvailable === true
      );
    const provider = providers.find((p) => p.backend === settings.provider);
    return provider ? matchingReadiness(provider, state)?.readiness === 'ready' : false;
  });
}

export function needsInitialSetup(config: ConfigSnapshot, state: SetupReadinessState): boolean {
  return !profileConfigured(config, state) || pendingSetupDownloads(config);
}

export function deriveInitialSetupState(
  config: ConfigSnapshot,
  state: SetupReadinessState,
): InitialSetupState {
  return {
    hasConnectedProvider: hasConnectedProvider(config, state),
    profileConfigured: profileConfigured(config, state),
    profileReady: profileReady(config, state) && !pendingSetupDownloads(config),
    needsInitialSetup: needsInitialSetup(config, state),
  };
}
