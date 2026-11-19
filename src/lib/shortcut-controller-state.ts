import type { CaptureBindingSnapshot, ProcessingProfileSnapshot } from '$lib/generated/bindings';
import type { ShortcutRecordingTarget } from '$lib/page-types';

export function recordingTargetExists(
  target: ShortcutRecordingTarget,
  bindings: CaptureBindingSnapshot[],
  profiles: ProcessingProfileSnapshot[],
): boolean {
  if (target.kind === 'binding') {
    return bindings.some((binding) => binding.id === target.bindingId);
  }
  if (target.kind === 'open') return true;
  return profiles.some((profile) => profile.id === target.profileId);
}

/** Shared blur handlers may call cancel while a restore is already in flight. */
export function shouldHandleRecordingCancel(
  target: ShortcutRecordingTarget | null,
  recording: boolean,
  recordingPending: boolean,
): boolean {
  return Boolean(target && (recording || recordingPending));
}
