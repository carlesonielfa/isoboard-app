/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import { createHistoryController } from '../src/lib/history-controller.svelte';
import type {
  HistoryFilter,
  HistorySettingsSnapshot,
  HistoryWindowSnapshot,
} from '../src/lib/generated/bindings';

const plainState = <T>(value: T): T => value;
type NativeEvent<T> = { event: `${string}`; id: number; payload: T };

const settingsSnapshot: HistorySettingsSnapshot = {
  firstModel: { provider: 'openai_api', model: 'gpt-test' },
  secondModel: null,
  tables: true,
  colors: false,
  images: true,
  applicationMode: 'prompt_and_html_cleanup',
  additionalRulesApplied: false,
  separatePassUsed: false,
};

function eventSource<T>(
  name: string,
  listeners: Map<string, (event: { payload: unknown }) => void>,
  cleanupCalls?: Set<string>,
) {
  return {
    listen: async (handler: (event: NativeEvent<T>) => void) => {
      listeners.set(name, handler as (event: { payload: unknown }) => void);
      return () => {
        cleanupCalls?.add(name);
        listeners.delete(name);
      };
    },
  };
}

describe('history controller', () => {
  test('loads DTOs and reloads on history changes', async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    const cleanupCalls = new Set<string>();
    const environment = {
      isTauri: () => true,
      thumbnailUrl: (id: string) => `history-thumbnail://${id}`,
      commands: {
        listHistory: async () => {
          calls.push({ command: 'list_history' });
          return {
            entries: [
              {
                outcome: 'success' as const,
                id: 'entry-1',
                timestamp: '2026-09-13T10:00:00Z',
                captureSource: 'file',
                processingProfileId: 'default',
                processingProfileName: 'Default',
                settingsSnapshot,
                sequence: 1,
                hasThumbnail: true,
                hasOriginal: false,
                plainTextPreview: 'Preview text',
                matchSnippet: null,
                contentTags: [],
                hasPlainText: true,
                canRestoreFormatting: false,
                imageWidth: 1440,
                imageHeight: 900,
                storedBytes: 0,
                hasFirstInferenceOutput: false,
                hasSecondInferenceOutput: false,
              },
            ],
            pending: null,
            hasMore: false,
          };
        },
        getHistoryFacets: async () => ({ profiles: [] }),
        getHistoryPreview: async () => '<p>Stored preview</p>',
        previewHistoryFormatting: async () => ({
          previewHtml: '<p>Formatted</p>',
          hasPlainText: true,
        }),
        copyHistoryFormatted: async () => null,
        copyFailedHistoryDetails: async () => null,
        restoreHistoryEntry: async () => null,
        copyHistoryPlainText: async () => null,
        copyHistoryDebugHtml: async () => null,
        deleteHistoryEntry: async () => null,
        clearHistory: async () => null,
      },
      events: {
        historyChanged: eventSource<null>('history-changed', listeners, cleanupCalls),
        historyError: eventSource<{ code: string; message: string }>(
          'history-error',
          listeners,
          cleanupCalls,
        ),
      },
    };
    const controller = createHistoryController(
      { desktopAvailable: true },
      undefined,
      plainState,
      environment,
    );

    const cleanup = controller.mount();
    await Promise.resolve();
    await Promise.resolve();
    expect(controller.state.entries[0]).toEqual({
      id: 'entry-1',
      createdAt: '2026-09-13T10:00:00Z',
      sequence: 1,
      thumbnail: 'history-thumbnail://entry-1',
      hasOriginal: false,
      captureSource: 'file',
      status: 'completed',
      plainTextPreview: 'Preview text',
      matchSnippet: null,
      contentTags: [],
      hasPlainText: true,
      canRestoreFormatting: false,
      processingProfile: 'Default',
      settingsSnapshot,
      imageWidth: 1440,
      imageHeight: 900,
      hasFirstInferenceOutput: false,
      hasSecondInferenceOutput: false,
    });
    expect(controller.state.selectedId).toBe('entry-1');
    expect(calls[0]).toEqual({ command: 'list_history' });

    listeners.get('history-changed')?.({ payload: null });
    await Promise.resolve();
    await Promise.resolve();
    expect(calls).toHaveLength(2);

    cleanup();
    expect(cleanupCalls).toEqual(new Set(['history-changed', 'history-error']));
  });

  test('preserves the selected entry across reloads and falls back when it disappears', async () => {
    let rows = [
      { id: 'entry-1', plainTextPreview: 'First' },
      { id: 'entry-2', plainTextPreview: 'Second' },
    ];
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    const environment = {
      isTauri: () => true,
      thumbnailUrl: (id: string) => `history-thumbnail://${id}`,
      commands: {
        listHistory: async () => ({
          entries: rows.map((row, index) => ({
            outcome: 'success' as const,
            id: row.id,
            timestamp: '',
            captureSource: 'file',
            processingProfileId: '',
            processingProfileName: '',
            settingsSnapshot: null,
            sequence: rows.length - index,
            hasThumbnail: false,
            hasOriginal: false,
            plainTextPreview: row.plainTextPreview,
            matchSnippet: null,
            contentTags: [],
            hasPlainText: true,
            canRestoreFormatting: false,
            imageWidth: null,
            imageHeight: null,
            storedBytes: 0,
            hasFirstInferenceOutput: false,
            hasSecondInferenceOutput: false,
          })),
          pending: null,
          hasMore: false,
        }),
        getHistoryFacets: async () => ({ profiles: [] }),
        getHistoryPreview: async () => '<p>Stored preview</p>',
        previewHistoryFormatting: async () => ({
          previewHtml: '<p>Formatted</p>',
          hasPlainText: true,
        }),
        copyHistoryFormatted: async () => null,
        copyFailedHistoryDetails: async () => null,
        restoreHistoryEntry: async () => null,
        copyHistoryPlainText: async () => null,
        copyHistoryDebugHtml: async () => null,
        deleteHistoryEntry: async () => null,
        clearHistory: async () => null,
      },
      events: {
        historyChanged: eventSource<null>('history-changed', listeners),
        historyError: eventSource<{ code: string; message: string }>('history-error', listeners),
      },
    };
    const controller = createHistoryController(
      { desktopAvailable: true },
      undefined,
      plainState,
      environment,
    );
    const cleanup = controller.mount();
    await Promise.resolve();
    await Promise.resolve();

    expect(controller.state.selectedId).toBe('entry-1');
    controller.select('entry-2');
    await controller.load();
    expect(controller.state.selectedId).toBe('entry-2');

    rows = [{ id: 'entry-1', plainTextPreview: 'First' }];
    await controller.load();
    expect(controller.state.selectedId).toBe('entry-1');
    cleanup();
  });

  test('keeps a failed attempt selected and limits clipboard actions to saved results', async () => {
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    const calls: string[] = [];
    const attempt = {
      id: 'attempt-1',
      timestamp: '2026-09-13T10:00:00Z',
      captureSource: 'region',
      processingProfileName: 'Default',
      thumbnailDataUrl: 'data:image/png;base64,thumb',
      imageWidth: 100,
      imageHeight: 80,
    };
    let snapshot: HistoryWindowSnapshot = { entries: [], pending: attempt, hasMore: false };
    const environment = {
      isTauri: () => true,
      thumbnailUrl: (id: string) => `history-thumbnail://${id}`,
      commands: {
        listHistory: async () => snapshot,
        getHistoryFacets: async () => ({ profiles: [] }),
        getHistoryPreview: async () => '<p>Stored preview</p>',
        previewHistoryFormatting: async () => ({
          previewHtml: '<p>Formatted</p>',
          hasPlainText: true,
        }),
        copyHistoryFormatted: async () => null,
        copyFailedHistoryDetails: async (id: string) => {
          calls.push(`copy:${id}`);
          return null;
        },
        restoreHistoryEntry: async (id: string) => {
          calls.push(`restore:${id}`);
          return null;
        },
        copyHistoryPlainText: async (id: string) => {
          calls.push(`plain:${id}`);
          return null;
        },
        copyHistoryDebugHtml: async (id: string, kind: 'raw' | 'clean' | 'first' | 'second') => {
          calls.push(`debug:${id}:${kind}`);
          return null;
        },
        deleteHistoryEntry: async (id: string) => {
          calls.push(`delete:${id}`);
          return null;
        },
        clearHistory: async () => null,
      },
      events: {
        historyChanged: eventSource<null>('history-changed', listeners),
        historyError: eventSource<{ code: string; message: string }>('history-error', listeners),
      },
    };
    const controller = createHistoryController(
      { desktopAvailable: true },
      undefined,
      plainState,
      environment,
    );
    const cleanup = controller.mount();
    await Promise.resolve();
    await Promise.resolve();
    expect(controller.state.entries[0].status).toBe('pending');
    expect(controller.state.selectedId).toBe('attempt-1');
    expect(await controller.restore('attempt-1')).toBe(false);
    expect(await controller.copyPlainText('attempt-1')).toBe(false);
    expect(await controller.deleteEntry('attempt-1')).toBe(false);

    snapshot = {
      pending: null,
      hasMore: false,
      entries: [
        {
          outcome: 'failure',
          ...attempt,
          sequence: 1,
          hasThumbnail: true,
          hasOriginal: false,
          settingsSnapshot,
          errorCode: 'Inference',
          errorMessage: 'inference timed out',
          errorDetails: 'Inference failed (TimedOut): inference timed out.',
          hasRawInferenceOutput: true,
          matchSnippet: null,
        },
      ],
    };
    listeners.get('history-changed')?.({ payload: null });
    await Promise.resolve();
    await Promise.resolve();
    expect(controller.state.entries[0].status).toBe('failed');
    expect(controller.state.entries[0]).toMatchObject({
      errorDetails: 'Inference failed (TimedOut): inference timed out.',
    });
    expect(controller.state.entries[0].settingsSnapshot).toEqual(settingsSnapshot);
    expect(controller.state.selectedId).toBe('attempt-1');
    expect(await controller.restore('attempt-1')).toBe(false);
    expect(await controller.copyPlainText('attempt-1')).toBe(false);
    expect(await controller.copyDebugHtml('attempt-1', 'clean')).toBe(false);
    expect(await controller.copyDebugHtml('attempt-1', 'raw')).toBe(true);
    expect(controller.state.debugCopied).toEqual({ id: 'attempt-1', kind: 'raw' });
    expect(await controller.copyFailureDetails('attempt-1')).toBe(true);
    expect(await controller.deleteEntry('attempt-1')).toBe(true);
    expect(calls).toEqual(['debug:attempt-1:raw', 'copy:attempt-1', 'delete:attempt-1']);
    cleanup();
  });

  test('replaces a selected pending item with its published result', async () => {
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    const pending = {
      id: 'attempt-2',
      timestamp: '2026-09-13T10:00:00Z',
      captureSource: 'file',
      processingProfileName: 'Default',
      thumbnailDataUrl: null,
      imageWidth: 100,
      imageHeight: 80,
    };
    let snapshot: HistoryWindowSnapshot = { entries: [], pending, hasMore: false };
    const environment = {
      isTauri: () => true,
      thumbnailUrl: (id: string) => `history-thumbnail://${id}`,
      commands: {
        listHistory: async () => snapshot,
        getHistoryFacets: async () => ({ profiles: [] }),
        getHistoryPreview: async () => '<p>Stored preview</p>',
        previewHistoryFormatting: async () => ({
          previewHtml: '<p>Formatted</p>',
          hasPlainText: true,
        }),
        copyHistoryFormatted: async () => null,
        copyFailedHistoryDetails: async () => null,
        restoreHistoryEntry: async () => null,
        copyHistoryPlainText: async () => null,
        copyHistoryDebugHtml: async () => null,
        deleteHistoryEntry: async () => null,
        clearHistory: async () => null,
      },
      events: {
        historyChanged: eventSource<null>('history-changed', listeners),
        historyError: eventSource<{ code: string; message: string }>('history-error', listeners),
      },
    };
    const controller = createHistoryController(
      { desktopAvailable: true },
      undefined,
      plainState,
      environment,
    );
    const cleanup = controller.mount();
    await Promise.resolve();
    await Promise.resolve();
    expect(controller.state.entries[0].status).toBe('pending');

    snapshot = {
      pending: null,
      hasMore: false,
      entries: [
        {
          outcome: 'success' as const,
          id: pending.id,
          timestamp: pending.timestamp,
          captureSource: pending.captureSource,
          processingProfileId: 'default',
          processingProfileName: pending.processingProfileName,
          settingsSnapshot: null,
          sequence: 1,
          hasThumbnail: false,
          hasOriginal: false,
          plainTextPreview: 'Published text',
          matchSnippet: null,
          contentTags: [],
          hasPlainText: true,
          canRestoreFormatting: false,
          imageWidth: pending.imageWidth,
          imageHeight: pending.imageHeight,
          storedBytes: 0,
          hasFirstInferenceOutput: false,
          hasSecondInferenceOutput: false,
        },
      ],
    };
    listeners.get('history-changed')?.({ payload: null });
    await Promise.resolve();
    await Promise.resolve();
    expect(controller.state.selectedId).toBe(pending.id);
    expect(controller.state.entries[0].status).toBe('completed');
    cleanup();
  });

  test('normalizes split-stage output availability for advanced history actions', async () => {
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    const environment = {
      isTauri: () => true,
      thumbnailUrl: (id: string) => `history-thumbnail://${id}`,
      commands: {
        listHistory: async () => ({
          entries: [
            {
              outcome: 'success' as const,
              id: 'split',
              timestamp: '',
              captureSource: 'file',
              processingProfileId: '',
              processingProfileName: '',
              settingsSnapshot: null,
              sequence: 1,
              hasThumbnail: false,
              hasOriginal: false,
              plainTextPreview: '',
              matchSnippet: null,
              contentTags: [],
              hasPlainText: false,
              canRestoreFormatting: false,
              imageWidth: null,
              imageHeight: null,
              storedBytes: 0,
              hasFirstInferenceOutput: true,
              hasSecondInferenceOutput: true,
            },
          ],
          pending: null,
          hasMore: false,
        }),
        getHistoryFacets: async () => ({ profiles: [] }),
        getHistoryPreview: async () => '<p>Stored preview</p>',
        previewHistoryFormatting: async () => ({
          previewHtml: '<p>Formatted</p>',
          hasPlainText: true,
        }),
        copyHistoryFormatted: async () => null,
        copyFailedHistoryDetails: async () => null,
        restoreHistoryEntry: async () => null,
        copyHistoryPlainText: async () => null,
        copyHistoryDebugHtml: async () => null,
        deleteHistoryEntry: async () => null,
        clearHistory: async () => null,
      },
      events: {
        historyChanged: eventSource<null>('history-changed', listeners),
        historyError: eventSource<{ code: string; message: string }>('history-error', listeners),
      },
    };
    const controller = createHistoryController(
      { desktopAvailable: true },
      undefined,
      plainState,
      environment,
    );
    const cleanup = controller.mount();
    await Promise.resolve();
    await Promise.resolve();

    const entry = controller.state.entries[0];
    expect(entry.status).toBe('completed');
    if (entry.status === 'completed') {
      expect(entry.hasFirstInferenceOutput).toBe(true);
      expect(entry.hasSecondInferenceOutput).toBe(true);
      expect(entry.hasPlainText).toBe(false);
    }
    expect(await controller.copyPlainText('split')).toBe(false);
    cleanup();
  });

  test('routes sanitized history errors to the shared app error store', async () => {
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    let shown: { message: unknown; source: string; revision?: number } | undefined;
    const errors = {
      current: null,
      show(message: unknown, identity: { source: string; revision?: number }) {
        shown = { message, source: identity.source, revision: identity.revision };
      },
      dismiss() {},
    };
    const environment = {
      isTauri: () => true,
      thumbnailUrl: (id: string) => `history-thumbnail://${id}`,
      commands: {
        listHistory: async () => ({ entries: [], pending: null, hasMore: false }),
        getHistoryFacets: async () => ({ profiles: [] }),
        getHistoryPreview: async () => '<p>Stored preview</p>',
        previewHistoryFormatting: async () => ({
          previewHtml: '<p>Formatted</p>',
          hasPlainText: true,
        }),
        copyHistoryFormatted: async () => null,
        copyFailedHistoryDetails: async () => null,
        restoreHistoryEntry: async () => null,
        copyHistoryPlainText: async () => null,
        copyHistoryDebugHtml: async () => null,
        deleteHistoryEntry: async () => null,
        clearHistory: async () => null,
      },
      events: {
        historyChanged: eventSource<null>('history-changed', listeners),
        historyError: eventSource<{ code: string; message: string }>('history-error', listeners),
      },
    };
    const controller = createHistoryController(
      { desktopAvailable: true },
      errors,
      plainState,
      environment,
    );
    const cleanup = controller.mount();
    await Promise.resolve();
    listeners.get('history-error')?.({ payload: 'history database failed' });
    expect(shown).toEqual({
      message: 'history database failed',
      source: 'history',
      revision: 1,
    });
    cleanup();
  });

  test('routes rich, plain, debug, delete, and clear actions through native commands', async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const listeners = new Map<string, (event: { payload: unknown }) => void>();
    const environment = {
      isTauri: () => true,
      thumbnailUrl: (id: string) => `history-thumbnail://${id}`,
      commands: {
        listHistory: async () => {
          calls.push({ command: 'list_history' });
          return {
            entries: [
              {
                outcome: 'success' as const,
                id: 'entry-1',
                timestamp: '2026-09-13T10:00:00Z',
                captureSource: 'region',
                processingProfileId: 'default',
                processingProfileName: 'Default',
                settingsSnapshot: null,
                sequence: 1,
                hasThumbnail: false,
                hasOriginal: false,
                plainTextPreview: 'Result',
                matchSnippet: null,
                contentTags: [],
                hasPlainText: true,
                canRestoreFormatting: false,
                imageWidth: 100,
                imageHeight: 100,
                storedBytes: 0,
                hasFirstInferenceOutput: false,
                hasSecondInferenceOutput: false,
              },
            ],
            pending: null,
            hasMore: false,
          };
        },
        getHistoryFacets: async () => ({ profiles: [] }),
        getHistoryPreview: async () => '<p>Stored preview</p>',
        previewHistoryFormatting: async () => ({
          previewHtml: '<p>Formatted</p>',
          hasPlainText: true,
        }),
        copyHistoryFormatted: async () => null,
        copyFailedHistoryDetails: async () => null,
        restoreHistoryEntry: async (id: string) => {
          calls.push({ command: 'restore_history_entry', args: { id } });
          return null;
        },
        copyHistoryPlainText: async (id: string) => {
          calls.push({ command: 'copy_history_plain_text', args: { id } });
          return null;
        },
        copyHistoryDebugHtml: async (id: string, kind: 'raw' | 'clean' | 'first' | 'second') => {
          calls.push({ command: 'copy_history_debug_html', args: { id, kind } });
          return null;
        },
        deleteHistoryEntry: async (id: string) => {
          calls.push({ command: 'delete_history_entry', args: { id } });
          return null;
        },
        clearHistory: async () => {
          calls.push({ command: 'clear_history' });
          return null;
        },
      },
      events: {
        historyChanged: eventSource<null>('history-changed', listeners),
        historyError: eventSource<{ code: string; message: string }>('history-error', listeners),
      },
    };
    const controller = createHistoryController(
      { desktopAvailable: true },
      undefined,
      plainState,
      environment,
    );
    const cleanup = controller.mount();
    await Promise.resolve();
    await controller.restore('entry-1');
    await controller.copyPlainText('entry-1');
    expect(controller.state.resultCopied).toEqual({ id: 'entry-1', format: 'plain' });
    await controller.copyDebugHtml('entry-1', 'raw');
    await controller.deleteEntry('entry-1');
    await controller.clearAll();
    cleanup();

    expect(calls).toEqual([
      { command: 'list_history' },
      { command: 'restore_history_entry', args: { id: 'entry-1' } },
      { command: 'copy_history_plain_text', args: { id: 'entry-1' } },
      { command: 'copy_history_debug_html', args: { id: 'entry-1', kind: 'raw' } },
      { command: 'delete_history_entry', args: { id: 'entry-1' } },
      { command: 'list_history' },
      { command: 'clear_history' },
      { command: 'list_history' },
    ]);
  });
});

