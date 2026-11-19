/**
 * Presentation is deliberately separate from the canonical accelerator syntax
 * used by the desktop shortcut runtime. This keeps platform-specific glyphs a
 * UI concern and leaves room for native-looking Windows and Linux presenters.
 */
export type ShortcutPresentationPlatform = 'macos' | 'windows' | 'linux' | 'generic';

export type ShortcutPresentation = {
  display: string;
  spoken: string;
};

export interface ShortcutPresentationStrategy {
  present(shortcut: string): ShortcutPresentation;
}

type KeyPresentation = {
  display: string;
  spoken: string;
};

const modifierPresentation: Record<string, { mac: KeyPresentation; generic: KeyPresentation }> = {
  CommandOrControl: {
    mac: { display: '⌘', spoken: 'Command' },
    generic: { display: 'Cmd/Ctrl', spoken: 'Command or Control' },
  },
  Control: {
    mac: { display: '⌃', spoken: 'Control' },
    generic: { display: 'Ctrl', spoken: 'Control' },
  },
  Alt: {
    mac: { display: '⌥', spoken: 'Option' },
    generic: { display: 'Alt', spoken: 'Alt' },
  },
  Shift: {
    mac: { display: '⇧', spoken: 'Shift' },
    generic: { display: 'Shift', spoken: 'Shift' },
  },
};

const keyPresentation: Record<string, KeyPresentation> = {
  Space: { display: 'Space', spoken: 'Space' },
  Enter: { display: '↩', spoken: 'Enter' },
  Return: { display: '↩', spoken: 'Return' },
  Escape: { display: '⎋', spoken: 'Escape' },
  Esc: { display: '⎋', spoken: 'Escape' },
  Backspace: { display: '⌫', spoken: 'Backspace' },
  Delete: { display: '⌦', spoken: 'Delete' },
  Tab: { display: '⇥', spoken: 'Tab' },
  CapsLock: { display: '⇪', spoken: 'Caps Lock' },
  ArrowUp: { display: '↑', spoken: 'Arrow Up' },
  ArrowDown: { display: '↓', spoken: 'Arrow Down' },
  ArrowLeft: { display: '←', spoken: 'Arrow Left' },
  ArrowRight: { display: '→', spoken: 'Arrow Right' },
  PageUp: { display: '⇞', spoken: 'Page Up' },
  PageDown: { display: '⇟', spoken: 'Page Down' },
  Home: { display: '↖', spoken: 'Home' },
  End: { display: '↘', spoken: 'End' },
};

const macModifierOrder = ['Control', 'Alt', 'Shift', 'CommandOrControl'];
const genericModifierOrder = ['CommandOrControl', 'Control', 'Alt', 'Shift'];

function keyForToken(token: string): KeyPresentation {
  const known = keyPresentation[token];
  if (known) return known;

  const letterMatch = /^Key([A-Z])$/.exec(token);
  if (letterMatch) return { display: letterMatch[1], spoken: letterMatch[1] };

  const digitMatch = /^Digit([0-9])$/.exec(token);
  if (digitMatch) return { display: digitMatch[1], spoken: digitMatch[1] };

  const functionMatch = /^F([1-9][0-9]?)$/.exec(token);
  if (functionMatch) return { display: token, spoken: token };

  return { display: token, spoken: token };
}

function normalizeShortcutTokens(shortcut: string): string[] {
  return shortcut
    .split('+')
    .map((token) => token.trim())
    .filter(Boolean);
}

function formatShortcut(
  shortcut: string,
  platform: ShortcutPresentationPlatform,
): ShortcutPresentation {
  const tokens = normalizeShortcutTokens(shortcut);
  if (tokens.length === 0) return { display: '', spoken: '' };

  const isMac = platform === 'macos';
  const modifierOrder = isMac ? macModifierOrder : genericModifierOrder;
  const modifiers = modifierOrder.filter((token) => tokens.includes(token));
  const keys = tokens.filter((token) => !modifierOrder.includes(token));
  const orderedTokens = [...modifiers, ...keys];
  const displayParts: string[] = [];
  const spokenParts: string[] = [];

  for (const token of orderedTokens) {
    const modifier = modifierPresentation[token];
    const presentation = modifier ? (isMac ? modifier.mac : modifier.generic) : keyForToken(token);
    displayParts.push(presentation.display);
    spokenParts.push(presentation.spoken);
  }

  return {
    display: isMac ? displayParts.join('') : displayParts.join('+'),
    spoken: spokenParts.join(' '),
  };
}

class PlatformShortcutPresentation implements ShortcutPresentationStrategy {
  constructor(private readonly platform: ShortcutPresentationPlatform) {}

  present(shortcut: string): ShortcutPresentation {
    return formatShortcut(shortcut, this.platform);
  }
}

export function createShortcutPresentationStrategy(
  platform: ShortcutPresentationPlatform = 'generic',
): ShortcutPresentationStrategy {
  return new PlatformShortcutPresentation(platform);
}

export function presentShortcut(
  shortcut: string,
  platform: ShortcutPresentationPlatform = 'generic',
): ShortcutPresentation {
  return formatShortcut(shortcut, platform);
}
