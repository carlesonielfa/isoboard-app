/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import { isModifierOnlyEvent, shortcutFromKeyboardEvent } from '../src/lib/shortcut-recorder';
import { restoreThenSaveShortcut } from '../src/lib/shortcut-autosave';

function event(overrides: Partial<KeyboardEvent> = {}): KeyboardEvent {
  return {
    code: 'KeyR',
    key: 'r',
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    repeat: false,
    ...overrides,
  } as KeyboardEvent;
}

describe('shortcutFromKeyboardEvent', () => {
  test('uses the portable CommandOrControl token for the meta modifier', () => {
    expect(
      shortcutFromKeyboardEvent(event({ code: 'Digit2', key: '2', metaKey: true, shiftKey: true })),
    ).toBe('CommandOrControl+Shift+Digit2');
  });

  test('rejects the unsupported ISO keyboard code before native validation', () => {
    expect(
      shortcutFromKeyboardEvent(
        event({ code: 'IntlBackslash', key: '\\', ctrlKey: true, altKey: true }),
      ),
    ).toBe(null);
  });

  test('passes other unfamiliar nonmodifier codes through for native validation', () => {
    expect(shortcutFromKeyboardEvent(event({ code: 'Unknown', key: '/', shiftKey: true }))).toBe(
      'Shift+Unknown',
    );
  });

  test('does not fall back to the character when the code is unavailable', () => {
    expect(shortcutFromKeyboardEvent(event({ code: '', key: '/', shiftKey: true }))).toBe(null);
    expect(
      shortcutFromKeyboardEvent(event({ code: 'Unidentified', key: '/', shiftKey: true })),
    ).toBe(null);
  });

  test('does not produce a shortcut from modifiers or repeat events', () => {
    expect(isModifierOnlyEvent(event({ code: 'ShiftLeft', key: 'Shift' }))).toBe(true);
    expect(
      shortcutFromKeyboardEvent(event({ code: 'ShiftLeft', key: 'Shift', shiftKey: true })),
    ).toBe(null);
    expect(shortcutFromKeyboardEvent(event({ repeat: true }))).toBe(null);
    expect(shortcutFromKeyboardEvent(event())).toBe(null);
  });
});

describe('restoreThenSaveShortcut', () => {
  test('restores native registration before saving the recorded chord', async () => {
    const steps: string[] = [];

    const result = await restoreThenSaveShortcut('CommandOrControl+Shift+KeyR', {
      restoreNativeShortcut: async () => {
        steps.push('restore');
        return true;
      },
      isCurrent: () => true,
      saveShortcut: async (shortcut) => {
        steps.push(`save:${shortcut}`);
      },
    });

    expect(result).toBe('saved');
    expect(steps).toEqual(['restore', 'save:CommandOrControl+Shift+KeyR']);
  });

  test('does not persist when native registration cannot be restored', async () => {
    let saved = false;

    const result = await restoreThenSaveShortcut('CommandOrControl+Shift+KeyR', {
      restoreNativeShortcut: async () => false,
      isCurrent: () => true,
      saveShortcut: async () => {
        saved = true;
      },
    });

    expect(result).toBe('restore-failed');
    expect(saved).toBe(false);
  });

  test('does not persist a completion invalidated by cancellation', async () => {
    let current = true;
    let saved = false;

    const result = await restoreThenSaveShortcut('CommandOrControl+Shift+KeyR', {
      restoreNativeShortcut: async () => {
        current = false;
        return true;
      },
      isCurrent: () => current,
      saveShortcut: async () => {
        saved = true;
      },
    });

    expect(result).toBe('cancelled');
    expect(saved).toBe(false);
  });
});
