/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import type { ConfigSnapshot } from '../src/lib/generated/bindings';
import { deriveCaptureReadiness } from '../src/lib/capture-readiness';
import { DEFAULT_INFERENCE_BACKEND_METADATA } from '../src/lib/inference-profile-state';

const config: ConfigSnapshot = {
  version: 1,
  revision: 1,
  configPath: 'config.toml',
  captureBindings: [{ id: 'default', shortcut: 'CommandOrControl+Shift+2', profileId: null }],
  openShortcut: 'CommandOrControl+Alt+KeyZ',
  uiMode: 'basic',
  soundsEnabled: true,
  keepHistoryOriginals: true,
  automaticUpdateChecks: true,
  overlayPosition: 'bottom_center',
  registered: true,
  registrationError: null,
  lastError: null,
  processing: {
    activeProfileId: 'active',
    profiles: [
      {
        id: 'active',
        name: 'Default',
        modelSettings: { provider: 'openai_api', model: 'vision-model', options: null },
        rulesModelSettings: null,
        rulesModelMode: 'same',
        conversion: { tables: true, colors: true, images: true },
        additionalRules: '',
        additionalRulesSeparatePass: false,
        applicationModeOverride: null,
        conversionPromptTemplate: null,
        postprocessingPromptTemplate: null,
      },
    ],
    promptTemplateDefaults: { conversion: '', postprocessing: '' },
  },
  inference: {
    providers: [{ id: 'openai_api', backend: 'openai_api', endpoint: null, credentialRef: 'key' }],
  },
};

const readyInput = {
  desktopAvailable: true,
  configLoaded: true,
  permission: 'granted' as const,
  config,
  credentials: {
    metadata: DEFAULT_INFERENCE_BACKEND_METADATA,
    readiness: {
      openai_api: {
        provider: 'openai_api' as const,
        credentialRef: 'key',
        readiness: 'ready' as const,
      },
    },
  },
  shortcutsRegistered: true,
  shortcutError: null,
};

describe('capture readiness', () => {
  test('requires an assigned inference profile on the active profile', () => {
    const incomplete = {
      ...config,
      processing: {
        ...config.processing,
        profiles: [{ ...config.processing.profiles[0], modelSettings: null }],
      },
    };
    expect(deriveCaptureReadiness({ ...readyInput, config: incomplete })).toEqual({
      ready: false,
      label: 'Profile needs attention',
      detail: 'Choose a model for capture.',
    });
  });

  test('requires a model on the selected inference profile', () => {
    const incomplete = {
      ...config,
      processing: {
        ...config.processing,
        profiles: [
          {
            ...config.processing.profiles[0],
            modelSettings: { provider: 'openai_api' as const, model: '', options: null },
          },
        ],
      },
    };
    expect(deriveCaptureReadiness({ ...readyInput, config: incomplete })).toEqual({
      ready: false,
      label: 'Profile needs attention',
      detail: 'Choose a model for capture.',
    });
  });

  test('checks the selected model for a separate postprocessing pass', () => {
    const incomplete = {
      ...config,
      processing: {
        ...config.processing,
        profiles: [
          {
            ...config.processing.profiles[0],
            additionalRules: 'Translate to Spanish.',
            additionalRulesSeparatePass: true,
            rulesModelMode: 'independent' as const,
            rulesModelSettings: { provider: 'openai_api' as const, model: '', options: null },
          },
        ],
      },
    };
    expect(deriveCaptureReadiness({ ...readyInput, config: incomplete })).toEqual({
      ready: false,
      label: 'Profile needs attention',
      detail: 'Choose a model for rules.',
    });
  });

  test('requires provider access, matching credential readiness, and screen access', () => {
    const noProvider = {
      ...config,
      inference: {
        ...config.inference,
        providers: [{ ...config.inference.providers[0], credentialRef: null }],
      },
    };
    expect(deriveCaptureReadiness({ ...readyInput, config: noProvider }).ready).toBe(false);
    expect(
      deriveCaptureReadiness({
        ...readyInput,
        credentials: { ...readyInput.credentials, readiness: {} },
      }).ready,
    ).toBe(false);
    expect(deriveCaptureReadiness({ ...readyInput, permission: 'required' }).ready).toBe(false);
    expect(deriveCaptureReadiness({ ...readyInput, permission: 'restartRequired' })).toEqual({
      ready: false,
      label: 'Restart required',
      detail: 'Restart isoboard to finish enabling Screen Recording.',
    });
    expect(deriveCaptureReadiness(readyInput).ready).toBe(true);
  });

  test('does not present a shortcut registration failure as Ready', () => {
    expect(
      deriveCaptureReadiness({
        ...readyInput,
        shortcutsRegistered: false,
        shortcutError: 'Shortcut is in use.',
      }),
    ).toEqual({ ready: false, label: 'Shortcut unavailable', detail: 'Shortcut is in use.' });
  });
});

test('does not mistake pending or stale credential checks for a broken profile', () => {
  const readiness = {
    openai_api: {
      provider: 'openai_api' as const,
      credentialRef: 'old-key',
      readiness: 'ready' as const,
    },
  };
  for (const pending of [{}, readiness]) {
    expect(
      deriveCaptureReadiness({
        ...readyInput,
        credentials: { ...readyInput.credentials, readiness: pending },
      }),
    ).toMatchObject({ ready: false, checking: true, label: 'Checking…' });
  }
  expect(deriveCaptureReadiness({ ...readyInput, permission: 'required' })).toMatchObject({
    label: 'Screen access required',
  });
});
