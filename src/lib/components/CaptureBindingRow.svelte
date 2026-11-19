<script lang="ts">
  import type { CaptureBindingSnapshot, ProcessingProfileSnapshot } from '$lib/generated/bindings';
  import Button from '$lib/components/Button.svelte';
  import CaptureBindingProfileSelect from '$lib/components/CaptureBindingProfileSelect.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import ShortcutRecorder from '$lib/components/ShortcutRecorder.svelte';

  let {
    binding,
    profiles,
    recording,
    recordingPending,
    disabled,
    onStart,
    onRecord,
    onCancel,
    onDelete,
    onSelectProfile,
  }: {
    binding: CaptureBindingSnapshot;
    profiles: ProcessingProfileSnapshot[];
    recording: boolean;
    recordingPending: boolean;
    disabled: boolean;
    onStart: () => void;
    onRecord: (shortcut: string) => void;
    onCancel: () => void;
    onDelete: () => void;
    onSelectProfile: (profileId: string) => Promise<boolean> | void;
  } = $props();

  const profile = $derived(
    binding.profileId === null
      ? undefined
      : profiles.find((candidate) => candidate.id === binding.profileId),
  );
  const profileName = $derived(profile?.name ?? binding.profileId ?? '');

  const rowDisabled = $derived(disabled || recording || recordingPending);
</script>

<li class="flex min-w-0 flex-col gap-2">
  <div
    class="grid min-w-0 grid-cols-[minmax(12rem,max-content)_minmax(0,1fr)_auto] items-center gap-2"
  >
    {#if binding.profileId === null}
      <span class="min-w-0 truncate px-2 text-sm">Capture region</span>
    {:else}
      <CaptureBindingProfileSelect
        label={`Profile for ${profileName} capture shortcut`}
        profileId={binding.profileId}
        {profiles}
        disabled={rowDisabled}
        onSelect={onSelectProfile}
      />
    {/if}

    <ShortcutRecorder
      label={binding.profileId === null
        ? 'Shortcut to capture a region with the active Profile'
        : `Shortcut to capture a region with ${profileName}`}
      labelHidden
      id={`capture-binding-${binding.id}`}
      value={binding.shortcut}
      {recording}
      {recordingPending}
      {disabled}
      {onStart}
      {onRecord}
      {onCancel}
    />

    {#if binding.profileId === null}
      <span aria-hidden="true" class="size-10 shrink-0"></span>
    {:else}
      <Button
        variant="ghost"
        size="icon"
        class="shrink-0"
        type="button"
        aria-label={`Delete ${profileName} capture shortcut`}
        title={`Delete ${profileName} capture shortcut`}
        disabled={rowDisabled}
        onclick={onDelete}
      >
        <Icon name="trash" />
      </Button>
    {/if}
  </div>
</li>
