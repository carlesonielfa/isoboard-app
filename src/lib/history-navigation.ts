import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { commands, events } from '$lib/ipc';

type Cleanup = () => void;
type Subscribe = (wake: () => void) => Promise<Cleanup>;

export type HistoryNavigationEnvironment = {
  available: () => boolean;
  ready: () => Promise<boolean>;
  pending: typeof commands.getPendingHistoryNavigation;
  acknowledge: typeof commands.acknowledgeHistoryNavigation;
  listenRequests: Subscribe;
  listenFocus: Subscribe;
};

const nativeEnvironment: HistoryNavigationEnvironment = {
  available: isTauri,
  ready: async () => {
    const window = getCurrentWindow();
    return (await window.isVisible()) && (await window.isFocused());
  },
  pending: () => commands.getPendingHistoryNavigation(),
  acknowledge: (requestId) => commands.acknowledgeHistoryNavigation(requestId),
  listenRequests: (wake) => events.openHistory.listen(() => wake()),
  listenFocus: (wake) =>
    getCurrentWindow().onFocusChanged(({ payload }) => {
      if (payload) wake();
    }),
};

export function createHistoryNavigation(
  open: (entryId: string | null) => Promise<void>,
  reportError: (error: unknown) => void,
  environment: HistoryNavigationEnvironment = nativeEnvironment,
) {
  let disposed = true;
  let running = false;
  let dirty = false;

  async function sync(): Promise<void> {
    dirty = true;
    if (disposed || running) return;
    running = true;
    try {
      while (dirty && !disposed) {
        dirty = false;
        if (!(await environment.ready())) break;
        const request = await environment.pending();
        if (!request || disposed) continue;
        if (!(await environment.ready())) break;
        await open(request.entryId);
        if (disposed) break;
        await environment.acknowledge(request.requestId);
        // A newer request may have arrived while this one was being applied.
        dirty = true;
      }
    } catch (error) {
      if (!disposed) reportError(error);
    } finally {
      running = false;
      if (dirty && !disposed) void sync();
    }
  }

  function mount(): Cleanup {
    if (!environment.available()) return () => {};
    disposed = false;
    const cleanups: Cleanup[] = [];
    async function subscribe(listen: Subscribe): Promise<void> {
      try {
        const cleanup = await listen(() => void sync());
        if (disposed) cleanup();
        else cleanups.push(cleanup);
      } catch (error) {
        if (!disposed) reportError(error);
      }
    }
    void Promise.all([
      subscribe(environment.listenRequests),
      subscribe(environment.listenFocus),
    ]).then(() => {
      if (!disposed) void sync();
    });
    return () => {
      disposed = true;
      cleanups.forEach((cleanup) => cleanup());
    };
  }

  return { mount };
}
