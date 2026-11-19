import { productConfig } from '$lib/product-config';
import { isTauri } from '@tauri-apps/api/core';
import { commands, events, type LicenseSnapshot } from '$lib/generated/bindings';

type LicensingState = {
  snapshot: LicenseSnapshot | null;
  loaded: boolean;
  busy: boolean;
  error: string;
};
type Environment = {
  isDesktop?: () => boolean;
  stateFactory?: (initial: LicensingState) => LicensingState;
  commands?: Pick<
    typeof commands,
    | 'getLicenseStatus'
    | 'activateCommercialLicense'
    | 'removeCommercialLicense'
    | 'recheckCommercialLicense'
    | 'continueWithoutLicenseKey'
    | 'dismissEvaluationNotice'
  >;
  listen?: typeof events.licenseChangedEvent.listen;
  focusTarget?: EventTarget;
};

function reactive(initial: LicensingState): LicensingState {
  const state = $state(initial);
  return state;
}

export function createLicensingController(environment: Environment = {}) {
  const api = environment.commands ?? commands;
  const isDesktop = environment.isDesktop ?? isTauri;
  const state = (environment.stateFactory ?? reactive)({
    snapshot: null as LicenseSnapshot | null,
    loaded: false,
    busy: false,
    error: '',
  });
  let disposed = false;
  let unlisten: (() => void) | undefined;
  let refreshing = false;
  let refreshQueued = false;
  let revision = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;

  function schedule() {
    clearTimeout(timer);
    if (disposed) return;
    const end = state.snapshot?.evaluation === 'active' ? state.snapshot.expiresAt : null;
    const untilExpiry = end
      ? end * 1000 - Date.now()
      : productConfig.licensing.statusRefreshMilliseconds;
    timer = setTimeout(
      () => void refresh(),
      Math.min(productConfig.licensing.statusRefreshMilliseconds, Math.max(1000, untilExpiry)),
    );
  }

  async function refresh() {
    if (disposed || state.busy || !isDesktop()) return;
    if (refreshing) {
      refreshQueued = true;
      return;
    }
    refreshing = true;
    const requestRevision = revision;
    try {
      const snapshot = await api.getLicenseStatus();
      if (!disposed && requestRevision === revision) {
        state.snapshot = snapshot;
        state.error = '';
      }
    } catch (error) {
      if (!disposed && requestRevision === revision) state.error = String(error);
    } finally {
      state.loaded = true;
      refreshing = false;
      if (refreshQueued && !disposed) {
        refreshQueued = false;
        void refresh();
      } else schedule();
    }
  }

  async function act(action: () => Promise<LicenseSnapshot>): Promise<boolean> {
    if (state.busy || !isDesktop()) return false;
    state.busy = true;
    revision += 1;
    try {
      const snapshot = await action();
      if (disposed) return false;
      state.snapshot = snapshot;
      state.error = '';
      return true;
    } catch (error) {
      state.error = String(error);
      return false;
    } finally {
      state.busy = false;
      schedule();
    }
  }

  function mount() {
    if (!isDesktop()) state.loaded = true;
    else {
      void (environment.listen ?? events.licenseChangedEvent.listen)(() => void refresh())
        .then((cleanup) => {
          if (disposed) cleanup();
          else unlisten = cleanup;
        })
        .catch(() => {
          /* The periodic status refresh also observes background changes. */
        });
      void refresh();
    }
    const target = environment.focusTarget ?? window;
    target.addEventListener('focus', refresh);
    return () => {
      disposed = true;
      unlisten?.();
      clearTimeout(timer);
      target.removeEventListener('focus', refresh);
    };
  }

  return {
    state,
    mount,
    refresh,
    activate: (key: string) => act(() => api.activateCommercialLicense(key)),
    remove: () => act(api.removeCommercialLicense),
    recheck: () => act(api.recheckCommercialLicense),
    continueWithoutKey: () => act(api.continueWithoutLicenseKey),
    dismissNotice: () => act(api.dismissEvaluationNotice),
  };
}
export type LicensingController = ReturnType<typeof createLicensingController>;
