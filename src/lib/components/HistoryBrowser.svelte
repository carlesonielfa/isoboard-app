<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  import CaptureIcon from '$lib/components/CaptureIcon.svelte';
  import CaptureInstructions from '$lib/components/CaptureInstructions.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import HistoryDetail from '$lib/components/HistoryDetail.svelte';
  import HistoryFilters from '$lib/components/HistoryFilters.svelte';
  import { isFiltered, type HistoryViewFilter } from '$lib/history-search';
  import type { HistoryFacets } from '$lib/generated/bindings';
  import HistoryTabEntry from '$lib/components/HistoryTabEntry.svelte';
  import SidebarTabContent from '$lib/components/SidebarTabContent.svelte';
  import SidebarTabList from '$lib/components/SidebarTabList.svelte';
  import SidebarTabs from '$lib/components/SidebarTabs.svelte';
  import type { HistoryDebugKind } from '$lib/generated/bindings';
  import type { HistoryEntry, HistoryViewState } from '$lib/page-types';

  let {
    entries,
    formatting,
    selectedId,
    advanced,
    debug,
    loading,
    loadingMore,
    hasMore,
    filter,
    facets,
    debugCopied,
    resultCopied,
    failureCopiedId,
    disabled,
    captureShortcut,
    onSelect,
    onFormattingChange,
    onRestore,
    onCopyPlainText,
    onCopyFailureDetails,
    onCopyDebugHtml,
    onDelete,
    onLoadMore,
    onFilterChange,
  }: {
    entries: HistoryEntry[];
    formatting: HistoryViewState['formatting'];
    selectedId: string | null;
    advanced: boolean;
    debug: boolean;
    loading: boolean;
    loadingMore: boolean;
    hasMore: boolean;
    filter: HistoryViewFilter;
    facets: HistoryFacets;
    debugCopied: HistoryViewState['debugCopied'];
    resultCopied: HistoryViewState['resultCopied'];
    failureCopiedId: string | null;
    disabled: boolean;
    /** Canonical accelerator shown in the first-capture prompt. */
    captureShortcut: string;
    onSelect: (id: string) => void;
    onFormattingChange: (
      id: string,
      settings: import('$lib/generated/bindings').ProcessingConversionSettings,
    ) => void;
    onRestore: (id: string) => void;
    onCopyPlainText: (id: string) => void;
    onCopyFailureDetails: (id: string) => void;
    onCopyDebugHtml: (id: string, kind: HistoryDebugKind) => void;
    onDelete: (id: string) => void;
    onLoadMore: () => void;
    onFilterChange: (change: Partial<HistoryViewFilter>, immediate?: boolean) => void;
  } = $props();
  const filtered = $derived(isFiltered(filter));
  const neverCaptured = $derived(entries.length === 0 && !filtered && !loading);
  let detailSize = $state<number | null>(null);
  const selectedEntry = $derived(entries.find((entry) => entry.id === selectedId) ?? null);

  /** Requests older entries whenever the end of the list scrolls into view. */
  function loadMoreWhenVisible(node: HTMLElement) {
    const observer = new IntersectionObserver(
      (records) => {
        if (records.some((record) => record.isIntersecting)) onLoadMore();
      },
      { root: node.parentElement, rootMargin: '200px' },
    );
    observer.observe(node);
    return { destroy: () => observer.disconnect() };
  }
</script>

<div class="flex min-h-0 min-w-0 flex-1 overflow-hidden" aria-label="Clipboard history">
  <SidebarTabs
    value={selectedId ?? undefined}
    onValueChange={onSelect}
    class="grid h-full min-h-0 w-full grid-cols-[18rem_minmax(0,1fr)] grid-rows-[minmax(0,1fr)]"
  >
    <div class="flex min-h-0 min-w-0 flex-col border-r border-border">
      <HistoryFilters {filter} {facets} {disabled} onChange={onFilterChange} />
      <SidebarTabList
        aria-label="History entries"
        class="flex min-h-0 min-w-0 flex-1 flex-col items-stretch justify-start gap-1 overflow-y-auto p-2"
      >
        {#each entries as entry (entry.id)}
          <HistoryTabEntry
            {entry}
            selected={entry.id === selectedId}
            {disabled}
            copyDisabled={formatting?.id === entry.id && (formatting.loading || !!formatting.error)}
            hasPlainText={formatting?.id === entry.id
              ? formatting.hasPlainText
              : entry.status === 'completed' && entry.hasPlainText}
            {resultCopied}
            {onRestore}
            {onCopyPlainText}
            {failureCopiedId}
            {onCopyFailureDetails}
            {onDelete}
          />
        {/each}
        {#if hasMore}
          <div use:loadMoreWhenVisible class="flex justify-center p-2">
            <Button
              variant="ghost"
              size="sm"
              type="button"
              disabled={loadingMore}
              onclick={onLoadMore}
            >
              {loadingMore ? 'Loading older entries…' : 'Load older entries'}
            </Button>
          </div>
        {/if}
      </SidebarTabList>
      {#if entries.length === 0 && (loading || filtered)}
        <p class="m-0 px-4 py-3 text-sm" role="status" aria-live="polite">
          {loading ? (filtered ? 'Searching…' : 'Loading history…') : 'No matching entries.'}
        </p>
      {/if}
    </div>

    <div class="min-h-0 min-w-0 overflow-hidden">
      <!-- Only the selected entry gets a detail pane, so list size does not
           multiply preview surfaces. -->
      {#if selectedEntry}
        {@const entry = selectedEntry}
        <SidebarTabContent value={entry.id} class="h-full min-h-0 m-0 overflow-y-auto">
          <HistoryDetail
            {entry}
            formatting={formatting?.id === entry.id ? formatting : null}
            {onFormattingChange}
            {advanced}
            {debug}
            {debugCopied}
            {failureCopiedId}
            {disabled}
            {onCopyFailureDetails}
            {onCopyDebugHtml}
            bind:detailSize
          />
        </SidebarTabContent>
      {:else if neverCaptured}
        <EmptyState title="Start your first capture">
          {#snippet icon()}<CaptureIcon class="size-10 shrink-0" />{/snippet}
          <CaptureInstructions shortcut={captureShortcut} class="m-0 text-sm text-ink" />
        </EmptyState>
      {/if}
    </div>
  </SidebarTabs>
</div>
