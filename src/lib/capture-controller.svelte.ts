import { isTauri } from '@tauri-apps/api/core';
import {
  applyCaptureStatus,
  shouldApplyCaptureStatus,
  type CaptureStatus,
} from '$lib/capture-status';
import { errorIdentity, errorMessage, type AppErrorStore } from '$lib/app-error';
import {
  hideMainWindowForCapture,
  restoreMainWindowAfterCaptureFailure,
} from '$lib/window-presentation';
import { commands, events } from '$lib/ipc';
import type { CaptureViewState, DesktopAvailability } from '$lib/page-types';

type CaptureCommand = 'region' | 'file';
type StateFactory = <T>(value: T) => T;
type CaptureEnvironment = {
  isTauri: () => boolean;
  windowPresentation: { hide: () => Promise<void>; restore: () => Promise<void> };
  commands: Pick<
    typeof commands,
    'startRegionCaptureCommand' | 'openImageCommand' | 'getCaptureSnapshot'
  >;
  events: { captureStatus: Pick<typeof events.captureStatus, 'listen'> };
};

const defaultEnvironment: CaptureEnvironment = {
  isTauri,
  windowPresentation: {
    hide: hideMainWindowForCapture,
    restore: restoreMainWindowAfterCaptureFailure,
  },
  commands,
  events,
};

function createReactiveState<T>(initialState: T): T {
  const state = $state(initialState);
  return state;
}

export function createCaptureController(
  availability: DesktopAvailability,
  errors?: AppErrorStore,
  stateFactory: StateFactory = createReactiveState,
  environment: CaptureEnvironment = defaultEnvironment,
) {
  const state = stateFactory<CaptureViewState>({
    busy: false,
    activeSource: null,
    phase: 'idle',
    errorMessage: null,
    noticeMessage: null,
    preview: null,
    htmlResult: null,
    htmlResultAnnouncement: '',
    statusMessage: 'Ready to capture an image.',
  });
  let captureSequence = stateFactory(-1);
  let captureStatus = stateFactory<CaptureStatus['status'] | null>(null);
  let captureRevision = 0;
  let localErrorRevision = 0;

  function reportLocalError(message: string): void {
    errors?.show(message, errorIdentity('capture', ++localErrorRevision));
  }

  function handleCaptureStatus(payload: CaptureStatus): void {
    if (
      !shouldApplyCaptureStatus(payload, {
        sequence: captureSequence,
        status: captureStatus,
        revision: captureRevision,
      })
    ) {
      return;
    }
    captureSequence = payload.sequence;
    captureStatus = payload.status;
    captureRevision = 'revision' in payload ? payload.revision : 0;

    const error = applyCaptureStatus(state, payload);
    if (error) errors?.show(error, errorIdentity('capture', payload.sequence));
  }

  async function startCapture(
    command: CaptureCommand,
    { hideWindow = false }: { hideWindow?: boolean } = {},
  ): Promise<void> {
    if (!availability.desktopAvailable || state.busy) return;

    state.busy = true;
    state.activeSource = command === 'region' ? 'region' : 'file';
    state.phase = 'selecting';
    state.errorMessage = null;
    state.noticeMessage = null;
    state.statusMessage = `Starting ${state.activeSource === 'region' ? 'screen region' : 'image file'}…`;

    let hidden = false;
    try {
      if (hideWindow) {
        await environment.windowPresentation.hide();
        hidden = true;
      }
      await (command === 'region'
        ? environment.commands.startRegionCaptureCommand()
        : environment.commands.openImageCommand());
    } catch (error) {
      const commandError = error instanceof Error ? error.message : String(error);
      let captureInProgress = false;
      try {
        const snapshot = await environment.commands.getCaptureSnapshot();
        handleCaptureStatus(snapshot);
        captureInProgress = [
          'started',
          'imageReady',
          'processing',
          'downloadingModel',
          'loadingModel',
        ].includes(snapshot.status);
        if (!captureInProgress) {
          state.busy = false;
          state.activeSource = null;
          state.errorMessage = commandError;
          state.phase = 'failed';
          reportLocalError(commandError);
          state.statusMessage = 'Ready to capture an image.';
        }
      } catch {
        state.busy = false;
        state.activeSource = null;
        state.errorMessage = `Could not confirm capture state: ${commandError}`;
        state.phase = 'failed';
        reportLocalError(state.errorMessage);
      }
      if (hidden && !captureInProgress) {
        try {
          await environment.windowPresentation.restore();
        } catch (restoreError) {
          state.errorMessage = `${state.errorMessage ?? commandError} Could not reopen isoboard: ${errorMessage(restoreError)}`;
          reportLocalError(state.errorMessage);
        }
      }
    }
  }

  function mount(): () => void {
    let disposed = false;

    if (!environment.isTauri()) {
      availability.desktopAvailable = false;
      state.phase = 'failed';
      state.statusMessage = 'Open the isoboard desktop app to capture an image.';
      return () => {
        disposed = true;
      };
    }

    let unlisten: (() => void) | undefined;

    async function initializeCaptureStatus(): Promise<void> {
      const cleanup = await environment.events.captureStatus.listen((event) => {
        if (!disposed) handleCaptureStatus(event.payload);
      });

      if (disposed) {
        cleanup();
        return;
      }
      unlisten = cleanup;

      try {
        const snapshot = await environment.commands.getCaptureSnapshot();
        if (!disposed) handleCaptureStatus(snapshot);
      } catch (error) {
        if (!disposed) {
          state.errorMessage = errorMessage(error);
          state.phase = 'failed';
          reportLocalError(state.errorMessage);
          state.statusMessage = 'Ready to capture an image.';
        }
      }
    }

    initializeCaptureStatus().catch((error) => {
      if (!disposed) {
        availability.desktopAvailable = false;
        state.errorMessage = errorMessage(error);
        state.phase = 'failed';
        reportLocalError(state.errorMessage);
        state.statusMessage = 'Open the isoboard desktop app to capture an image.';
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }

  return { state, startCapture, mount };
}
