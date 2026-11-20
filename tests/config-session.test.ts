/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import type { ConfigError, ConfigSnapshot, UiMode } from '../src/lib/generated/bindings';
import { createConfigSession } from '../src/lib/config-session.svelte';
import { processingProfileSnapshotFromDraft } from '../src/lib/processing-profile-state';

const plainState = <T>(value: T): T => value;
type NativeEvent<T> = { event: `${string}`; id: number; payload: T };

function snapshot(revision: number, overrides: Partial<ConfigSnapshot> = {}): ConfigSnapshot {
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
      profiles: [processingProfileSnapshotFromDraft('default')],
      promptTemplateDefaults: { conversion: '', postprocessing: '' },
    },
    inference: { providers: [] },
    revision,
    ...overrides,
    registered: overrides.registered ?? false,
    registrationError: overrides.registrationError ?? null,
    lastError: overrides.lastError ?? null,
  };
}

function environment(
  getCaptureConfig: () => Promise<ConfigSnapshot>,
  listeners: Map<string, (event: { payload: unknown }) => void>,
  setUiMode: (mode: UiMode, expectedRevision: number) => Promise<ConfigSnapshot> = getCaptureConfig,
) {
  const eventSource = <T>(name: string) => ({
    listen: async (handler: (event: NativeEvent<T>) => void) => {
      listeners.set(name, handler as (event: { payload: unknown }) => void);
      return () => listeners.delete(name);
    },
  });
  return {
    isTauri: () => true,
    commands: {
      getCaptureConfig,
      setUiMode,
    },
    events: {
      configChanged: eventSource<ConfigSnapshot>('config-changed'),
      configError: eventSource<ConfigError>('config-error'),
    },
  };
}

async function settle(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
  await new Promise((resolve) => setTimeout(resolve, 0));
}

describe('config session', () => {
  test('installs one listener set, loads once, and disposes listeners', async () => {
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    let loads = 0;
    const session = createConfigSession(
      { desktopAvailable: true },
      undefined,
      plainState,
      environment(async () => {
        loads += 1;
        return snapshot(2);
      }, listeners),
    );
    const cleanup = session.mount();
    let stateNotifications = 0;
    const unsubscribeState = session.subscribe(() => (stateNotifications += 1));
    await settle();

    expect(loads).toBe(1);
    expect(listeners.size).toBe(2);
    expect(session.state.snapshot?.revision).toBe(2);
    const changed = listeners.get('config-changed');
    cleanup();
    expect(listeners.size).toBe(0);
    const notificationsAfterDispose = stateNotifications;
    changed?.({ payload: snapshot(3) });
    expect(stateNotifications).toBe(notificationsAfterDispose);
    unsubscribeState();
  });

  test('rejects stale revisions and suppresses duplicate snapshots', async () => {
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    const session = createConfigSession(
      { desktopAvailable: true },
      undefined,
      plainState,
      environment(async () => snapshot(3), listeners),
    );
    const cleanup = session.mount();
    await settle();
    let changes = 0;
    const unsubscribe = session.subscribe(() => (changes += 1));
    expect(changes).toBe(1);
    const changed = listeners.get('config-changed');
    changed?.({ payload: snapshot(0) });
    changed?.({ payload: snapshot(3) });
    changed?.({ payload: snapshot(2) });
    changed?.({ payload: snapshot(3) });
    expect(session.state.revision).toBe(3);
    expect(changes).toBe(1);
    unsubscribe();
    cleanup();
  });

  test('merges partial config errors and then resynchronizes the full snapshot', async () => {
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    const full = snapshot(4, {
      inference: {
        providers: [
          { id: 'openai_api', backend: 'openai_api', endpoint: null, credentialRef: 'ref' },
        ],
      },
      captureBindings: [{ id: 'default', shortcut: 'CommandOrControl+Shift+N', profileId: null }],
    });
    let resyncs = 0;
    const session = createConfigSession(
      { desktopAvailable: true },
      undefined,
      plainState,
      environment(async () => {
        resyncs += 1;
        return resyncs === 1 ? snapshot(3) : full;
      }, listeners),
    );
    session.mount();
    await settle();
    const observed: Array<{ revision: number; providers: number }> = [];
    const unsubscribe = session.subscribe((current) => {
      observed.push({
        revision: current.revision ?? 0,
        providers: current.inference.providers.length,
      });
    });
    const payload: ConfigError = {
      kind: 'conflict',
      message: 'Shortcut is already registered.',
      configPath: 'config.toml',
      shortcut: null,
      bindingId: null,
      currentBindings: [{ id: 'default', shortcut: 'CommandOrControl+Shift+N', profileId: null }],
      currentOpenShortcut: 'CommandOrControl+Alt+KeyX',
      registered: true,
      revision: 4,
    };
    listeners.get('config-error')?.({ payload });
    expect(session.state.snapshot?.inference.providers).toHaveLength(0);
    await settle();

    expect(resyncs).toBe(2);
    expect(session.state.snapshot).toEqual(full);
    expect(session.state.revision).toBe(4);
    expect(observed).toEqual([
      { revision: 3, providers: 0 },
      { revision: 4, providers: 0 },
      { revision: 4, providers: 1 },
    ]);
    unsubscribe();
  });

  test('serializes mutations and supplies the latest expected revision', async () => {
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    const session = createConfigSession(
      { desktopAvailable: true },
      undefined,
      plainState,
      environment(async () => snapshot(7), listeners),
    );
    session.mount();
    await settle();
    const expected: number[] = [];
    const first = session.mutate(async (revision) => {
      expected.push(revision);
      return snapshot(8);
    });
    const second = session.mutate(async (revision) => {
      expected.push(revision);
      return snapshot(9);
    });
    await Promise.all([first, second]);

    expect(expected).toEqual([7, 8]);
    expect(session.state.revision).toBe(9);
  });

  test('restores the persisted UI mode and saves mode changes through the config owner', async () => {
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    const session = createConfigSession(
      { desktopAvailable: true },
      undefined,
      plainState,
      environment(
        async () => snapshot(2, { uiMode: 'advanced' }),
        listeners,
        async (mode, expectedRevision) => {
          if (expectedRevision === 2) {
            expect(mode).toBe('debug');
            return snapshot(3, { uiMode: 'debug' });
          }
          expect(expectedRevision).toBe(3);
          expect(mode).toBe('basic');
          return snapshot(4, { uiMode: 'basic' });
        },
      ),
    );
    session.mount();
    await settle();

    expect(session.current.uiMode).toBe('advanced');
    await session.setUiMode('debug');
    expect(session.current.uiMode).toBe('debug');
    await session.setUiMode('basic');
    expect(session.current.uiMode).toBe('basic');
  });

  test('ends loading on an early config error while full resync is pending', async () => {
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    let resyncPending!: Promise<ConfigSnapshot>;
    const session = createConfigSession(
      { desktopAvailable: true },
      undefined,
      plainState,
      environment(async () => {
        resyncPending = new Promise(() => undefined);
        return resyncPending;
      }, listeners),
    );
    session.mount();
    await Promise.resolve();
    const payload: ConfigError = {
      kind: 'conflict',
      message: 'Shortcut is already registered.',
      configPath: 'config.toml',
      shortcut: null,
      bindingId: null,
      currentOpenShortcut: 'CommandOrControl+Alt+KeyZ',
      currentBindings: [{ id: 'default', shortcut: 'CommandOrControl+Shift+N', profileId: null }],
      registered: false,
      revision: 5,
    };

    listeners.get('config-error')?.({ payload });

    expect(session.state.loaded).toBe(true);
    expect(session.state.loading).toBe(false);
    expect(session.current.captureBindings[0]?.shortcut).toBe(payload.currentBindings[0]?.shortcut);
    expect(session.state.error).toBe(payload.message);
  });

  test('does not start a queued mutation after disposal while initialization is pending', async () => {
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    let resolveInitial!: (value: ConfigSnapshot) => void;
    let mutationCalls = 0;
    const session = createConfigSession(
      { desktopAvailable: true },
      undefined,
      plainState,
      environment(
        () =>
          new Promise<ConfigSnapshot>((resolve) => {
            resolveInitial = resolve;
          }),
        listeners,
      ),
    );
    const cleanup = session.mount();
    await Promise.resolve();
    await Promise.resolve();
    const mutation = session.mutate(async () => {
      mutationCalls += 1;
      return snapshot(2);
    });
    cleanup();
    resolveInitial(snapshot(1));

    await expect(mutation).rejects.toThrow('Config session is disposed.');
    expect(mutationCalls).toBe(0);
  });
});

