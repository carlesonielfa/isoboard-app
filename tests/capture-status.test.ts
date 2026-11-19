/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import {
  applyCaptureStatus,
  shouldApplyCaptureStatus,
  type CaptureStatus,
  type CaptureStatusState,
} from '../src/lib/capture-status';
import type { CaptureViewState } from '../src/lib/page-types';

function state(sequence: number, status: CaptureStatusState['status']): CaptureStatusState {
  return { sequence, status };
}

describe('shouldApplyCaptureStatus', () => {
  test('accepts the terminal status for the same capture sequence', () => {
    expect(
      shouldApplyCaptureStatus({ sequence: 7, status: 'imageReady' }, state(7, 'started')),
    ).toBe(true);
    expect(
      shouldApplyCaptureStatus({ sequence: 7, status: 'processing' }, state(7, 'imageReady')),
    ).toBe(true);
    expect(
      shouldApplyCaptureStatus({ sequence: 7, status: 'completed' }, state(7, 'processing')),
    ).toBe(true);
    expect(
      shouldApplyCaptureStatus({ sequence: 7, status: 'cancelled' }, state(7, 'started')),
    ).toBe(true);
    expect(shouldApplyCaptureStatus({ sequence: 7, status: 'failed' }, state(7, 'started'))).toBe(
      true,
    );
  });

  test('rejects older events and a delayed Started event after completion', () => {
    expect(
      shouldApplyCaptureStatus({ sequence: 6, status: 'imageReady' }, state(7, 'started')),
    ).toBe(false);
    expect(
      shouldApplyCaptureStatus({ sequence: 7, status: 'started' }, state(7, 'imageReady')),
    ).toBe(false);
    expect(shouldApplyCaptureStatus({ sequence: 7, status: 'idle' }, state(7, 'started'))).toBe(
      false,
    );
  });

  test('accepts a first snapshot, including the idle startup snapshot', () => {
    expect(shouldApplyCaptureStatus({ sequence: 0, status: 'idle' }, state(-1, null))).toBe(true);
    expect(shouldApplyCaptureStatus({ sequence: 3, status: 'started' }, state(-1, null))).toBe(
      true,
    );
  });

  test('orders model preparation and progress across multiple inference passes', () => {
    const loading = { sequence: 7, status: 'loadingModel' as const, revision: 2 };
    expect(
      shouldApplyCaptureStatus(loading, { sequence: 7, status: 'downloadingModel', revision: 1 }),
    ).toBe(true);
    expect(
      shouldApplyCaptureStatus({ sequence: 7, status: 'processing', revision: 3 }, loading),
    ).toBe(true);
    expect(
      shouldApplyCaptureStatus(
        { ...loading, revision: 4 },
        { sequence: 7, status: 'processing', revision: 3 },
      ),
    ).toBe(true);
    expect(
      shouldApplyCaptureStatus(loading, { sequence: 7, status: 'processing', revision: 3 }),
    ).toBe(false);
    expect(
      shouldApplyCaptureStatus(
        { sequence: 7, status: 'downloadingModel', revision: 5 },
        { sequence: 7, status: 'completed' },
      ),
    ).toBe(false);
    const view = captureView();
    applyCaptureStatus(view, {
      status: 'downloadingModel',
      sequence: 7,
      revision: 1,
      source: 'region',
      profileId: 'default',
      bindingId: null,
      model: 'qwen3.5-2b',
      downloadedBytes: 25,
      totalBytes: 100,
    });
    expect(view.phase).toBe('downloading');
    expect(view.downloadProgress).toBe(25);
    expect(view.busy).toBe(true);
    applyCaptureStatus(view, {
      ...loading,
      source: 'region',
      profileId: 'default',
      bindingId: null,
      model: 'qwen3.5-2b',
    });
    expect(view.phase).toBe('loading');
    expect(view.downloadProgress).toBeNull();
  });

  test('preserves the image while processing and commits only completed HTML', () => {
    const view = captureView();
    const imageReady: CaptureStatus = {
      status: 'imageReady',
      sequence: 4,
      source: 'file',
      profileId: 'default',
      bindingId: null,
      image: {
        mimeType: 'image/png',
        dataBase64: 'new-image',
        width: 640,
        height: 480,
      },
    };

    expect(applyCaptureStatus(view, imageReady)).toBeNull();
    expect(view.preview?.src).toBe('data:image/png;base64,new-image');
    expect(view.phase).toBe('processing');
    expect(view.htmlResult).toBeNull();
    expect(
      applyCaptureStatus(view, {
        status: 'processing',
        revision: 0,
        sequence: 4,
        source: 'file',
        profileId: 'default',
        bindingId: null,
      }),
    ).toBeNull();
    expect(view.htmlResult).toBeNull();

    expect(
      applyCaptureStatus(view, {
        status: 'completed',
        sequence: 4,
        source: 'file',
        profileId: 'default',
        bindingId: null,
        html: '<p>complete</p>',
      }),
    ).toBeNull();
    expect(view.htmlResult).toBe('<p>complete</p>');
    expect(view.htmlResultAnnouncement).toBe('HTML result updated.');
    expect(view.phase).toBe('idle');
  });

  test('preserves the last HTML and current image when inference fails', () => {
    const view = captureView();
    view.preview = {
      src: 'data:image/png;base64,previous',
      mimeType: 'image/png',
      width: 320,
      height: 200,
    };
    view.htmlResult = '<p>previous</p>';
    view.htmlResultAnnouncement = '';

    expect(
      applyCaptureStatus(view, {
        status: 'started',
        sequence: 5,
        source: 'region',
        profileId: 'default',
        bindingId: null,
      }),
    ).toBeNull();
    expect(
      applyCaptureStatus(view, {
        status: 'imageReady',
        sequence: 5,
        source: 'region',
        profileId: 'default',
        bindingId: null,
        image: {
          mimeType: 'image/jpeg',
          dataBase64: 'current',
          width: 800,
          height: 600,
        },
      }),
    ).toBeNull();

    const message = applyCaptureStatus(view, {
      status: 'failed',
      sequence: 5,
      source: 'region',
      profileId: 'default',
      bindingId: null,
      code: 'inference',
      message: 'Provider failed',
    });

    expect(message).toBe('Provider failed');
    expect(view.preview?.src).toBe('data:image/jpeg;base64,current');
    expect(view.htmlResult).toBe('<p>previous</p>');
    expect(view.htmlResultAnnouncement).toBe('');
    expect(view.busy).toBe(false);
    expect(view.phase).toBe('failed');
  });
});

function captureView(): CaptureViewState {
  return {
    busy: false,
    activeSource: null,
    phase: 'idle',
    errorMessage: null,
    noticeMessage: null,
    preview: null,
    htmlResult: null,
    htmlResultAnnouncement: '',
    statusMessage: 'Ready to capture an image.',
  };
}
