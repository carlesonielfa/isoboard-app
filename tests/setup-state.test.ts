import { describe, expect, test } from 'bun:test';
import type {
  ConfigSnapshot,
  ModelSettingsSnapshot,
  InferenceCredentialReadinessSnapshot,
} from '../src/lib/generated/bindings';
import { DEFAULT_INFERENCE_BACKEND_METADATA } from '../src/lib/inference-profile-state';
import { processingProfileSnapshotFromDraft } from '../src/lib/processing-profile-state';
import {
  automaticBuiltInSetupSelection,
  localSetupProfileName,
  deriveInitialSetupState,
  hasConnectedExternalProvider,
  needsInitialSetup,
  profileReady,
} from '../src/lib/setup-state';

const remote: ModelSettingsSnapshot = { provider: 'openai_api', model: 'vision', options: null };
const local: ModelSettingsSnapshot = {
  provider: 'local',
  model: 'isocr1-0.8b',
  options: null,
  localModelDownloadAvailable: true,
  localModelDownloaded: false,
  localModelSupportsText: false,
};
function snapshot(model: ModelSettingsSnapshot | null = remote): ConfigSnapshot {
  return {
    version: 1,
    revision: 1,
    captureBindings: [],
    openShortcut: 'CommandOrControl+Alt+KeyZ',
    uiMode: 'basic',
    soundsEnabled: true,
    keepHistoryOriginals: true,
    automaticUpdateChecks: true,
    overlayEnabled: true,
    overlayPosition: 'bottom_center',
    configPath: 'config.toml',
    registered: true,
    registrationError: null,
    lastError: null,
    processing: {
      activeProfileId: 'default',
      profiles: [{ ...processingProfileSnapshotFromDraft('default'), modelSettings: model }],
      promptTemplateDefaults: { conversion: '', postprocessing: '' },
    },
    inference: {
      providers: [
        { id: 'openai_api', backend: 'openai_api', endpoint: null, credentialRef: 'key' },
      ],
    },
  };
}
function state(
  status: InferenceCredentialReadinessSnapshot['readiness'] = 'ready',
  credentialRef = 'key',
) {
  return {
    metadata: DEFAULT_INFERENCE_BACKEND_METADATA,
    readiness: {
      openai_api: { provider: 'openai_api' as const, credentialRef, readiness: status },
    },
  };
}
describe('profile setup readiness', () => {
  test('requires matching provider readiness without reading credentials', () => {
    expect(profileReady(snapshot(), state())).toBe(true);
    for (const status of ['unknown', 'missing', 'locked', 'corrupt', 'unavailable'] as const)
      expect(profileReady(snapshot(), state(status))).toBe(false);
    expect(profileReady(snapshot(), state('ready', 'stale'))).toBe(false);
    expect(hasConnectedExternalProvider(snapshot(), state())).toBe(true);
    expect(needsInitialSetup(snapshot(null), state())).toBe(true);
  });
  test('local downloads are required for setup completion but may start on capture', () => {
    const config = snapshot({ ...local });
    expect(profileReady(config, state())).toBe(true);
    expect(deriveInitialSetupState(config, state()).needsInitialSetup).toBe(true);
    config.processing.profiles[0].modelSettings!.localModelDownloaded = true;
    expect(deriveInitialSetupState(config, state()).needsInitialSetup).toBe(false);
  });
  test('remembered inactive rules settings never block readiness', () => {
    const config = snapshot();
    const profile = config.processing.profiles[0];
    profile.rulesModelSettings = { ...local, localModelDownloadAvailable: false };
    profile.additionalRules = 'Translate';
    profile.additionalRulesSeparatePass = true;
    expect(profileReady(config, state())).toBe(true);
    profile.rulesModelMode = 'independent';
    expect(profileReady(config, state())).toBe(false);
    profile.rulesModelSettings = null;
    expect(profileReady(config, state())).toBe(false);
    profile.additionalRules = '  ';
    expect(profileReady(config, state())).toBe(true);
  });
  test('onboarding selects an existing editable provisioned profile and never recreates a deleted one', () => {
    const config = snapshot(null);
    const ready = state();
    ready.metadata = ready.metadata.map((m) =>
      m.id === 'openai_api'
        ? {
            ...m,
            builtInProfiles: [
              {
                id: 'seed',
                name: 'Efficient',
                model: 'vision',
                tier: 'efficient',
                explanation: '',
              },
            ],
          }
        : m,
    );
    expect(automaticBuiltInSetupSelection(config, ready)).toBeNull();
    config.processing.profiles.push({
      ...processingProfileSnapshotFromDraft('seeded'),
      name: 'Efficient',
      modelSettings: remote,
    });
    expect(automaticBuiltInSetupSelection(config, ready)).toEqual({ profileId: 'seeded' });
    config.processing.profiles[0].modelSettings = local;
    config.processing.profiles[0].modelSettings!.localModelDownloaded = true;
    expect(automaticBuiltInSetupSelection(config, ready)).toEqual({ profileId: 'default' });
    expect(automaticBuiltInSetupSelection(config, ready, true)).toEqual({ profileId: 'seeded' });
  });
});