test('local model progress does not reload config; download readiness changes do', async () => {
  const listeners = new Map<string, (event: { payload: unknown }) => void>();
  let calls = 0;
  let downloaded = false;
  const env = environment(async () => {
    calls++;
    return snapshot(1, {
      processing: {
        activeProfileId: 'default',
        promptTemplateDefaults: { conversion: '', postprocessing: '' },
        profiles: [
          {
            ...processingProfileSnapshotFromDraft('default'),
            modelSettings: {
              provider: 'local',
              model: 'qwen3.5-2b',
              options: null,
              localModelDownloaded: downloaded,
            },
          },
        ],
      },
    });
  }, listeners);
  const session = createConfigSession(undefined, undefined, plainState, {
    ...env,
    events: {
      ...env.events,
      localModelsChanged: {
        listen: async (handler) => {
          listeners.set('local-models-changed', handler as (event: { payload: unknown }) => void);
          return () => listeners.delete('local-models-changed');
        },
      },
    },
  });
  const cleanup = session.mount();
  await settle();
  const emit = (revision: number) =>
    listeners.get('local-models-changed')?.({
      payload: {
        revision,
        models: [{ id: 'qwen3.5-2b', downloaded }],
      },
    });
  emit(1);
  emit(2);
  emit(3);
  await settle();
  expect(calls).toBe(1);
  downloaded = true;
  emit(4);
  await settle();
  expect(calls).toBe(2);
  expect(session.current.processing.profiles[0].modelSettings?.localModelDownloaded).toBe(true);
  emit(5);
  await settle();
  expect(calls).toBe(2);
  downloaded = false;
  emit(6);
  await settle();
  expect(calls).toBe(3);
  expect(session.current.processing.profiles[0].modelSettings?.localModelDownloaded).toBe(false);
  cleanup();
});
