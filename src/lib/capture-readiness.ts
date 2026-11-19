import type { SetupReadinessState } from '$lib/setup-state';
import { assessProfile } from '$lib/processing-profile-state';
import {
  synthesizeInferenceProviders,
  DEFAULT_INFERENCE_BACKEND_METADATA,
} from '$lib/inference-profile-state';
import type { ConfigSnapshot, ScreenCapturePermissionStatus } from '$lib/generated/bindings';

export type CaptureReadiness = {
  ready: boolean;
  detail: string;
  label?: string;
  checking?: boolean;
};

export function deriveCaptureReadiness({
  desktopAvailable,
  configLoaded,
  providersLoaded = true,
  permission,
  config,
  credentials,
  shortcutsRegistered,
  shortcutError,
}: {
  desktopAvailable: boolean;
  configLoaded: boolean;
  providersLoaded?: boolean;
  permission: ScreenCapturePermissionStatus | 'checking';
  config: ConfigSnapshot;
  credentials: SetupReadinessState;
  shortcutsRegistered: boolean;
  shortcutError: string | null;
}): CaptureReadiness {
  if (!desktopAvailable)
    return { ready: false, label: 'Desktop required', detail: 'Open the desktop app to capture.' };
  if (!configLoaded || !providersLoaded)
    return {
      ready: false,
      label: 'Checking…',
      checking: true,
      detail: 'Loading capture settings.',
    };
  if (permission === 'checking')
    return { ready: false, label: 'Checking…', checking: true, detail: 'Checking screen access.' };
  if (permission === 'restartRequired') {
    return {
      ready: false,
      label: 'Restart required',
      detail: 'Restart isoboard to finish enabling Screen Recording.',
    };
  }
  if (permission !== 'granted') {
    return {
      ready: false,
      label: 'Screen access required',
      detail: 'Allow Screen Recording access to capture a region.',
    };
  }
  const profile = config.processing.profiles.find(
    (p) => p.id === config.processing.activeProfileId,
  );
  const providers = synthesizeInferenceProviders(
    credentials.metadata.length ? credentials.metadata : DEFAULT_INFERENCE_BACKEND_METADATA,
    config.inference.providers,
  );
  const issue = profile
    ? assessProfile(profile, providers, undefined, credentials.readiness)
    : null;
  if (issue)
    return {
      ready: false,
      label: issue.checking ? 'Checking…' : 'Profile needs attention',
      detail: issue.message,
      ...(issue.checking ? { checking: true } : {}),
    };
  if (!profile) {
    return {
      ready: false,
      label: 'Profile needs attention',
      detail: 'Check the active profile settings.',
    };
  }
  if (!shortcutsRegistered) {
    return {
      ready: false,
      label: 'Shortcut unavailable',
      detail: shortcutError || 'Check capture shortcut registration.',
    };
  }
  return { ready: true, detail: 'Ready to capture an image.' };
}
