<script lang="ts">
  import type { DesktopAvailability, ShortcutViewState } from '$lib/page-types';
  import type { ConfigSnapshot } from '$lib/generated/bindings';
  import CaptureBindingDraftRow from '$lib/components/CaptureBindingDraftRow.svelte';
  import CaptureBindingRow from '$lib/components/CaptureBindingRow.svelte';
  import ShortcutRecorder from '$lib/components/ShortcutRecorder.svelte';
  import InfoLink from '$lib/components/InfoLink.svelte';
  import { docsLinks } from '$lib/docs-links';

  let {
    state,
    config,
    configLoaded,
    availability,
    onStartBindingRecording,
    onStartCreateBindingRecording,
    onStartOpenShortcutRecording,
    onRecordShortcut,
    onCancelRecording,
    onDeleteBinding,
    onSelectNewBindingProfile,
    onSelectBindingProfile,
  }: {
    state: ShortcutViewState;
    config: ConfigSnapshot;
    configLoaded: boolean;
    availability: DesktopAvailability;
    onStartBindingRecording: (bindingId: string) => void;
    onStartCreateBindingRecording: (profileId: string) => void;
    onStartOpenShortcutRecording: () => void;
    onRecordShortcut: (shortcut: string) => void;
    onCancelRecording: () => void;
    onDeleteBinding: (bindingId: string) => Promise<boolean>;
    onSelectNewBindingProfile: (profileId: string) => void;
    onSelectBindingProfile: (bindingId: string, profileId: string) => Promise<boolean>;
  } = $props();

  const busy = $derived(!availability.desktopAvailable || state.saving);

  function isBindingRecording(id: string): boolean {
    return (
      state.recordingTarget?.kind === 'binding' &&
      state.recordingTarget.bindingId === id &&
      state.recording
    );
  }

  function isBindingPending(id: string): boolean {
    return (
      state.recordingTarget?.kind === 'binding' &&
      state.recordingTarget.bindingId === id &&
      state.recordingPending
    );
  }

  function bindingDisabled(id: string): boolean {
    return (
      busy ||
      ((state.recording || state.recordingPending) &&
        !(state.recordingTarget?.kind === 'binding' && state.recordingTarget.bindingId === id))
    );
  }

  const canCreate = $derived(configLoaded && config.processing.profiles.length > 0);
  const defaultBinding = $derived(
    config.captureBindings.find((binding) => binding.profileId === null),
  );
  const profileBindings = $derived(
    config.captureBindings.filter((binding) => binding.profileId !== null),
  );
</script>

<div class="flex flex-col gap-4">
  <div class="ml-4 flex flex-col gap-2">
    <h3 id="capture-heading" class="m-0 text-sm font-semibold">Capture</h3>

    {#if configLoaded}
      {#if defaultBinding}
        <ul
          class="m-0 ml-4 flex min-w-0 list-none flex-col gap-2 p-0"
          aria-label="Capture region shortcut"
        >
          <CaptureBindingRow
            binding={defaultBinding}
            profiles={config.processing.profiles}
            recording={isBindingRecording(defaultBinding.id)}
            recordingPending={isBindingPending(defaultBinding.id)}
            disabled={bindingDisabled(defaultBinding.id)}
            onStart={() => onStartBindingRecording(defaultBinding.id)}
            onRecord={onRecordShortcut}
            onCancel={onCancelRecording}
            onDelete={() => void onDeleteBinding(defaultBinding.id)}
            onSelectProfile={(profileId) => onSelectBindingProfile(defaultBinding.id, profileId)}
          />
        </ul>
      {/if}
      {#if profileBindings.length > 0 || config.processing.profiles.length > 1}
        <h4 class="m-0 mt-2 ml-4 flex items-center gap-1 text-sm font-semibold">
          Profile shortcuts
          <InfoLink {...docsLinks.profileShortcuts} />
        </h4>
        <ul
          class="m-0 ml-4 flex min-w-0 list-none flex-col gap-2 p-0"
          aria-label="Profile capture shortcuts"
        >
          {#each profileBindings as binding (binding.id)}
            <CaptureBindingRow
              {binding}
              profiles={config.processing.profiles}
              recording={isBindingRecording(binding.id)}
              recordingPending={isBindingPending(binding.id)}
              disabled={bindingDisabled(binding.id)}
              onStart={() => onStartBindingRecording(binding.id)}
              onRecord={onRecordShortcut}
              onCancel={onCancelRecording}
              onDelete={() => void onDeleteBinding(binding.id)}
              onSelectProfile={(profileId) => onSelectBindingProfile(binding.id, profileId)}
            />
          {/each}
          {#if config.processing.profiles.length > 1}
            <CaptureBindingDraftRow
              profiles={config.processing.profiles}
              profileId={state.newBindingProfileId}
              shortcut={state.newBindingShortcut}
              recording={state.recordingTarget?.kind === 'create' && state.recording}
              recordingPending={state.recordingTarget?.kind === 'create' && state.recordingPending}
              disabled={busy || !canCreate || Boolean(state.recordingTarget?.kind === 'binding')}
              onStart={() => onStartCreateBindingRecording(state.newBindingProfileId)}
              onRecord={onRecordShortcut}
              onCancel={onCancelRecording}
              onSelectProfile={onSelectNewBindingProfile}
            />
          {/if}
        </ul>
      {/if}
    {/if}
  </div>
  <div class="ml-4 flex flex-col gap-2">
    <h3 class="m-0 text-sm font-semibold">Window</h3>
    {#if configLoaded}
      <div
        class="grid ml-4 mr-12 min-w-0 grid-cols-[minmax(12rem,max-content)_minmax(0,1fr)] items-center gap-2"
      >
        <span class="min-w-0 truncate px-2 text-sm">Open isoboard window</span>
        <ShortcutRecorder
          label="Shortcut for opening the isoboard window"
          labelHidden
          id="open-isoboard-shortcut"
          value={config.openShortcut}
          recording={state.recordingTarget?.kind === 'open' && state.recording}
          recordingPending={state.recordingTarget?.kind === 'open' && state.recordingPending}
          disabled={busy ||
            (state.recordingTarget?.kind !== 'open' && (state.recording || state.recordingPending))}
          onStart={onStartOpenShortcutRecording}
          onRecord={onRecordShortcut}
          onCancel={onCancelRecording}
        />
      </div>
    {/if}
  </div>
</div>