test('formatting obeys capture modes, ignores stale previews, and copies the displayed variation', async () => {
  const previews: Array<{
    resolve: (value: { previewHtml: string; hasPlainText: boolean }) => void;
    reject: (reason: unknown) => void;
  }> = [];
  const copies: Array<{ id: string; plain: boolean }> = [];
  const originals: string[] = [];
  const listeners = new Map<string, (event: { payload: unknown }) => void>();
  const row = {
    outcome: 'success' as const,
    id: 'prompt',
    timestamp: '2026-10-02T10:00:00Z',
    captureSource: 'file',
    processingProfileId: 'default',
    processingProfileName: 'Default',
    settingsSnapshot: {
      ...settingsSnapshot,
      applicationMode: 'prompt' as const,
    },
    sequence: 1,
    hasThumbnail: false,
    hasOriginal: false,
    plainTextPreview: 'Original',
    matchSnippet: null,
    contentTags: [],
    hasPlainText: true,
    canRestoreFormatting: true,
    imageWidth: 1,
    imageHeight: 1,
    storedBytes: 0,
    hasFirstInferenceOutput: false,
    hasSecondInferenceOutput: false,
  };
  const controller = createHistoryController({ desktopAvailable: true }, undefined, plainState, {
    isTauri: () => true,
    thumbnailUrl: (id: string) => `history-thumbnail://${id}`,
    commands: {
      listHistory: async () => ({
        pending: null,
        hasMore: false,
        entries: [
          row,
          {
            ...row,
            id: 'cleanup',
            sequence: 0,
            settingsSnapshot: {
              ...settingsSnapshot,
              applicationMode: 'html_cleanup',
              tables: false,
              colors: false,
              images: false,
            },
          },
        ],
      }),
      getHistoryFacets: async () => ({ profiles: [] }),
      getHistoryPreview: async () => '<p>Stored preview</p>',
      previewHistoryFormatting: async () =>
        new Promise((resolve, reject) => previews.push({ resolve, reject })),
      copyHistoryFormatted: async (id, _settings, plain) => {
        copies.push({ id, plain });
        return null;
      },
      restoreHistoryEntry: async (id) => {
        originals.push(id);
        return null;
      },
      copyHistoryPlainText: async () => null,
      copyFailedHistoryDetails: async () => null,
      copyHistoryDebugHtml: async () => null,
      deleteHistoryEntry: async () => null,
      clearHistory: async () => null,
    },
    events: {
      historyChanged: eventSource<null>('history-changed', listeners),
      historyError: eventSource<{ code: string; message: string }>('history-error', listeners),
    },
  });
  await controller.load();
  await controller.changeFormatting('prompt', { tables: true, colors: true, images: true });
  expect(previews).toHaveLength(0);
  const first = controller.changeFormatting('prompt', {
    tables: false,
    colors: false,
    images: true,
  });
  expect(await controller.restore('prompt')).toBe(false);
  const second = controller.changeFormatting('prompt', {
    tables: false,
    colors: false,
    images: false,
  });
  previews[1].resolve({ previewHtml: '<p>Latest</p>', hasPlainText: true });
  await second;
  previews[0].resolve({ previewHtml: '<p>Stale</p>', hasPlainText: true });
  await first;
  expect(controller.state.formatting?.previewHtml).toBe('<p>Latest</p>');
  expect(await controller.restore('prompt')).toBe(true);
  expect(await controller.copyPlainText('prompt')).toBe(true);
  expect(copies).toEqual([
    { id: 'prompt', plain: false },
    { id: 'prompt', plain: true },
  ]);
  controller.select('cleanup');
  const restoreFeatures = controller.changeFormatting('cleanup', {
    tables: true,
    colors: true,
    images: true,
  });
  previews[2].resolve({ previewHtml: '<table></table>', hasPlainText: false });
  await restoreFeatures;
  expect(controller.state.formatting?.settings).toEqual({
    tables: true,
    colors: true,
    images: true,
  });
  expect(await controller.copyPlainText('cleanup')).toBe(false);
  const pending = controller.changeFormatting('cleanup', {
    tables: true,
    colors: false,
    images: true,
  });
  expect(controller.state.formatting?.previewHtml).toBe('<table></table>');
  controller.select('prompt');
  previews[3].resolve({ previewHtml: '<p>Wrong entry</p>', hasPlainText: true });
  await pending;
  expect(controller.state.formatting?.previewHtml).toBe('<p>Stored preview</p>');
  expect(controller.state.formatting?.changed).toBe(false);
  expect(await controller.restore('prompt')).toBe(true);
  expect(originals).toEqual(['prompt']);
  const empty = controller.changeFormatting('prompt', {
    tables: false,
    colors: false,
    images: false,
  });
  previews[4].reject('Nothing to copy');
  await empty;
  expect(controller.state.formatting?.error).toBe('Nothing to copy');
  expect(await controller.restore('prompt')).toBe(false);
  const resetOptions = controller.changeFormatting('prompt', {
    tables: true,
    colors: false,
    images: true,
  });
  previews[5].resolve({ previewHtml: '<p>Cleaned initial options</p>', hasPlainText: true });
  await resetOptions;
  expect(controller.state.formatting?.error).toBeNull();
  expect(controller.state.formatting?.changed).toBe(true);
  expect(controller.state.formatting?.previewHtml).toBe('<p>Cleaned initial options</p>');
  controller.mount()();
});

