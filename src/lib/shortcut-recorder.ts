/**
 * The desktop shortcut runtime uses the same code names as the browser
 * KeyboardEvent.code property. Keep the event shape here so the recorder and
 * its tests do not depend on a browser DOM implementation.
 */
export type ShortcutKeyboardEvent = {
  code: string;
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  repeat: boolean;
};

const modifierCodes = new Set([
  'AltLeft',
  'AltRight',
  'ControlLeft',
  'ControlRight',
  'MetaLeft',
  'MetaRight',
  'OSLeft',
  'OSRight',
  'ShiftLeft',
  'ShiftRight',
]);

function codeForEvent(event: ShortcutKeyboardEvent): string | null {
  if (!event.code || event.code === 'Unidentified') return null;
  // global-hotkey/muda do not recognize the ISO-only browser code.
  if (event.code === 'IntlBackslash') return null;
  return event.code;
}

export function isModifierOnlyEvent(event: ShortcutKeyboardEvent): boolean {
  return (
    modifierCodes.has(event.code) || ['Alt', 'Control', 'Meta', 'OS', 'Shift'].includes(event.key)
  );
}

/** Convert one keydown into the accelerator syntax accepted by global-hotkey. */
export function shortcutFromKeyboardEvent(event: ShortcutKeyboardEvent): string | null {
  if (event.repeat || isModifierOnlyEvent(event)) return null;

  const key = codeForEvent(event);
  if (!key) return null;

  const modifiers: string[] = [];
  if (event.metaKey) modifiers.push('CommandOrControl');
  if (event.ctrlKey) modifiers.push('Control');
  if (event.altKey) modifiers.push('Alt');
  if (event.shiftKey) modifiers.push('Shift');
  if (modifiers.length === 0) return null;
  return [...modifiers, key].join('+');
}
