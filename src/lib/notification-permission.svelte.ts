import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { commands } from '$lib/ipc';
import type { NotificationPermission } from '$lib/generated/bindings';

export type NotificationPermissionState = {
  status: NotificationPermission | 'checking';
  busy: boolean;
  requestPending: boolean;
  error: string | null;
};

type Environment = {
  isDesktop: () => boolean;
  commands: Pick<
    typeof commands,
    'getNotificationPermission' | 'requestNotificationPermission' | 'openNotificationSettings'
  >;
  stateFactory: (value: NotificationPermissionState) => NotificationPermissionState;
  listenFocus: (onFocus: () => void) => Promise<() => void>;
};

function reactive(value: NotificationPermissionState): NotificationPermissionState {
  const state = $state(value);
  return state;
}

export function createNotificationPermissionController(overrides: Partial<Environment> = {}) {
  const environment: Environment = {
    isDesktop: isTauri,
    commands,
    stateFactory: reactive,
    listenFocus: (onFocus) =>
      getCurrentWindow().onFocusChanged(({ payload }) => {
        if (payload) onFocus();
      }),
    ...overrides,
  };
  const state = environment.stateFactory({
    status: 'checking',
    busy: false,
    requestPending: false,
    error: null,
  });
  let sequence = 0;
  let refreshQueued = false;
  let disposed = false;
  let unlistenFocus: (() => void) | undefined;

  function finishAction(): void {
    state.busy = false;
    if (refreshQueued && !disposed) {
      refreshQueued = false;
      void refresh();
    }
  }

  async function refresh(): Promise<void> {
    if (disposed) return;
    if (state.busy) {
      refreshQueued = true;
      return;
    }
    if (!environment.isDesktop()) {
      state.status = 'unsupported';
      return;
    }
    const current = ++sequence;
    try {
      const status = await environment.commands.getNotificationPermission();
      if (current !== sequence) return;
      state.status = status;
      if (status === 'required' && state.requestPending) return;
      state.requestPending = false;
      state.error = null;
    } catch (error) {
      if (current !== sequence) return;
      state.status = 'required';
      state.error = `Could not check notifications: ${String(error)}`;
    }
  }

  async function request(): Promise<void> {
    if (disposed || state.busy || !environment.isDesktop()) return;
    const current = ++sequence;
    state.busy = true;
    state.requestPending = false;
    state.error = null;
    try {
      const status = await environment.commands.requestNotificationPermission();
      if (current !== sequence) return;
      state.status = status;
      state.requestPending = state.status === 'required';
    } catch (error) {
      if (current !== sequence) return;
      const requestError = `Could not enable notifications: ${String(error)}`;
      try {
        const status = await environment.commands.getNotificationPermission();
        if (current !== sequence) return;
        state.status = status;
        if (status === 'required') {
          state.requestPending = true;
        } else {
          state.error = null;
        }
      } catch (checkError) {
        if (current !== sequence) return;
        state.error = `${requestError}. Could not check notifications: ${String(checkError)}`;
      }
    } finally {
      if (!disposed) finishAction();
    }
  }

  async function openSettings(): Promise<void> {
    if (disposed || state.busy || !environment.isDesktop()) return;
    ++sequence;
    state.busy = true;
    state.error = null;
    try {
      await environment.commands.openNotificationSettings();
    } catch (error) {
      if (!disposed) state.error = `Could not open System Settings: ${String(error)}`;
    } finally {
      if (!disposed) finishAction();
    }
  }

  function mount(): () => void {
    if (!environment.isDesktop()) {
      void refresh();
    } else {
      void environment
        .listenFocus(() => void refresh())
        .then((unlisten) => {
          if (disposed) {
            unlisten();
            return;
          }
          unlistenFocus = unlisten;
          void refresh();
        })
        .catch(async (error: unknown) => {
          if (disposed) return;
          await refresh();
          if (!disposed) state.error = `Could not track app focus: ${String(error)}`;
        });
    }
    return () => {
      disposed = true;
      ++sequence;
      refreshQueued = false;
      unlistenFocus?.();
    };
  }

  return { state, refresh, request, openSettings, mount };
}