function navigationSnapshot(pendingId: string | null = null): HistoryWindowSnapshot {
  const common = {
    timestamp: '2026-10-30T12:00:00Z',
    captureSource: 'region',
    processingProfileName: 'Default',
    settingsSnapshot,
    thumbnailDataUrl: '',
    imageWidth: null,
    imageHeight: null,
  };
  const snapshot: HistoryWindowSnapshot = {
    entries: [
      {
        outcome: 'failure',
        id: 'other',
        sequence: 3,
        hasThumbnail: false,
        hasOriginal: false,
        ...common,
        errorCode: 'inference',
        errorMessage: 'Failed',
        errorDetails: null,
        hasRawInferenceOutput: false,
        matchSnippet: null,
      },
      {
        outcome: 'success',
        id: 'success',
        sequence: 2,
        hasThumbnail: false,
        hasOriginal: false,
        ...common,
        processingProfileId: 'default',
        plainTextPreview: 'Result',
        matchSnippet: null,
        contentTags: [],
        hasPlainText: true,
        canRestoreFormatting: false,
        storedBytes: 0,
        hasFirstInferenceOutput: false,
        hasSecondInferenceOutput: false,
      },
      {
        outcome: 'failure',
        id: 'capture',
        sequence: 1,
        hasThumbnail: false,
        hasOriginal: false,
        ...common,
        errorCode: 'inference',
        errorMessage: 'Capture failed',
        errorDetails: null,
        hasRawInferenceOutput: false,
        matchSnippet: null,
      },
    ],
    pending: pendingId ? { id: pendingId, ...common } : null,
    hasMore: false,
  };
  if (pendingId) snapshot.entries = snapshot.entries.filter((entry) => entry.id !== pendingId);
  return snapshot;
}

