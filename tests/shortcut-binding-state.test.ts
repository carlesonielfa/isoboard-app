/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import {
  canDeleteShortcutBinding,
  draftShortcutAfterSave,
  selectedShortcutBinding,
  resolveNewBindingProfileId,
  shortcutBindingTargetLabel,
} from '../src/lib/shortcut-binding-state';
import {
  recordingTargetExists,
  shouldHandleRecordingCancel,
} from '../src/lib/shortcut-controller-state';
import {
  defaultProcessingProfileDraft,
  processingProfileSnapshotFromDraft,
} from '../src/lib/processing-profile-state';

const bindings = [
  { id: 'active', shortcut: 'CommandOrControl+Shift+Digit2', profileId: null },
  { id: 'notes', shortcut: 'CommandOrControl+Shift+KeyN', profileId: 'notes' },
];

describe('capture binding state', () => {
  test('resolves the active and fixed profile labels from the current snapshot', () => {
    const profiles = [
      processingProfileSnapshotFromDraft('default'),
      processingProfileSnapshotFromDraft('notes', defaultProcessingProfileDraft('Notes')),
    ];

    expect(shortcutBindingTargetLabel(bindings[0], profiles)).toBe('Active profile');
    expect(shortcutBindingTargetLabel(bindings[1], profiles)).toBe('Notes');
  });

  test('allows deleting fixed-profile bindings while preserving the default binding', () => {
    expect(canDeleteShortcutBinding(bindings[0])).toBe(false);
    expect(canDeleteShortcutBinding(bindings[1])).toBe(true);
  });

  test('selects a binding by its stable snapshot id', () => {
    expect(selectedShortcutBinding(bindings, 'notes')).toEqual(bindings[1]);
    expect(selectedShortcutBinding(bindings, 'missing')).toBeUndefined();
  });

  test('invalidates a recording target when a snapshot removes its binding or profile', () => {
    expect(
      recordingTargetExists(
        { kind: 'binding', bindingId: 'notes' },
        [bindings[0]],
        [processingProfileSnapshotFromDraft('default')],
      ),
    ).toBe(false);
    expect(
      recordingTargetExists(
        { kind: 'create', profileId: 'notes' },
        [bindings[0]],
        [processingProfileSnapshotFromDraft('default')],
      ),
    ).toBe(false);
  });

  test('ignores duplicate blur cancellation after the active target has been cleared', () => {
    expect(shouldHandleRecordingCancel(null, false, true)).toBe(false);
    expect(shouldHandleRecordingCancel({ kind: 'binding', bindingId: 'active' }, false, true)).toBe(
      true,
    );
  });

  test('starts the empty draft on the active profile and preserves a selected profile', () => {
    const profiles = [
      processingProfileSnapshotFromDraft('default'),
      processingProfileSnapshotFromDraft('notes', defaultProcessingProfileDraft('Notes')),
    ];

    expect(resolveNewBindingProfileId('default', profiles, 'notes', false)).toBe('notes');
    expect(resolveNewBindingProfileId('default', profiles, 'notes', true)).toBe('default');
  });

  test('retargets an empty draft to the active profile when its prior profile disappears', () => {
    expect(
      resolveNewBindingProfileId(
        'missing',
        [processingProfileSnapshotFromDraft('notes', defaultProcessingProfileDraft('Notes'))],
        'notes',
        true,
      ),
    ).toBe('notes');
  });

  test('clears a new draft only after its binding saves successfully', () => {
    expect(draftShortcutAfterSave('', 'create', 'CommandOrControl+Shift+KeyN', false)).toBe(
      'CommandOrControl+Shift+KeyN',
    );
    expect(
      draftShortcutAfterSave(
        'CommandOrControl+Shift+KeyN',
        'create',
        'CommandOrControl+Shift+KeyN',
        true,
      ),
    ).toBe('');
    expect(
      draftShortcutAfterSave(
        'CommandOrControl+Shift+KeyN',
        'binding',
        'CommandOrControl+Shift+KeyR',
        false,
      ),
    ).toBe('CommandOrControl+Shift+KeyN');
  });
});
