<script lang="ts">
  import type { ProcessingProfileSnapshot } from '$lib/generated/bindings';
  import CaptureBindingProfileSelect from '$lib/components/CaptureBindingProfileSelect.svelte';
  import ShortcutRecorder from '$lib/components/ShortcutRecorder.svelte';

  let {
    profiles,
    profileId,
    shortcut,
    recording,
    recordingPending,
    disabled,
    onStart,
    onRecord,
    onCancel,
    onSelectProfile,
  }: {
    profiles: ProcessingProfileSnapshot[];
    profileId: string;
    shortcut: string;
    recording: boolean;
    recordingPending: boolean;
    disabled: boolean;
    onStart: () => void;
    onRecord: (shortcut: string) => void;
    onCancel: () => void;
    onSelectProfile: (profileId: string) => void;
  } = $props();

  const profileName = $derived(
    profiles.find((profile) => profile.id === profileId)?.name ?? profileId,
  );
  const rowDisabled = $derived(disabled || recording || recordingPending);
</script>

<li class="flex min-w-0 flex-col gap-2">
  <div
    class="grid min-w-0 grid-cols-[minmax(12rem,max-content)_minmax(0,1fr)_auto] items-center gap-2"
  >
    <CaptureBindingProfileSelect
      label="Profile for new capture shortcut"
      {profileId}
      {profiles}
      disabled={rowDisabled}
      onSelect={onSelectProfile}
    />

    <ShortcutRecorder
      label={`Shortcut to capture a region with ${profileName}`}
      labelHidden
      id="new-capture-binding"
      value={shortcut}
      {recording}
      {recordingPending}
      {disabled}
      {onStart}
      {onRecord}
      {onCancel}
    />

    <span aria-hidden="true" class="size-10 shrink-0"></span>
  </div>
</li>
