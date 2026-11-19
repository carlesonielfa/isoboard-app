/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import { createScreenCapturePermissionController } from '../src/lib/screen-capture-permission.svelte';

const plainState = <T>(value: T): T => value;

describe('screen capture permission setup', () => {
  test('the first native focus refreshes access and a request-time focus is replayed', async () => {
    let focus!: () => void;
    let finishRequest!: (status: 'required') => void;
    let allowed = false;
    let stopped = false;
    const controller = createScreenCapturePermissionController(plainState, {
      isTauri: () => true,
      listenFocus: async (onFocus) => {
        focus = onFocus;
        return () => {
          stopped = true;
        };
      },
      commands: {
        getScreenCapturePermissionStatus: async () => (allowed ? 'restartRequired' : 'required'),
        requestScreenCapturePermission: () =>
          new Promise((resolve) => {
            finishRequest = resolve;
          }),
        openScreenCaptureSettings: async () => null,
        restartAfterScreenCaptureAccess: async () => null,
      },
    });
    const cleanup = controller.mount();
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(controller.state.status).toBe('required');
    const requesting = controller.request();
    allowed = true;
    focus();
    finishRequest('required');
    await requesting;
    await Promise.resolve();
    expect(controller.state.status).toBe('restartRequired');
    expect(controller.state.busy).toBe(false);
    expect(controller.state.error).toBeNull();
    cleanup();
    expect(stopped).toBe(true);
  });

  test('an older screen-access response cannot undo a completed request', async () => {
    let finishCheck!: (status: 'required') => void;
    const controller = createScreenCapturePermissionController(plainState, {
      isTauri: () => true,
      commands: {
        getScreenCapturePermissionStatus: () =>
          new Promise((resolve) => {
            finishCheck = resolve;
          }),
        requestScreenCapturePermission: async () => 'granted',
        openScreenCaptureSettings: async () => null,
        restartAfterScreenCaptureAccess: async () => null,
      },
    });
    const checking = controller.refresh();
    await controller.request();
    finishCheck('required');
    await checking;
    expect(controller.state.status).toBe('granted');
  });

  test('checks without prompting, then recovers after a declined request', async () => {
    let allowed = false;
    let requests = 0;
    let settingsOpens = 0;
    let restarts = 0;
    const controller = createScreenCapturePermissionController(plainState, {
      isTauri: () => true,
      commands: {
        getScreenCapturePermissionStatus: async () => (allowed ? 'granted' : 'required'),
        requestScreenCapturePermission: async () => {
          requests += 1;
          return allowed ? 'granted' : 'required';
        },
        openScreenCaptureSettings: async () => {
          settingsOpens += 1;
          return null;
        },
        restartAfterScreenCaptureAccess: async () => {
          restarts += 1;
          return null;
        },
      },
    });

    await controller.refresh();
    expect(controller.state.status).toBe('required');
    expect(requests).toBe(0);

    await controller.request();
    expect(controller.state.status).toBe('required');
    expect(controller.state.requested).toBe(true);
    expect(requests).toBe(1);

    await controller.restart();
    expect(restarts).toBe(1);

    await controller.openSettings();
    expect(settingsOpens).toBe(1);
    allowed = true;
    await controller.refresh();
    expect(controller.state.status).toBe('granted');
  });

  test('keeps the browser preview unsupported without native permission calls', async () => {
    const controller = createScreenCapturePermissionController(plainState, {
      isTauri: () => false,
      commands: {
        getScreenCapturePermissionStatus: async () => {
          throw new Error('unexpected native call');
        },
        requestScreenCapturePermission: async () => {
          throw new Error('unexpected native call');
        },
        openScreenCaptureSettings: async () => {
          throw new Error('unexpected native call');
        },
        restartAfterScreenCaptureAccess: async () => {
          throw new Error('unexpected native call');
        },
      },
    });

    await controller.refresh();
    await controller.request();
    await controller.restart();
    expect(controller.state.status).toBe('unsupported');
    expect(controller.state.requested).toBe(false);
  });
});