type ListHistory = (
  beforeSequence: number | null,
  minSequence: number | null,
  limit: number,
  filter: HistoryFilter,
) => Promise<HistoryWindowSnapshot>;

function navigationEnvironment(listHistory: ListHistory) {
  const listeners = new Map<string, (event: { payload: unknown }) => void>();
  return {
    isTauri: () => true,
    thumbnailUrl: (id: string) => `history-thumbnail://${id}`,
    commands: {
      listHistory,
      getHistoryFacets: async () => ({ profiles: [] }),
      getHistoryPreview: async () => '<p>Stored preview</p>',
      previewHistoryFormatting: async () => ({ previewHtml: '', hasPlainText: true }),
      copyHistoryFormatted: async () => null,
      copyFailedHistoryDetails: async () => null,
      restoreHistoryEntry: async () => null,
      copyHistoryPlainText: async () => null,
      copyHistoryDebugHtml: async () => null,
      deleteHistoryEntry: async () => null,
      clearHistory: async () => null,
    },
    events: {
      historyChanged: eventSource<null>('history-changed', listeners),
      historyError: eventSource<{ code: string; message: string }>('history-error', listeners),
    },
  };
}

test('opens the requested successful or pending capture and retains it when it fails', async () => {
  let snapshot = navigationSnapshot();
  const controller = createHistoryController(
    { desktopAvailable: true },
    undefined,
    plainState,
    navigationEnvironment(async () => snapshot),
  );
  await controller.load();
  expect(controller.state.selectedId).toBe('other');
  await controller.open('success');
  expect(controller.state.selectedId).toBe('success');
  snapshot = navigationSnapshot('capture');
  await controller.open('capture');
  expect(
    controller.state.entries.find((entry) => entry.id === controller.state.selectedId)?.status,
  ).toBe('pending');
  snapshot = navigationSnapshot();
  await controller.load();
  expect(controller.state.selectedId).toBe('capture');
  expect(
    controller.state.entries.find((entry) => entry.id === controller.state.selectedId)?.status,
  ).toBe('failed');
  await controller.open('deleted');
  expect(controller.state.selectedId).toBe('capture');
});

