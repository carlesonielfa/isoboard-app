import { isTauri } from '@tauri-apps/api/core';
import { commands, events, type ChangelogEntry, type UpdateStatus } from '$lib/generated/bindings';

export type UpdatesState = {
  status: UpdateStatus;
  error: string;
  changelog: ChangelogEntry[];
};

type Environment = {
  isDesktop?: () => boolean;
  commands?: Pick<
    typeof commands,
    'getUpdateStatus' | 'getChangelog' | 'checkForUpdates' | 'installUpdate'
  >;
  listen?: typeof events.updateChanged.listen;
};

/** Mirrors the Rust-owned update status and forwards update actions. */
export function createUpdatesController(environment: Environment = {}) {
  const api = environment.commands ?? commands;
  const isDesktop = environment.isDesktop ?? isTauri;
  const state = $state<UpdatesState>({ status: { status: 'idle' }, error: '', changelog: [] });
  let disposed = false;

  /** The changelog gains newer versions when a check finds an update. */
  function refreshChangelog(): void {
    void api
      .getChangelog()
      .then((changelog) => {
        if (!disposed) state.changelog = changelog;
      })
      .catch(() => undefined);
  }

  async function check(): Promise<void> {
    if (disposed || !isDesktop()) return;
    state.error = '';
    try {
      const status = await api.checkForUpdates();
      if (!disposed) state.status = status;
    } catch (error) {
      if (!disposed) state.error = String(error);
    }
  }

  async function install(): Promise<void> {
    if (disposed || !isDesktop()) return;
    state.error = '';
    try {
      await api.installUpdate();
    } catch (error) {
      if (!disposed) state.error = String(error);
    }
  }

  function mount(): () => void {
    if (!isDesktop()) return () => (disposed = true);
    let unlisten: (() => void) | undefined;
    void (environment.listen ?? events.updateChanged.listen)((event) => {
      state.status = event.payload;
      refreshChangelog();
    })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlisten = cleanup;
      })
      .catch(() => undefined);
    void api
      .getUpdateStatus()
      .then((status) => {
        if (!disposed) state.status = status;
      })
      .catch(() => undefined);
    refreshChangelog();
    return () => {
      disposed = true;
      unlisten?.();
    };
  }

  return { state, mount, check, install };
}

export type UpdatesController = ReturnType<typeof createUpdatesController>;
