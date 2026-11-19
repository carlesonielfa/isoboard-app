<script lang="ts">
  import { Check, Copy, Trash2 } from '@lucide/svelte';
  import SplitButton from '$lib/components/SplitButton.svelte';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import SidebarTabRow from '$lib/components/SidebarTabRow.svelte';
  import SidebarTabTrigger from '$lib/components/SidebarTabTrigger.svelte';
  import type { HistoryEntry, HistoryViewState } from '$lib/page-types';
  import { snippetSegments } from '$lib/history-search';

  let {
    entry,
    selected,
    disabled,
    resultCopied,
    hasPlainText,
    onRestore,
    onCopyPlainText,
    copyDisabled = false,
    failureCopiedId,
    onCopyFailureDetails,
    onDelete,
  }: {
    entry: HistoryEntry;
    selected: boolean;
    disabled: boolean;
    resultCopied: HistoryViewState['resultCopied'];
    hasPlainText: boolean;
    onRestore: (id: string) => void;
    onCopyPlainText: (id: string) => void;
    copyDisabled?: boolean;
    failureCopiedId: string | null;
    onCopyFailureDetails: (id: string) => void;
    onDelete: (id: string) => void;
  } = $props();

  const preview = $derived(
    entry.status === 'completed'
      ? entry.plainTextPreview.trim() || 'Image'
      : entry.status === 'pending'
        ? 'Processing image…'
        : 'Capture failed',
  );
  const copiedFormat = $derived(resultCopied?.id === entry.id ? resultCopied.format : null);
  const matchSegments = $derived(
    entry.status === 'completed' && entry.matchSnippet ? snippetSegments(entry.matchSnippet) : null,
  );
</script>

<SidebarTabRow {selected}>
  <SidebarTabTrigger value={entry.id} aria-label={`View history entry: ${preview}`}>
    {#if entry.thumbnail}
      <img class="h-12 w-14 shrink-0 rounded-control object-cover" src={entry.thumbnail} alt="" />
    {:else}
      <span
        class="flex h-12 w-14 shrink-0 items-center justify-center rounded-control bg-surface-control"
        aria-hidden="true">—</span
      >
    {/if}
    <span class="line-clamp-2 min-w-0 break-words text-xs leading-4">
      {#if matchSegments}
        {#each matchSegments as segment, index (index)}
          {#if segment.match}<mark class="rounded-sm bg-highlight-surface text-ink"
              >{segment.text}</mark
            >{:else}{segment.text}{/if}
        {/each}
      {:else}
        {preview}
      {/if}
      {#if entry.status === 'failed'}
        <span class="block text-danger">{entry.errorMessage}</span>
      {/if}
    </span>
  </SidebarTabTrigger>

  {#if entry.status !== 'pending'}
    <SplitButton
      size="icon-sm"
      class="mr-1"
      label={entry.status === 'failed' ? 'Copy error details' : 'Copy result'}
      title={copiedFormat || failureCopiedId === entry.id
        ? 'Copied'
        : entry.status === 'failed'
          ? 'Copy error details'
          : 'Copy result'}
      menuLabel="History entry actions"
      {disabled}
      actionDisabled={copyDisabled}
      onclick={() =>
        entry.status === 'failed' ? onCopyFailureDetails(entry.id) : onRestore(entry.id)}
    >
      {#snippet menu()}
        {#if entry.status === 'completed'}
          <DropdownMenu.Item
            disabled={disabled || copyDisabled || !hasPlainText}
            onSelect={() => onCopyPlainText(entry.id)}
          >
            Copy as plain text
          </DropdownMenu.Item>
          <DropdownMenu.Separator />
        {/if}
        <DropdownMenu.Item variant="destructive" {disabled} onSelect={() => onDelete(entry.id)}>
          <Trash2 class="size-4" aria-hidden="true" /> Delete
        </DropdownMenu.Item>
      {/snippet}
      {#if copiedFormat || failureCopiedId === entry.id}
        <Check class="size-4 shrink-0 text-success" aria-hidden="true" />
      {:else}
        <Copy class="size-4 shrink-0" strokeWidth={2} aria-hidden="true" />
      {/if}
    </SplitButton>
    <span class="sr-only" role="status" aria-live="polite">
      {copiedFormat === 'rich'
        ? 'Rich result copied'
        : copiedFormat === 'plain'
          ? 'Plain text copied'
          : ''}
    </span>
  {:else if entry.status === 'pending'}
    <span
      class="mr-2 size-4 shrink-0 animate-spin rounded-full border-2 border-ink/25 border-t-ink motion-reduce:animate-none"
      role="status"
      aria-label="Processing image"
    ></span>
  {/if}
</SidebarTabRow>
