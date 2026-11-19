<script lang="ts">
  import { onMount } from 'svelte';
  import {
    isModifierOnlyEvent,
    shortcutFromKeyboardEvent,
    type ShortcutKeyboardEvent,
  } from '$lib/shortcut-recorder';
  import { presentShortcut, type ShortcutPresentationPlatform } from '$lib/shortcut-presentation';
  import { getWindowPresentationCapabilities } from '$lib/window-presentation';

  let {
    label,
    id,
    value,
    labelHidden = false,
    recording,
    recordingPending,
    disabled,
    onStart,
    onRecord,
    onCancel,
  }: {
    label: string;
    id: string;
    value: string;
    labelHidden?: boolean;
    recording: boolean;
    recordingPending: boolean;
    disabled: boolean;
    onStart: () => void;
    onRecord: (shortcut: string) => void;
    onCancel: () => void;
  } = $props();

  let recorderMessage = $state('');
  let recorderButton: HTMLButtonElement | undefined;
  let shortcutPresentationPlatform = $state<ShortcutPresentationPlatform>('generic');

  const labelId = $derived(`${id}-label`);
  const controlId = $derived(`${id}-control`);
  const messageId = $derived(`${id}-message`);
  const presentation = $derived(presentShortcut(value, shortcutPresentationPlatform));
  const displayValue = $derived(presentation.display || 'Click to record');
  const spokenValue = $derived(presentation.spoken || 'No shortcut set');
  const accessibleLabel = $derived(
    recordingPending
      ? `${label}: Preparing shortcut`
      : recording
        ? `${label}: Press a shortcut`
        : `${label}: ${spokenValue}`,
  );

  onMount(() => {
    getWindowPresentationCapabilities()
      .then(
        (capabilities) =>
          (shortcutPresentationPlatform = capabilities.shortcutPresentationPlatform),
      )
      .catch(() => (shortcutPresentationPlatform = 'generic'));
  });

  $effect(() => {
    if (recording) queueMicrotask(() => recorderButton?.focus());
  });

  function startRecording(): void {
    recorderMessage = '';
    onStart();
    recorderButton?.focus();
  }

  function cancelRecording(): void {
    recorderMessage = '';
    onCancel();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (!recording && !recordingPending) return;

    if (event.key === 'Escape') {
      event.preventDefault();
      cancelRecording();
      return;
    }

    if (!recording) {
      event.preventDefault();
      return;
    }

    if (event.repeat || isModifierOnlyEvent(event)) {
      event.preventDefault();
      if (event.repeat) recorderMessage = 'Release the keys, then press the shortcut once.';
      return;
    }

    const shortcut = shortcutFromKeyboardEvent(event as ShortcutKeyboardEvent);
    event.preventDefault();
    if (shortcut) {
      recorderMessage = '';
      onRecord(shortcut);
    } else {
      recorderMessage = 'That key is not available for a global shortcut. Try another key.';
    }
  }

  function handleFocusOut(event: FocusEvent): void {
    const container = event.currentTarget as HTMLElement;
    const nextTarget = event.relatedTarget as Node | null;
    if ((recording || recordingPending) && !container.contains(nextTarget)) cancelRecording();
  }

  function handleWindowBlur(): void {
    if (recording || recordingPending) cancelRecording();
  }

  function describedBy(): string | undefined {
    return recorderMessage ? messageId : undefined;
  }
</script>

<svelte:window onblur={handleWindowBlur} />

<div class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1" onfocusout={handleFocusOut}>
  <label
    class={labelHidden ? 'sr-only' : 'shrink-0 text-xs font-semibold text-ink'}
    for={controlId}
    id={labelId}>{label}</label
  >
  <div class="flex min-w-0 flex-1 flex-wrap items-center gap-x-3 gap-y-1">
    <button
      id={controlId}
      type="button"
      aria-describedby={describedBy()}
      aria-label={accessibleLabel}
      aria-pressed={recording || recordingPending}
      bind:this={recorderButton}
      {disabled}
      onkeydown={handleKeydown}
      onclick={startRecording}
      class="box-border h-11 min-h-11 min-w-0 flex-1 overflow-hidden rounded-control border border-border bg-surface-control px-3 py-2.5 text-left text-ink text-ellipsis whitespace-nowrap hover:border-border-strong hover:bg-surface-raised aria-pressed:border-ink/12"
    >
      <span class="ml-auto text-right">
        {#if recordingPending}
          Preparing…
        {:else if recording}
          Press a shortcut…
        {:else}
          {displayValue}
        {/if}
      </span>
    </button>
  </div>
  {#if recorderMessage}
    <p id={messageId} class="m-0 basis-full text-xs text-warning" role="status" aria-live="polite">
      {recorderMessage}
    </p>
  {/if}
</div>