test('setup prefers the newly connected provider and only selects usable profiles', () => {
  const config = snapshot();
  config.inference.providers.push({
    id: 'anthropic_api',
    backend: 'anthropic_api',
    endpoint: null,
    credentialRef: 'second',
  });
  const ready = {
    metadata: DEFAULT_INFERENCE_BACKEND_METADATA.map((m) =>
      m.id === 'anthropic_api'
        ? {
            ...m,
            builtInProfiles: [
              {
                id: 'efficient',
                name: 'Anthropic Efficient',
                model: 'fast',
                tier: 'efficient' as const,
                explanation: '',
              },
            ],
          }
        : m,
    ),
    readiness: {
      ...state().readiness,
      anthropic_api: {
        provider: 'anthropic_api' as const,
        credentialRef: 'second',
        readiness: 'ready' as const,
      },
    },
  };
  config.processing.profiles.push({
    ...processingProfileSnapshotFromDraft('accurate'),
    modelSettings: { provider: 'anthropic_api', model: 'accurate', options: null },
  });
  config.processing.profiles.push({
    ...processingProfileSnapshotFromDraft('efficient'),
    name: 'Anthropic Efficient',
    modelSettings: { provider: 'anthropic_api', model: 'fast', options: null },
  });
  const select = () => automaticBuiltInSetupSelection(config, ready, true, 'anthropic_api');
  expect(select()).toEqual({ profileId: 'efficient' });
  config.processing.profiles[2].modelSettings!.model = 'user-edited-model';
  expect(select()).toEqual({ profileId: 'efficient' });
  config.processing.profiles[2].additionalRules = 'Translate';
  config.processing.profiles[2].additionalRulesSeparatePass = true;
  config.processing.profiles[2].rulesModelMode = 'independent';
  expect(select()).toEqual({ profileId: 'accurate' });
  config.processing.profiles.splice(1);
  expect(select()).toBeNull();
  expect(automaticBuiltInSetupSelection(config, ready, true, 'openai_compatible')).toBeNull();
  expect(hasConnectedExternalProvider(config, ready, 'openai_compatible')).toBe(false);
});

test('local setup names the default profile without overwriting custom names or colliding', () => {
  const config = snapshot(null);
  expect(localSetupProfileName(config)).toBe('isOCR - default');
  config.processing.profiles.push({
    ...processingProfileSnapshotFromDraft('other'),
    name: 'ISOCR - Default',
  });
  expect(localSetupProfileName(config)).toBe('isOCR - default 2');
  config.processing.profiles[0].name = 'My workflow';
  expect(localSetupProfileName(config)).toBe('My workflow');
  config.processing.activeProfileId = 'other';
  expect(localSetupProfileName(config)).toBe('ISOCR - Default');
});
