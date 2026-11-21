<script lang="ts">
  import HistoryBrowser from '$lib/components/HistoryBrowser.svelte';
  import type { HistoryDebugKind } from '$lib/generated/bindings';
  import type { HistoryViewState } from '$lib/page-types';

  let {
    state,
    availability,
    advanced,
    debug,
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
    state: HistoryViewState;
    availability: { desktopAvailable: boolean };
    advanced: boolean;
    debug: boolean;
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
    onFilterChange: (
      change: Partial<import('$lib/history-search').HistoryViewFilter>,
      immediate?: boolean,
    ) => void;
  } = $props();
</script>

<div class="flex h-full min-h-0 min-w-0">
  <HistoryBrowser
    formatting={state.formatting}
    {onFormattingChange}
    entries={state.entries}
    selectedId={state.selectedId}
    {advanced}
    {debug}
    loading={state.loading}
    loadingMore={state.loadingMore}
    hasMore={state.hasMore}
    filter={state.filter}
    facets={state.facets}
    debugCopied={state.debugCopied}
    resultCopied={state.resultCopied}
    failureCopiedId={state.failureCopiedId}
    disabled={!availability.desktopAvailable}
    {captureShortcut}
    {onSelect}
    {onRestore}
    {onCopyPlainText}
    {onCopyFailureDetails}
    {onCopyDebugHtml}
    {onDelete}
    {onLoadMore}
    {onFilterChange}
  />
</div>
