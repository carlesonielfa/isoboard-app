/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import * as bridge from '../src/lib/inference-profile-bridge';
import { createInferenceProfilesController } from '../src/lib/inference-profiles-controller.svelte';
import { createConfigSession } from '../src/lib/config-session.svelte';
import { events } from '../src/lib/ipc';
import type {
  ConfigSnapshot,
  InferenceCredentialReadinessSnapshot,
} from '../src/lib/generated/bindings';

const desktop = { desktopAvailable: true };
const plainState = <T>(value: T): T => value;

function environment(
  initial: ConfigSnapshot,
  listeners: Map<string, (event: { payload: unknown }) => void>,
) {
  type ConfigChangedEvent = Parameters<Parameters<typeof events.configChanged.listen>[0]>[0];
  type ConfigErrorEvent = Parameters<Parameters<typeof events.configError.listen>[0]>[0];
  return {
    isTauri: () => true,
    commands: {
      getCaptureConfig: async () => initial,
      setUiMode: async () => initial,
    },
    events: {
      configChanged: {
        listen: async (handler: (event: ConfigChangedEvent) => void) => {
          listeners.set('config-changed', handler as (event: { payload: unknown }) => void);
          return () => listeners.delete('config-changed');
        },
      },
      configError: {
        listen: async (handler: (event: ConfigErrorEvent) => void) => {
          listeners.set('config-error', handler as (event: { payload: unknown }) => void);
          return () => listeners.delete('config-error');
        },
      },
    },
  };
}

function provider(backend: 'openai_api', credentialRef: string | null) {
  return { id: backend, backend, endpoint: null, credentialRef };
}

function snapshot(
  providers: ConfigSnapshot['inference']['providers'] = [],
  revision = 1,
): ConfigSnapshot {
  return {
    version: 1,
    captureBindings: [],
    openShortcut: 'CommandOrControl+Alt+KeyZ',
    uiMode: 'basic',
    soundsEnabled: true,
    keepHistoryOriginals: true,
    automaticUpdateChecks: true,
    overlayEnabled: true,
    overlayPosition: 'bottom_center',
    configPath: 'config.toml',
    processing: {
      activeProfileId: 'default',
      profiles: [],
      promptTemplateDefaults: { conversion: '', postprocessing: '' },
    },
    inference: { providers },
    revision,
    registered: false,
    registrationError: null,
    lastError: null,
  };
}

async function configSession(initial = snapshot([], 0)): Promise<{
  session: ReturnType<typeof createConfigSession>;
  listeners: Map<string, (event: { payload: unknown }) => void>;
}> {
  const listeners = new Map<string, (event: { payload: unknown }) => void>();
  const session = createConfigSession(
    desktop,
    undefined,
    plainState,
    environment(initial, listeners),
  );
  session.mount();
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
  return { session, listeners };
}

describe('inference profile controller auth boundaries', () => {
  test('clears old readiness before refreshing a newly saved credential', async () => {
    let resolveReadiness: ((status: InferenceCredentialReadinessSnapshot) => void) | undefined;
    let readinessStarted = false;
    const pending = new Promise<InferenceCredentialReadinessSnapshot>((resolve) => {
      resolveReadiness = resolve;
    });
    const { session } = await configSession(snapshot([provider('openai_api', 'a'.repeat(32))]));
    const controller = createInferenceProfilesController({
      availability: desktop,
      session,
      bridge: {
        ...bridge,
        saveInferenceProviderWithCredential: async () =>
          snapshot([provider('openai_api', 'b'.repeat(32))]),
        getInferenceCredentialReadiness: () => {
          readinessStarted = true;
          return pending;
        },
      },
      stateFactory: plainState,
    });
    controller.state.readiness.openai_api = {
      provider: 'openai_api',
      credentialRef: 'a'.repeat(32),
      readiness: 'ready',
    };

    const saved = await controller.saveProvider('openai_api', null, 'secret');

    expect(saved).toBe(true);
    expect(readinessStarted).toBe(true);
    expect(controller.state.readiness.openai_api).toBeUndefined();
    resolveReadiness?.({
      provider: 'openai_api',
      credentialRef: 'b'.repeat(32),
      readiness: 'ready',
    });
  });

  test('does not surface a stale readiness failure after the provider credential changes', async () => {
    let rejectReadiness: ((error: Error) => void) | undefined;
    const pending = new Promise<InferenceCredentialReadinessSnapshot>((_, reject) => {
      rejectReadiness = reject;
    });
    const { session, listeners } = await configSession(
      snapshot([provider('openai_api', 'a'.repeat(32))]),
    );
    const controller = createInferenceProfilesController({
      availability: desktop,
      session,
      bridge: {
        ...bridge,
        getInferenceCredentialReadiness: () => pending,
      },
      stateFactory: plainState,
    });
    const refresh = controller.refreshReadiness('openai_api');
    listeners.get('config-changed')?.({
      payload: snapshot([provider('openai_api', 'b'.repeat(32))], 2),
    });
    rejectReadiness?.(new Error('old credential failed'));
    await refresh;

    expect(controller.state.error).toBeNull();
  });
});

describe('optional provider keys', () => {
  test('a blank key saves only an OpenAI-compatible endpoint change', async () => {
    const saves: Array<{ backend: string; endpoint: string | null; credential: string }> = [];
    const compatible = (endpoint: string | null, credentialRef: string | null) => ({
      id: 'openai_compatible',
      backend: 'openai_compatible' as const,
      endpoint,
      credentialRef,
    });
    const { session } = await configSession(
      snapshot([compatible('http://localhost:11434/v1', 'a'.repeat(32))]),
    );
    const controller = createInferenceProfilesController({
      availability: desktop,
      session,
      bridge: {
        ...bridge,
        saveInferenceProviderWithCredential: async (draft) => {
          saves.push(draft);
          return snapshot([compatible(draft.endpoint, null)], 2);
        },
        getInferenceCredentialReadiness: async () => ({
          provider: 'openai_compatible',
          credentialRef: null,
          readiness: 'ready',
        }),
      },
      stateFactory: plainState,
    });

    expect(await controller.saveProvider('openai_api', null, ' ')).toBe(false);
    expect(
      await controller.saveProvider('openai_compatible', 'http://localhost:11434/v1', ''),
    ).toBe(false);
    expect(
      await controller.saveProvider('openai_compatible', ' http://localhost:1234/v1 ', ''),
    ).toBe(true);
    expect(saves).toEqual([
      { backend: 'openai_compatible', endpoint: 'http://localhost:1234/v1', credential: '' },
    ]);
  });
});
