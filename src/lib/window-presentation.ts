import { getCurrentWindow } from '@tauri-apps/api/window';
import { isTauri } from '@tauri-apps/api/core';
import { commands } from '$lib/ipc';
import type { ShortcutPresentationPlatform } from '$lib/shortcut-presentation';

export type WindowPresentationCapabilities = {
  nativeBackdrop: boolean;
  shortcutPresentationPlatform: ShortcutPresentationPlatform;
};

const browserCapabilities: WindowPresentationCapabilities = {
  nativeBackdrop: false,
  shortcutPresentationPlatform: 'generic',
};

let capabilitiesPromise: Promise<WindowPresentationCapabilities> | undefined;

export async function getWindowPresentationCapabilities(): Promise<WindowPresentationCapabilities> {
  if (!isTauri()) return browserCapabilities;
  capabilitiesPromise ??= commands.getWindowPresentationCapabilities().catch((error) => {
    capabilitiesPromise = undefined;
    throw error;
  });
  return capabilitiesPromise;
}

export async function hideMainWindowForCapture(): Promise<void> {
  await getCurrentWindow().hide();
}

export async function getTitlebarControlsCenterY(): Promise<number | null> {
  if (!isTauri()) return null;
  return commands.getTitlebarControlsCenterY();
}

export async function restoreMainWindowAfterCaptureFailure(): Promise<void> {
  await commands.showMainWindow();
}
