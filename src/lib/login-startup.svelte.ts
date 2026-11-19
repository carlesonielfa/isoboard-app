import { isTauri } from '@tauri-apps/api/core';
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
import type { AppErrorStore } from '$lib/app-error';

export type LoginStartupState = {
  enabled: boolean;
  loading: boolean;
  saving: boolean;
};

export function createLoginStartupController(errors: AppErrorStore) {
  const state = $state<LoginStartupState>({ enabled: false, loading: true, saving: false });
  let disposed = false;
  let refreshing = false;

  async function refresh(): Promise<void> {
    if (disposed || !isTauri() || state.saving || refreshing) return;
    refreshing = true;
    state.loading = true;
    try {
      const enabled = await isEnabled();
      if (!disposed) state.enabled = enabled;
    } catch (error) {
      if (!disposed) errors.show(error, { source: 'login-startup' });
    } finally {
      refreshing = false;
      if (!disposed) state.loading = false;
    }
  }

  async function setEnabled(enabled: boolean): Promise<void> {
    if (disposed || !isTauri() || state.loading || state.saving || state.enabled === enabled)
      return;
    state.saving = true;
    try {
      if (enabled) await enable();
      else await disable();
      if (!disposed) state.enabled = enabled;
      const actual = await isEnabled();
      if (!disposed) state.enabled = actual;
    } catch (error) {
      if (!disposed) errors.show(error, { source: 'login-startup' });
    } finally {
      if (!disposed) state.saving = false;
    }
  }

  function mount(): () => void {
    if (!isTauri()) {
      state.loading = false;
      return () => {
        disposed = true;
      };
    }
    void refresh();
    window.addEventListener('focus', refresh);
    return () => {
      disposed = true;
      window.removeEventListener('focus', refresh);
    };
  }

  return { state, mount, setEnabled };
}
