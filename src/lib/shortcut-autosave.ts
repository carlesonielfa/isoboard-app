export type ShortcutAutosaveOperation = {
  restoreNativeShortcut: () => Promise<boolean>;
  saveShortcut: (shortcut: string) => Promise<void>;
  isCurrent: () => boolean;
};

/** Restore the active registration before persisting a recorded chord. */
export async function restoreThenSaveShortcut(
  shortcut: string,
  operation: ShortcutAutosaveOperation,
): Promise<'saved' | 'cancelled' | 'restore-failed'> {
  const restored = await operation.restoreNativeShortcut();
  if (!restored) return 'restore-failed';
  if (!operation.isCurrent()) return 'cancelled';

  await operation.saveShortcut(shortcut);
  return 'saved';
}