test('history navigation survives a competing refresh and yields to a manual selection', async () => {
  const requests: Array<(snapshot: HistoryWindowSnapshot) => void> = [];
  const controller = createHistoryController(
    { desktopAvailable: true },
    undefined,
    plainState,
    navigationEnvironment(() => new Promise((resolve) => requests.push(resolve))),
  );
  const open = controller.open('success');
  const refresh = controller.load();
  requests[0](navigationSnapshot());
  await open;
  requests[1](navigationSnapshot());
  await refresh;
  expect(controller.state.selectedId).toBe('success');
  const anotherOpen = controller.open('capture');
  controller.select('other');
  requests[2](navigationSnapshot());
  await anotherOpen;
  expect(controller.state.selectedId).toBe('other');
});

test('pages older entries and refreshes only the loaded window', async () => {
  const requests: Array<[number | null, number | null, number]> = [];
  const previewRequests: string[] = [];
  const total = 120;
  const row = (sequence: number) => ({
    outcome: 'success' as const,
    id: `entry-${sequence}`,
    sequence,
    timestamp: '',
    captureSource: 'region',
    processingProfileId: 'default',
    processingProfileName: 'Default',
    settingsSnapshot: null,
    hasThumbnail: sequence % 2 === 0,
    hasOriginal: false,
    plainTextPreview: `Entry ${sequence}`,
    matchSnippet: null,
    contentTags: [],
    hasPlainText: true,
    canRestoreFormatting: false,
    imageWidth: null,
    imageHeight: null,
    storedBytes: 0,
    hasFirstInferenceOutput: false,
    hasSecondInferenceOutput: false,
  });
  const environment = navigationEnvironment(async () => {
    throw new Error('unused');
  });
  environment.commands.listHistory = async (beforeSequence, minSequence, limit) => {
    requests.push([beforeSequence, minSequence, limit]);
    const sequences = Array.from({ length: total }, (_, index) => total - index).filter(
      (sequence) =>
        (beforeSequence === null || sequence < beforeSequence) &&
        (minSequence === null || sequence >= minSequence),
    );
    const page = sequences.slice(0, limit);
    return {
      entries: page.map(row),
      pending: null,
      hasMore: page.length > 0 && page[page.length - 1] > 1,
    };
  };
  const controller = createHistoryController({ desktopAvailable: true }, undefined, plainState, {
    ...environment,
    commands: {
      ...environment.commands,
      getHistoryFacets: async () => ({ profiles: [] }),
      getHistoryPreview: async (id: string) => {
        previewRequests.push(id);
        return `<p>${id}</p>`;
      },
    },
  });

  await controller.load();
  expect(requests).toEqual([[null, null, 50]]);
  expect(controller.state.entries).toHaveLength(50);
  expect(controller.state.hasMore).toBe(true);
  expect(controller.state.entries[0].thumbnail).toBe('history-thumbnail://entry-120');
  expect(controller.state.entries[1].thumbnail).toBeNull();
  await Promise.resolve();
  expect(previewRequests).toEqual(['entry-120']);
  expect(controller.state.formatting?.previewHtml).toBe('<p>entry-120</p>');
  expect(controller.state.formatting?.previewLoading).toBe(false);

  await controller.loadMore();
  expect(requests[1]).toEqual([71, null, 50]);
  expect(controller.state.entries).toHaveLength(100);
  expect(controller.state.entries.at(-1)?.sequence).toBe(21);

  await controller.load();
  expect(requests[2]).toEqual([null, 21, 150]);
  expect(controller.state.entries).toHaveLength(100);
  expect(previewRequests).toEqual(['entry-120']);

  await controller.loadMore();
  expect(controller.state.entries).toHaveLength(120);
  expect(controller.state.hasMore).toBe(false);
  await controller.loadMore();
  expect(requests).toHaveLength(4);
});

