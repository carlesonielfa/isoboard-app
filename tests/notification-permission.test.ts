import { expect, test } from 'bun:test';
import { createNotificationPermissionController } from '../src/lib/notification-permission.svelte';
import type { NotificationPermission } from '../src/lib/generated/bindings';

test('a stale permission check cannot overwrite a new authorization result', async () => {
  let finishCheck!: (status: NotificationPermission) => void;
  const controller = createNotificationPermissionController({
    isDesktop: () => true,
    stateFactory: (value) => value,
    commands: {
      openNotificationSettings: async () => null,
      getNotificationPermission: () =>
        new Promise((resolve) => {
          finishCheck = resolve;
        }),
      requestNotificationPermission: async () => 'granted',
    },
  });
  const checking = controller.refresh();
  await controller.request();
  finishCheck('required');
  await checking;
  expect(controller.state.status).toBe('granted');
  expect(controller.state.busy).toBe(false);
});

test('denial is a permission state and a later system change is refreshed', async () => {
  const controller = createNotificationPermissionController({
    isDesktop: () => true,
    stateFactory: (value) => value,
    commands: {
      openNotificationSettings: async () => null,
      getNotificationPermission: async () => 'granted',
      requestNotificationPermission: async () => 'denied',
    },
  });
  await controller.request();
  expect(controller.state.status).toBe('denied');
  expect(controller.state.error).toBeNull();
  expect(controller.state.busy).toBe(false);
  await controller.refresh();
  expect(controller.state.status).toBe('granted');
});

test('an undecided request stays neutral and resolves automatically after authorization changes', async () => {
  let permission: NotificationPermission = 'required';
  const controller = createNotificationPermissionController({
    isDesktop: () => true,
    stateFactory: (value) => value,
    commands: {
      openNotificationSettings: async () => null,
      getNotificationPermission: async () => permission,
      requestNotificationPermission: async () => {
        throw new Error('native request failed');
      },
    },
  });
  await controller.request();

  expect(controller.state.requestPending).toBe(true);
  expect(controller.state.busy).toBe(false);
  expect(controller.state.error).toBeNull();

  await controller.refresh();
  expect(controller.state.requestPending).toBe(true);
  expect(controller.state.error).toBeNull();

  permission = 'denied';
  await controller.refresh();
  expect(controller.state.status).toBe('denied');
  expect(controller.state.requestPending).toBe(false);
  expect(controller.state.busy).toBe(false);
  expect(controller.state.error).toBeNull();
});

test('a focus refresh arriving during authorization is replayed after the request completes', async () => {
  let finishRequest!: (status: NotificationPermission) => void;
  let checks = 0;
  const controller = createNotificationPermissionController({
    isDesktop: () => true,
    stateFactory: (value) => value,
    commands: {
      openNotificationSettings: async () => null,
      getNotificationPermission: async () => {
        checks += 1;
        return 'granted';
      },
      requestNotificationPermission: () =>
        new Promise((resolve) => {
          finishRequest = resolve;
        }),
    },
  });
  const requesting = controller.request();
  await controller.refresh();
  finishRequest('required');
  await requesting;
  await Promise.resolve();
  expect(controller.state.status).toBe('granted');
  expect(controller.state.requestPending).toBe(false);
  expect(checks).toBe(1);
});

test('the first native window activation refreshes permission and stops after unmount', async () => {
  let focus!: () => void;
  let checks = 0;
  let permission: NotificationPermission = 'required';
  let unlistened = false;
  const controller = createNotificationPermissionController({
    isDesktop: () => true,
    stateFactory: (value) => value,
    listenFocus: async (onFocus) => {
      focus = onFocus;
      return () => {
        unlistened = true;
      };
    },
    commands: {
      openNotificationSettings: async () => null,
      getNotificationPermission: async () => {
        checks += 1;
        return permission;
      },
      requestNotificationPermission: async () => 'required',
    },
  });
  const cleanup = controller.mount();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(controller.state.status).toBe('required');

  permission = 'granted';
  focus();
  await Promise.resolve();
  expect(controller.state.status).toBe('granted');
  expect(checks).toBe(2);

  cleanup();
  focus();
  await Promise.resolve();
  expect(checks).toBe(2);
  expect(unlistened).toBe(true);
});
