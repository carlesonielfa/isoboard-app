/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import {
  createShortcutPresentationStrategy,
  presentShortcut,
} from '../src/lib/shortcut-presentation';

describe('shortcut presentation', () => {
  test('uses compact native-style macOS glyphs while preserving spoken names', () => {
    expect(presentShortcut('CommandOrControl+Alt+KeyZ', 'macos')).toEqual({
      display: '⌥⌘Z',
      spoken: 'Option Command Z',
    });
  });

  test('keeps modifier ordering stable and formats common keys', () => {
    expect(presentShortcut('Shift+CommandOrControl+ArrowLeft', 'macos')).toEqual({
      display: '⇧⌘←',
      spoken: 'Shift Command Arrow Left',
    });
    expect(presentShortcut('Control+Space', 'macos')).toEqual({
      display: '⌃Space',
      spoken: 'Control Space',
    });
  });

  test('matches native macOS ordering for every modifier', () => {
    expect(presentShortcut('Shift+Alt+Control+CommandOrControl+KeyS', 'macos')).toEqual({
      display: '⌃⌥⇧⌘S',
      spoken: 'Control Option Shift Command S',
    });
  });

  test('uses readable non-macOS fallback text instead of macOS glyphs', () => {
    expect(presentShortcut('CommandOrControl+Alt+KeyZ', 'generic')).toEqual({
      display: 'Cmd/Ctrl+Alt+Z',
      spoken: 'Command or Control Alt Z',
    });
  });

  test('exposes a strategy boundary for future platform presenters', () => {
    const strategy = createShortcutPresentationStrategy('linux');
    expect(strategy.present('CommandOrControl+Shift+Digit2')).toEqual({
      display: 'Cmd/Ctrl+Shift+2',
      spoken: 'Command or Control Shift 2',
    });
  });
});
