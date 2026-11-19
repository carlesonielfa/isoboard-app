import type { CaptureBindingSnapshot, ProcessingProfileSnapshot } from '$lib/generated/bindings';

export function shortcutBindingTargetLabel(
  binding: CaptureBindingSnapshot,
  profiles: ProcessingProfileSnapshot[],
): string {
  if (binding.profileId === null) return 'Active profile';
  return profiles.find((profile) => profile.id === binding.profileId)?.name ?? binding.profileId;
}

export function canDeleteShortcutBinding(binding: CaptureBindingSnapshot): boolean {
  return binding.profileId !== null;
}

export function selectedShortcutBinding(
  bindings: CaptureBindingSnapshot[],
  bindingId: string,
): CaptureBindingSnapshot | undefined {
  return bindings.find((binding) => binding.id === bindingId);
}

export function resolveNewBindingProfileId(
  currentProfileId: string,
  profiles: ProcessingProfileSnapshot[],
  activeProfileId: string,
  initialized: boolean,
): string {
  if (initialized && profiles.some((profile) => profile.id === currentProfileId)) {
    return currentProfileId;
  }
  return (
    profiles.find((profile) => profile.id === activeProfileId)?.id ?? profiles[0]?.id ?? 'default'
  );
}

export function draftShortcutAfterSave(
  currentShortcut: string,
  targetKind: 'binding' | 'create' | 'open',
  recordedShortcut: string,
  saved: boolean,
): string {
  if (targetKind !== 'create') return currentShortcut;
  return saved ? '' : recordedShortcut;
}
