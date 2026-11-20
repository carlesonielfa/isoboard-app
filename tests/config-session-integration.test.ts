/// <reference types="bun-types" />

import { expect, test } from 'bun:test';
import * as bridge from '../src/lib/inference-profile-bridge';
import { createAppErrorRegistry } from '../src/lib/app-error';
import { createConfigSession } from '../src/lib/config-session.svelte';
import { createInferenceProfilesController } from '../src/lib/inference-profiles-controller.svelte';
import { createProcessingProfilesController } from '../src/lib/processing-profiles-controller.svelte';
import { createShortcutController } from '../src/lib/shortcut-controller.svelte';
import { events } from '../src/lib/ipc';
import {
  defaultProcessingProfileDraft,
  processingProfileSnapshotFromDraft,
} from '../src/lib/processing-profile-state';
import type { ConfigSnapshot } from '../src/lib/generated/bindings';

const plainState = <T>(value: T): T => value;
type ConfigChangedEvent = Parameters<Parameters<typeof events.configChanged.listen>[0]>[0];
type ConfigErrorEvent = Parameters<Parameters<typeof events.configError.listen>[0]>[0];
type ConfigChangedListener = (event: ConfigChangedEvent) => void;
type ConfigErrorListener = (event: ConfigErrorEvent) => void;
type ConfigEventListeners = {
  configChanged?: ConfigChangedListener;
  configError?: ConfigErrorListener;
};

function snapshot(
  revision: number,
  processingProfiles = [
    processingProfileSnapshotFromDraft('default'),
    processingProfileSnapshotFromDraft('other', defaultProcessingProfileDraft('Other')),
  ],
): ConfigSnapshot {
  return {
    version: 1,
    captureBindings: [{ id: 'default', shortcut: 'CommandOrControl+Shift+2', profileId: null }],
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
      profiles: processingProfiles,
      promptTemplateDefaults: { conversion: '', postprocessing: '' },
    },
    inference: { providers: [] },
    revision,
    registered: false,
    registrationError: null,
    lastError: null,
  };
}

function environment(
  getCaptureConfig: () => Promise<ConfigSnapshot>,
  listeners: ConfigEventListeners = {},
) {
  return {
    isTauri: () => true,
    commands: {
      getCaptureConfig,
      setUiMode: async () => getCaptureConfig(),
    },
    events: {
      configChanged: {
        listen: async (handler: ConfigChangedListener) => {
          listeners.configChanged = handler;
          return () => {
            if (listeners.configChanged === handler) listeners.configChanged = undefined;
          };
        },
      },
      configError: {
        listen: async (handler: ConfigErrorListener) => {
          listeners.configError = handler;
          return () => {
            if (listeners.configError === handler) listeners.configError = undefined;
          };
        },
      },
    },
  };
}

async function settle(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  await new Promise((resolve) => setTimeout(resolve, 0));
}

test('shared-session inference mount loads metadata independently', async () => {
  const metadata = [
    {
      id: 'openai_api' as const,
      credentialKind: 'api_key' as const,
      credentialOptional: false,
      supportsCustomEndpoint: true,
      builtInProfiles: [],
    },
  ];
  const availability = { desktopAvailable: true };
  const session = createConfigSession(
    availability,
    undefined,
    plainState,
    environment(async () => snapshot(2)),
  );
  const controller = createInferenceProfilesController({
    availability,
    session,
    bridge: { ...bridge, getInferenceBackendMetadata: async () => metadata },
    stateFactory: plainState,
  });
  const cleanup = controller.mount();
  session.mount();
  await settle();
  expect(controller.state.metadata).toEqual(metadata);
  expect(controller.state.metadataLoaded).toBe(true);
  expect(session.state.loaded).toBe(true);
  cleanup();
});

test('browser session supplies canonical default processing config', () => {
  const availability = { desktopAvailable: true };
  const session = createConfigSession(availability, undefined, plainState, {
    isTauri: () => false,
  });
  session.mount();
  expect(session.state.loaded).toBe(true);
  expect(session.current.processing.activeProfileId).toBe('default');
  expect(session.current.processing.profiles).toHaveLength(1);
});

test('config initialization failure leaves canonical state usable', async () => {
  const availability = { desktopAvailable: true };
  const session = createConfigSession(
    availability,
    undefined,
    plainState,
    environment(async () => Promise.reject(new Error('config unavailable'))),
  );
  session.mount();
  await settle();
  expect(session.state.loaded).toBe(true);
  expect(session.state.loading).toBe(false);
  expect(session.state.error).toBe('config unavailable');
  expect(session.current.processing.profiles[0]?.id).toBe('default');
});

