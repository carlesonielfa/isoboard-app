import type { CaptureViewState } from '$lib/page-types';
import type { CaptureSource, CaptureStatus } from '$lib/generated/bindings';

export type { CaptureSource, CaptureStatus } from '$lib/generated/bindings';

export type CaptureStatusState = {
  sequence: number;
  status: CaptureStatus['status'] | null;
  revision?: number;
};

const sourceLabels: Record<CaptureSource, string> = {
  region: 'screen region',
  file: 'image file',
};

function sourceLabel(source: CaptureSource): string {
  return sourceLabels[source];
}

/**
 * Apply an accepted native status to the capture view state.
 *
 * The image is committed at imageReady and the HTML is committed only at
 * completed. Failed and cancelled runs deliberately leave both values alone.
 * The returned message is shown through the shared app error toast by the
 * controller.
 */
export function applyCaptureStatus(state: CaptureViewState, payload: CaptureStatus): string | null {
  state.downloadProgress =
    payload.status === 'downloadingModel'
      ? Math.min(100, Math.floor((100 * payload.downloadedBytes) / Math.max(1, payload.totalBytes)))
      : null;
  if (payload.status === 'idle') {
    state.busy = false;
    state.activeSource = null;
    state.phase = 'idle';
    state.htmlResultAnnouncement = '';
    state.statusMessage = 'Ready to capture an image.';
    return null;
  }

  if (payload.status === 'started') {
    state.busy = true;
    state.activeSource = payload.source;
    state.phase = 'selecting';
    state.errorMessage = null;
    state.noticeMessage = null;
    state.htmlResultAnnouncement = '';
    state.statusMessage = `Waiting for ${sourceLabel(payload.source)}…`;
    return null;
  }

  if (payload.status === 'imageReady') {
    const mimeType =
      payload.image.mimeType === 'image/png' || payload.image.mimeType === 'image/jpeg'
        ? payload.image.mimeType
        : null;
    const validImage =
      payload.image.dataBase64.length > 0 &&
      mimeType !== null &&
      payload.image.width > 0 &&
      payload.image.height > 0;
    if (!validImage) {
      const message = 'The image could not be displayed.';
      state.busy = false;
      state.activeSource = null;
      state.phase = 'failed';
      state.errorMessage = message;
      state.noticeMessage = null;
      state.statusMessage = 'Ready to capture an image.';
      return message;
    }

    state.busy = true;
    state.activeSource = payload.source;
    state.phase = 'processing';
    state.errorMessage = null;
    state.noticeMessage = null;
    state.preview = {
      src: `data:${payload.image.mimeType};base64,${payload.image.dataBase64}`,
      mimeType,
      width: payload.image.width,
      height: payload.image.height,
    };
    state.statusMessage = `Processing ${sourceLabel(payload.source)}…`;
    return null;
  }

  if (payload.status === 'loadingModel' || payload.status === 'downloadingModel') {
    state.busy = true;
    state.activeSource = payload.source;
    state.phase = payload.status === 'loadingModel' ? 'loading' : 'downloading';
    state.errorMessage = null;
    state.noticeMessage = null;
    state.statusMessage =
      payload.status === 'loadingModel'
        ? `Loading model ${payload.model}…`
        : `Downloading model ${payload.model} · ${state.downloadProgress}%`;
    return null;
  }

  if (payload.status === 'processing') {
    state.busy = true;
    state.activeSource = payload.source;
    state.phase = 'processing';
    state.errorMessage = null;
    state.noticeMessage = null;
    state.statusMessage = `Processing ${sourceLabel(payload.source)}…`;
    return null;
  }

  state.busy = false;
  state.activeSource = null;
  state.phase = payload.status === 'failed' ? 'failed' : 'idle';

  if (payload.status === 'completed') {
    state.errorMessage = null;
    state.noticeMessage = null;
    state.htmlResult = payload.html;
    state.htmlResultAnnouncement = 'HTML result updated.';
    state.statusMessage = `${sourceLabel(payload.source)} complete.`;
    return null;
  }

  if (payload.status === 'cancelled') {
    state.errorMessage = null;
    state.noticeMessage = `${sourceLabel(payload.source)} capture cancelled.`;
    state.statusMessage = 'Ready to capture an image.';
    return null;
  }

  const message = payload.message || `Could not open the ${sourceLabel(payload.source)}.`;
  state.errorMessage = message;
  state.noticeMessage = null;
  state.statusMessage = 'Ready to capture an image.';
  return message;
}

/**
 * Accept status transitions for one capture while rejecting stale events.
 * Every status emitted for one capture intentionally shares one sequence number.
 */
export function shouldApplyCaptureStatus(
  incoming: Pick<CaptureStatus, 'sequence' | 'status'> & { revision?: number },
  current: CaptureStatusState,
): boolean {
  if (incoming.sequence < current.sequence) return false;
  if (incoming.sequence > current.sequence) return true;

  if (current.status === null) return true;

  if (incoming.status === 'idle') return false;
  if (current.status === 'idle') return true;

  // Capture events are monotonic within a sequence. This allows the native
  // event stream and the initial snapshot to arrive in either order without
  // allowing a delayed event to roll the UI backwards.
  const rank: Record<Exclude<CaptureStatus['status'], 'idle'>, number> = {
    started: 0,
    imageReady: 1,
    downloadingModel: 2,
    loadingModel: 2,
    processing: 2,
    completed: 3,
    cancelled: 3,
    failed: 3,
  };
  if (rank[incoming.status] === 2 && rank[current.status] === 2)
    return (incoming.revision ?? 0) > (current.revision ?? 0);
  return rank[incoming.status] > rank[current.status];
}
