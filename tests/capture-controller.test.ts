/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import { createCaptureController } from '../src/lib/capture-controller.svelte';
import type { CaptureStatus } from '../src/lib/capture-status';

const plainState = <T>(value: T): T => value;
type NativeEvent<T> = { event: `${string}`; id: number; payload: T };

describe('capture controller native status path', () => {
  test('marks the browser surface unavailable without a capture error', () => {
    const availability = { desktopAvailable: true };
    const controller = createCaptureController(availability, undefined, plainState, {
      isTauri: () => false,
      windowPresentation: { hide: async () => {}, restore: async () => {} },
      commands: {
        startRegionCaptureCommand: async () => null,
        openImageCommand: async () => null,
        getCaptureSnapshot: async () => ({ status: 'idle', sequence: 0 }),
      },
      events: {
        captureStatus: {
          listen: async () => () => undefined,
        },
      },
    });

    const cleanup = controller.mount();

    expect(availability.desktopAvailable).toBe(false);
    expect(controller.state.phase).toBe('failed');
    expect(controller.state.errorMessage).toBeNull();
    expect(controller.state.statusMessage).toContain('Open the isoboard desktop app');
    cleanup();
  });

  test('applies serialized lifecycle events and preserves HTML across a new cancelled run', async () => {
    let listener: ((event: { payload: CaptureStatus }) => void) | undefined;
    const snapshot = JSON.stringify({ status: 'idle', sequence: 0 });
    const environment = {
      isTauri: () => true,
      windowPresentation: { hide: async () => {}, restore: async () => {} },
      commands: {
        startRegionCaptureCommand: async () => null,
        openImageCommand: async () => null,
        getCaptureSnapshot: async () => JSON.parse(snapshot) as CaptureStatus,
      },
      events: {
        captureStatus: {
          listen: async (handler: (event: NativeEvent<CaptureStatus>) => void) => {
            listener = (event) =>
              handler({ event: 'capture-status', id: 1, payload: event.payload });
            return () => {
              listener = undefined;
            };
          },
        },
      },
    };
    const controller = createCaptureController(
      { desktopAvailable: true },
      undefined,
      plainState,
      environment,
    );

    const cleanup = controller.mount();
    await Promise.resolve();
    await Promise.resolve();
    expect(listener).toBeDefined();

    const emit = (payload: CaptureStatus): void => {
      listener?.({ payload: JSON.parse(JSON.stringify(payload)) as CaptureStatus });
    };
    const context = { source: 'region' as const, profileId: 'default', bindingId: null };

    emit({ status: 'started', sequence: 1, ...context });
    emit({
      status: 'imageReady',
      sequence: 1,
      ...context,
      image: { mimeType: 'image/png', dataBase64: 'captured', width: 320, height: 200 },
    });
    emit({ status: 'processing', revision: 0, sequence: 1, ...context });
    emit({ status: 'completed', sequence: 1, ...context, html: '<p>first</p>' });
    expect(controller.state.htmlResult).toBe('<p>first</p>');

    emit({ status: 'started', sequence: 2, ...context });
    expect(controller.state.htmlResult).toBe('<p>first</p>');
    emit({
      status: 'imageReady',
      sequence: 2,
      ...context,
      image: { mimeType: 'image/jpeg', dataBase64: 'second', width: 640, height: 480 },
    });
    emit({ status: 'cancelled', sequence: 2, ...context });

    expect(controller.state.htmlResult).toBe('<p>first</p>');
    expect(controller.state.preview?.src).toBe('data:image/jpeg;base64,second');
    cleanup();
  });
});

describe('capture initiated from the window', () => {
  function fixture(
    options: {
      hide?: () => Promise<void>;
      start?: () => Promise<null>;
      snapshot?: () => Promise<CaptureStatus>;
    } = {},
  ) {
    const calls: string[] = [];
    const controller = createCaptureController({ desktopAvailable: true }, undefined, plainState, {
      isTauri: () => true,
      windowPresentation: {
        hide: async () => {
          calls.push('hide');
          await options.hide?.();
        },
        restore: async () => {
          calls.push('restore');
        },
      },
      commands: {
        startRegionCaptureCommand: async () => {
          calls.push('capture');
          return options.start ? options.start() : null;
        },
        openImageCommand: async () => {
          calls.push('file');
          return null;
        },
        getCaptureSnapshot: options.snapshot ?? (async () => ({ status: 'idle', sequence: 0 })),
      },
      events: { captureStatus: { listen: async () => () => {} } },
    });
    return { controller, calls };
  }

  test('waits until the window is hidden before selecting and rejects duplicate clicks', async () => {
    let finishHide!: () => void;
    const { controller, calls } = fixture({
      hide: () =>
        new Promise<void>((resolve) => {
          finishHide = resolve;
        }),
    });
    const first = controller.startCapture('region', { hideWindow: true });
    await controller.startCapture('region', { hideWindow: true });
    expect(calls).toEqual(['hide']);
    finishHide();
    await first;
    expect(calls).toEqual(['hide', 'capture']);
    expect(controller.state.busy).toBe(true);
  });

  test('restores the window and clears local busy state if capture is rejected', async () => {
    const { controller, calls } = fixture({
      start: async () => {
        throw new Error('Capture setup is incomplete.');
      },
    });
    await controller.startCapture('region', { hideWindow: true });
    expect(calls).toEqual(['hide', 'capture', 'restore']);
    expect(controller.state.busy).toBe(false);
    expect(controller.state.phase).toBe('failed');
    expect(controller.state.errorMessage).toBe('Capture setup is incomplete.');
    await controller.startCapture('region', { hideWindow: true });
    expect(calls).toEqual(['hide', 'capture', 'restore', 'hide', 'capture', 'restore']);
  });

  test('does not start selection when the window cannot be hidden', async () => {
    const { controller, calls } = fixture({
      hide: async () => {
        throw new Error('Window hide failed');
      },
    });
    await controller.startCapture('region', { hideWindow: true });
    expect(calls).toEqual(['hide']);
    expect(controller.state.busy).toBe(false);
    expect(controller.state.errorMessage).toBe('Window hide failed');
  });

  test('does not restore over a region selection already in progress', async () => {
    const { controller, calls } = fixture({
      start: async () => {
        throw new Error('capture is busy');
      },
      snapshot: async () => ({
        status: 'started',
        sequence: 1,
        source: 'region',
        profileId: 'default',
        bindingId: null,
      }),
    });
    await controller.startCapture('region', { hideWindow: true });
    expect(calls).toEqual(['hide', 'capture']);
    expect(controller.state.busy).toBe(true);
  });

  test('restores the window when capture state cannot be confirmed', async () => {
    const { controller, calls } = fixture({
      start: async () => {
        throw new Error('IPC failed');
      },
      snapshot: async () => {
        throw new Error('Snapshot unavailable');
      },
    });
    await controller.startCapture('region', { hideWindow: true });
    expect(calls).toEqual(['hide', 'capture', 'restore']);
    expect(controller.state.errorMessage).toContain('Could not confirm capture state');
  });

  test('leaves window visibility unchanged for existing non-window capture paths', async () => {
    const { controller, calls } = fixture();
    await controller.startCapture('file');
    expect(calls).toEqual(['file']);
  });
});