test('shared shortcut conflict keeps controller guidance and raw global error', async () => {
  const availability = { desktopAvailable: true };
  const errors = createAppErrorRegistry();
  const listeners: ConfigEventListeners = {};
  const session = createConfigSession(
    availability,
    errors,
    plainState,
    environment(async () => snapshot(4), listeners),
  );
  session.mount();
  await settle();
  const conflict = {
    kind: 'conflict',
    message: 'Shortcut is already registered.',
    configPath: 'config.toml',
    currentBindings: snapshot(4).captureBindings,
    currentOpenShortcut: snapshot(4).openShortcut,
    registered: false,
    revision: 5,
  };
  const controller = createShortcutController({
    availability,
    session,
    errors,
    stateFactory: plainState,
    commands: {
      setCaptureShortcutRecording: async () => false,
      createCaptureBinding: async () => snapshot(6),
      setOpenShortcut: async () => snapshot(6),
      setCaptureBinding: async () => snapshot(6),
      setCaptureBindingProfile: async () => snapshot(6),
      deleteCaptureBinding: async () => Promise.reject(conflict),
    },
  });
  const cleanup = controller.mount();
  await controller.deleteBinding('default');
  expect(controller.state.error).toBe('Shortcut is already registered. Choose another shortcut.');
  expect(errors.current?.message).toBe('Shortcut is already registered.');
  cleanup();
});

test('shortcut recording target is cancelled when shared config removes its profile', async () => {
  const availability = { desktopAvailable: true };
  const listeners: ConfigEventListeners = {};
  const session = createConfigSession(
    availability,
    undefined,
    plainState,
    environment(async () => snapshot(1), listeners),
  );
  session.mount();
  await settle();
  const controller = createShortcutController({
    availability,
    session,
    stateFactory: plainState,
    commands: {
      setCaptureShortcutRecording: async () => true,
      createCaptureBinding: async () => snapshot(3),
      setOpenShortcut: async () => snapshot(3),
      setCaptureBinding: async () => snapshot(3),
      setCaptureBindingProfile: async () => snapshot(3),
      deleteCaptureBinding: async () => snapshot(3),
    },
  });
  controller.mount();
  controller.startCreateBindingRecording('default');
  await settle();

  expect(controller.state.recording).toBe(true);
  listeners.configChanged?.({ event: 'config-changed', id: 1, payload: snapshot(2, []) });
  expect(controller.state.recording).toBe(false);
  expect(controller.state.recordingTarget).toBeNull();
  controller.dispose();
});

test('processing and inference adapters share latest-revision mutation serialization', async () => {
  const expected: Array<{ command: string; revision: number }> = [];
  const availability = { desktopAvailable: true };
  const listeners: ConfigEventListeners = {};
  const session = createConfigSession(
    availability,
    undefined,
    plainState,
    environment(async () => snapshot(10), listeners),
  );
  session.mount();
  await settle();
  const processing = createProcessingProfilesController({
    availability,
    session,
    commands: {
      createProcessingProfile: async (_draft, expectedRevision) => {
        expected.push({ command: 'create_processing_profile', revision: expectedRevision });
        return { snapshot: snapshot(11), id: 'processing-1' };
      },
      updateProcessingProfile: async (_id, _draft, expectedRevision) => {
        expected.push({ command: 'update_processing_profile', revision: expectedRevision });
        return snapshot(11);
      },
      renameProcessingProfile: async (_id, _name, expectedRevision) => {
        expected.push({ command: 'rename_processing_profile', revision: expectedRevision });
        return snapshot(11);
      },
      deleteProcessingProfile: async (_id, expectedRevision) => {
        expected.push({ command: 'delete_processing_profile', revision: expectedRevision });
        return snapshot(11);
      },
      selectProcessingProfile: async (_id, expectedRevision) => {
        expected.push({ command: 'select_processing_profile', revision: expectedRevision });
        return snapshot(11);
      },
    },
    stateFactory: plainState,
  });
  const inference = createInferenceProfilesController({
    availability,
    session,
    bridge: {
      ...bridge,
      saveInferenceProviderWithCredential: async (_draft, expectedRevision: number) => {
        expected.push({ command: 'save_provider_with_credential', revision: expectedRevision });
        return snapshot(12);
      },
    },
    stateFactory: plainState,
  });
  const processingSave = processing.createProfile('Processing');
  const inferenceSave = inference.saveProvider('openai_api', null, 'test-key');
  await Promise.all([processingSave, inferenceSave]);
  expect(expected).toEqual([
    { command: 'create_processing_profile', revision: 10 },
    { command: 'save_provider_with_credential', revision: 11 },
  ]);
  expect(session.state.revision).toBe(12);
});
