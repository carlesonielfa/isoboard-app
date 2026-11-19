import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { commands } from '$lib/ipc';
import type { ScreenCapturePermissionStatus } from '$lib/generated/bindings';

export type ScreenCapturePermissionViewState = {
  status: ScreenCapturePermissionStatus | 'checking';
  busy: boolean;
  requested: boolean;
  error: string | null;
};

type StateFactory = <T>(value: T) => T;
type PermissionEnvironment = {
  isTauri: () => boolean;
  commands: Pick<
    typeof commands,
    | 'getScreenCapturePermissionStatus'
    | 'requestScreenCapturePermission'
    | 'openScreenCaptureSettings'
    | 'restartAfterScreenCaptureAccess'
  >;
  listenFocus?: (onFocus: () => void) => Promise<() => void>;
};
const defaultEnvironment: PermissionEnvironment = {
  isTauri,
  commands,
  listenFocus: (onFocus) =>
    getCurrentWindow().onFocusChanged(({ payload }) => {
      if (payload) onFocus();
    }),
};
function createReactiveState<T>(initialState: T): T {
  const state = $state(initialState);
  return state;
}

export function createScreenCapturePermissionController(
  stateFactory: StateFactory = createReactiveState,
  environment: PermissionEnvironment = defaultEnvironment,
) {
  const state = stateFactory<ScreenCapturePermissionViewState>({
    status: 'checking',
    busy: false,
    requested: false,
    error: null,
  });
  let sequence = 0;
  let refreshQueued = false;
  let disposed = false;
  let unlistenFocus: (() => void) | undefined;

  function finishAction() {
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
    if (!environment.isTauri()) {
      state.status = 'unsupported';
      return;
    }
    const current = ++sequence;
    try {
      const status = await environment.commands.getScreenCapturePermissionStatus();
      if (current !== sequence) return;
      state.status = status;
      state.error = null;
    } catch (error) {
      if (current !== sequence) return;
      state.status = 'required';
      state.error = `Could not check screen access: ${String(error)}`;
    }
  }
  async function request(): Promise<void> {
    if (disposed || state.busy || !environment.isTauri()) return;
    const current = ++sequence;
    state.busy = true;
    state.requested = true;
    state.error = null;
    try {
      const status = await environment.commands.requestScreenCapturePermission();
      if (current !== sequence) return;
      state.status = status;
    } catch (error) {
      if (current === sequence) state.error = `Could not request screen access: ${String(error)}`;
    } finally {
      if (!disposed) finishAction();
    }
  }
  async function openSettings(): Promise<void> {
    if (disposed || state.busy || !environment.isTauri()) return;
    ++sequence;
    state.busy = true;
    state.requested = true;
    state.error = null;
    try {
      await environment.commands.openScreenCaptureSettings();
    } catch (error) {
      if (!disposed) state.error = `Could not open System Settings: ${String(error)}`;
    } finally {
      if (!disposed) finishAction();
    }
  }
  async function restart(): Promise<void> {
    if (disposed || state.busy || !environment.isTauri()) return;
    state.error = null;
    try {
      await environment.commands.restartAfterScreenCaptureAccess();
    } catch (error) {
      if (!disposed) state.error = `Could not restart isoboard: ${String(error)}`;
    }
  }
  function mount(): () => void {
    if (!environment.isTauri()) void refresh();
    else {
      void (environment.listenFocus ?? defaultEnvironment.listenFocus!)(() => void refresh())
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
  return { state, mount, refresh, request, openSettings, restart };
}