test('filters the list, hides the pending capture, and clears filters to open an entry', async () => {
  const filters: HistoryFilter[] = [];
  const environment = navigationEnvironment(async (_before, _min, _limit, filter) => {
    filters.push(filter);
    return navigationSnapshot('pending');
  });
  const controller = createHistoryController(
    { desktopAvailable: true },
    undefined,
    plainState,
    environment,
  );
  await controller.load();
  expect(controller.state.entries[0].status).toBe('pending');
  expect(filters[0]).toEqual({
    query: null,
    outcome: null,
    profileId: null,
    since: null,
    until: null,
    contains: [],
  });

  controller.setFilter({ outcome: 'failure', profileId: 'default' });
  await Promise.resolve();
  await Promise.resolve();
  expect(filters[1]).toMatchObject({ outcome: 'failure', profileId: 'default' });
  expect(controller.state.entries.some((entry) => entry.status === 'pending')).toBe(false);

  controller.setFilter({ query: '  invoice ' });
  expect(filters).toHaveLength(2);
  await new Promise((resolve) => setTimeout(resolve, 250));
  expect(filters[2]).toMatchObject({ query: 'invoice', outcome: 'failure' });

  await controller.open('success');
  expect(controller.state.filter.query).toBe('');
  expect(filters.at(-1)?.outcome).toBeNull();
  expect(controller.state.selectedId).toBe('success');
});
