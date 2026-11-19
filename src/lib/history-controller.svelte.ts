import { applicationConfig } from '$lib/application-config';
import { convertFileSrc, isTauri } from '@tauri-apps/api/core';
import { errorIdentity, errorMessage, type AppErrorStore } from '$lib/app-error';
import { commands, events } from '$lib/ipc';
import type {
  CompletedHistoryRow,
  FailedHistoryRow,
  HistoryDebugKind,
  HistorySummaryRow,
  HistoryWindowSnapshot,
  PendingHistoryRow,
  ProcessingConversionSettings,
} from '$lib/generated/bindings';
import type { DesktopAvailability, HistoryEntry, HistoryViewState } from '$lib/page-types';
import {
  EMPTY_HISTORY_FILTER,
  historyFilter,
  isFiltered,
  type HistoryViewFilter,
} from '$lib/history-search';

type StateFactory = <T>(value: T) => T;
type HistoryEnvironment = {
  isTauri: () => boolean;
  commands: Pick<
    typeof commands,
    | 'listHistory'
    | 'getHistoryFacets'
    | 'getHistoryPreview'
    | 'copyFailedHistoryDetails'
    | 'restoreHistoryEntry'
    | 'copyHistoryPlainText'
    | 'previewHistoryFormatting'
    | 'copyHistoryFormatted'
    | 'copyHistoryDebugHtml'
    | 'deleteHistoryEntry'
    | 'clearHistory'
  >;
  events: {
    historyChanged: Pick<typeof events.historyChanged, 'listen'>;
    historyError: Pick<typeof events.historyError, 'listen'>;
  };
  thumbnailUrl: (id: string) => string;
};

/** Rows fetched per page of older entries. */
const HISTORY_PAGE_SIZE = 50;
/** Matches the backend's `MAX_HISTORY_PAGE_SIZE`. */
const MAX_HISTORY_PAGE_SIZE = 500;
/** Typing pauses this long before the list is searched again. */
const SEARCH_DEBOUNCE_MILLISECONDS = 200;
/** Matches the backend's `THUMBNAIL_PROTOCOL`. */
const THUMBNAIL_PROTOCOL = 'history-thumbnail';

const defaultEnvironment: HistoryEnvironment = {
  isTauri,
  commands,
  events,
  thumbnailUrl: (id) => convertFileSrc(id, THUMBNAIL_PROTOCOL),
};

function createReactiveState<T>(initialState: T): T {
  const state = $state(initialState);
  return state;
}

type ThumbnailUrl = HistoryEnvironment['thumbnailUrl'];

function normalizeEntry(source: HistorySummaryRow, thumbnailUrl: ThumbnailUrl): HistoryEntry {
  return {
    id: source.id,
    sequence: source.sequence,
    createdAt: source.timestamp,
    thumbnail: source.hasThumbnail ? thumbnailUrl(source.id) : null,
    hasOriginal: source.hasOriginal,
    captureSource: source.captureSource,
    status: 'completed',
    plainTextPreview: source.plainTextPreview,
    matchSnippet: source.matchSnippet,
    contentTags: source.contentTags,
    hasPlainText: source.hasPlainText,
    canRestoreFormatting: source.canRestoreFormatting,
    processingProfile: source.processingProfileName,
    settingsSnapshot: source.settingsSnapshot,
    imageWidth: source.imageWidth,
    imageHeight: source.imageHeight,
    hasFirstInferenceOutput: source.hasFirstInferenceOutput,
    hasSecondInferenceOutput: source.hasSecondInferenceOutput,
  };
}

function normalizeFailed(source: FailedHistoryRow, thumbnailUrl: ThumbnailUrl): HistoryEntry {
  return {
    id: source.id,
    sequence: source.sequence,
    createdAt: source.timestamp,
    thumbnail: source.hasThumbnail ? thumbnailUrl(source.id) : null,
    hasOriginal: source.hasOriginal,
    captureSource: source.captureSource,
    processingProfile: source.processingProfileName,
    settingsSnapshot: source.settingsSnapshot,
    imageWidth: source.imageWidth,
    imageHeight: source.imageHeight,
    status: 'failed',
    errorCode: source.errorCode,
    errorMessage: source.errorMessage,
    errorDetails: source.errorDetails,
    hasRawInferenceOutput: source.hasRawInferenceOutput,
  };
}

function normalizePending(source: PendingHistoryRow): HistoryEntry {
  return {
    id: source.id,
    sequence: null,
    createdAt: source.timestamp,
    thumbnail: source.thumbnailDataUrl,
    hasOriginal: false,
    captureSource: source.captureSource,
    processingProfile: source.processingProfileName,
    settingsSnapshot: null,
    imageWidth: source.imageWidth,
    imageHeight: source.imageHeight,
    status: 'pending',
  };
}

function normalizeRows(rows: CompletedHistoryRow[], thumbnailUrl: ThumbnailUrl): HistoryEntry[] {
  return rows.map((row) =>
    row.outcome === 'success'
      ? normalizeEntry(row, thumbnailUrl)
      : normalizeFailed(row, thumbnailUrl),
  );
}

/** A pending capture has no text yet, so it only shows in the unfiltered list. */
function normalizeEntries(
  value: HistoryWindowSnapshot,
  thumbnailUrl: ThumbnailUrl,
  filtered: boolean,
): HistoryEntry[] {
  const entries = normalizeRows(value.entries, thumbnailUrl);
  if (value.pending && !filtered) entries.unshift(normalizePending(value.pending));
  return entries;
}

/** The oldest saved row currently shown, which bounds refreshes and pages. */
function oldestSequence(entries: HistoryEntry[]): number | null {
  let oldest: number | null = null;
  for (const entry of entries) {
    if (entry.sequence !== null && (oldest === null || entry.sequence < oldest))
      oldest = entry.sequence;
  }
  return oldest;
}

export function createHistoryController(
  availability: DesktopAvailability,
  errors?: AppErrorStore,
  stateFactory: StateFactory = createReactiveState,
  environment: HistoryEnvironment = defaultEnvironment,
) {
  const initialState: HistoryViewState = {
    entries: [],
    selectedId: null,
    loaded: false,
    loading: false,
    loadingMore: false,
    hasMore: false,
    filter: { ...EMPTY_HISTORY_FILTER },
    facets: { profiles: [] },
    error: null,
    debugCopied: null,
    resultCopied: null,
    failureCopiedId: null,
    formatting: null,
  };
  const state = stateFactory(initialState);
  let disposed = false;
  let requestSequence = 0;
  let pageSequence = 0;
  let previewSequence = 0;
  let formattingSequence = 0;
  let localErrorRevision = 0;
  let debugFeedbackTimer: ReturnType<typeof setTimeout> | undefined;
  let resultFeedbackTimer: ReturnType<typeof setTimeout> | undefined;
  let failureFeedbackTimer: ReturnType<typeof setTimeout> | undefined;
  const commandApi = environment.commands;
  const eventApi = environment.events;
  let requestedSelection: string | null = null;
  let searchTimer: ReturnType<typeof setTimeout> | undefined;

  function reportError(error: unknown): void {
    state.error = errorMessage(error);
    errors?.show(state.error, errorIdentity('history', ++localErrorRevision));
  }

  /**
   * Re-reads every saved row from the oldest one shown, so inserts, deletes
   * and pruning are all reflected without reloading pages nobody scrolled to.
   */
  async function load(): Promise<void> {
    if (!availability.desktopAvailable || disposed) return;
    const sequence = ++requestSequence;
    pageSequence += 1;
    state.loadingMore = false;
    state.loading = true;
    state.error = null;
    const oldest = oldestSequence(state.entries);
    const shown = state.entries.filter((entry) => entry.sequence !== null).length;

    try {
      const filter = historyFilter(state.filter);
      const response =
        oldest === null
          ? await commandApi.listHistory(null, null, HISTORY_PAGE_SIZE, filter)
          : await commandApi.listHistory(
              null,
              oldest,
              Math.min(MAX_HISTORY_PAGE_SIZE, shown + HISTORY_PAGE_SIZE),
              filter,
            );
      if (disposed || sequence !== requestSequence) return;
      state.entries = normalizeEntries(
        response,
        environment.thumbnailUrl,
        isFiltered(state.filter),
      );
      state.hasMore = response.hasMore;
      if (requestedSelection !== null) {
        if (state.entries.some((entry) => entry.id === requestedSelection)) {
          state.selectedId = requestedSelection;
        }
        requestedSelection = null;
      }
      if (!state.entries.some((entry) => entry.id === state.selectedId)) {
        state.selectedId = state.entries[0]?.id ?? null;
      }
      if (state.formatting?.id !== state.selectedId) resetFormatting();
      state.loaded = true;
    } catch (error) {
      if (disposed || sequence !== requestSequence) return;
      reportError(error);
      state.loaded = true;
    } finally {
      if (!disposed && sequence === requestSequence) state.loading = false;
    }
  }

  /** Appends the next page of older entries. */
  async function loadMore(): Promise<void> {
    if (
      !availability.desktopAvailable ||
      disposed ||
      !state.hasMore ||
      state.loading ||
      state.loadingMore
    )
      return;
    const before = oldestSequence(state.entries);
    if (before === null) return load();
    const sequence = ++pageSequence;
    state.loadingMore = true;
    try {
      const response = await commandApi.listHistory(
        before,
        null,
        HISTORY_PAGE_SIZE,
        historyFilter(state.filter),
      );
      if (disposed || sequence !== pageSequence) return;
      // Rows come strictly before the oldest loaded sequence, so they never overlap.
      state.entries = [
        ...state.entries,
        ...normalizeRows(response.entries, environment.thumbnailUrl),
      ];
      state.hasMore = response.hasMore;
    } catch (error) {
      if (disposed || sequence !== pageSequence) return;
      reportError(error);
    } finally {
      if (!disposed && sequence === pageSequence) state.loadingMore = false;
    }
  }

  /** Reads which profiles and providers History holds, for the filter choices. */
  async function loadFacets(): Promise<void> {
    if (!availability.desktopAvailable || disposed) return;
    try {
      const facets = await commandApi.getHistoryFacets();
      if (!disposed) state.facets = facets;
    } catch (error) {
      if (!disposed) reportError(error);
    }
  }

  /**
   * Applies a filter change and lists the first matching page. Text changes
   * wait for a pause in typing unless `immediate`; other changes apply at once.
   */
  function setFilter(change: Partial<HistoryViewFilter>, immediate = false): void {
    state.filter = { ...state.filter, ...change };
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = undefined;
    const apply = () => {
      searchTimer = undefined;
      state.entries = [];
      state.hasMore = false;
      void load();
    };
    if (!immediate && Object.keys(change).every((key) => key === 'query'))
      searchTimer = setTimeout(apply, SEARCH_DEBOUNCE_MILLISECONDS);
    else apply();
  }

  function select(id: string): void {
    requestedSelection = null;
    if (id === state.selectedId) return;
    if (state.entries.some((entry) => entry.id === id)) {
      state.selectedId = id;
      resetFormatting();
    }
  }

  /** Shows a specific entry, clearing any filter that could hide it. */
  async function open(id: string | null): Promise<void> {
    requestedSelection = id;
    if (isFiltered(state.filter)) {
      if (searchTimer) clearTimeout(searchTimer);
      searchTimer = undefined;
      state.filter = { ...EMPTY_HISTORY_FILTER };
      state.entries = [];
    }
    await load();
  }

  function originalSettings(entry: HistoryEntry): ProcessingConversionSettings {
    return {
      tables: entry.settingsSnapshot?.tables ?? true,
      colors: entry.settingsSnapshot?.colors ?? true,
      images: entry.settingsSnapshot?.images ?? true,
    };
  }

  function resetFormatting(): void {
    formattingSequence += 1;
    const entry = state.entries.find((entry) => entry.id === state.selectedId);
    state.formatting =
      entry?.status === 'completed'
        ? {
            id: entry.id,
            settings: originalSettings(entry),
            previewHtml: '',
            previewLoading: true,
            hasPlainText: entry.hasPlainText,
            changed: false,
            loading: false,
            error: null,
          }
        : null;
    if (state.formatting) void loadPreview(state.formatting.id);
  }

  /** Reads the stored preview of the selected entry, unless a reformatted one replaced it. */
  async function loadPreview(id: string): Promise<void> {
    if (!availability.desktopAvailable || disposed) return;
    const sequence = ++previewSequence;
    try {
      const html = await commandApi.getHistoryPreview(id);
      if (disposed || sequence !== previewSequence || state.formatting?.id !== id) return;
      if (!state.formatting.changed) state.formatting.previewHtml = html;
    } catch (error) {
      if (disposed || sequence !== previewSequence || state.formatting?.id !== id) return;
      reportError(error);
    } finally {
      if (!disposed && sequence === previewSequence && state.formatting?.id === id)
        state.formatting.previewLoading = false;
    }
  }

  async function changeFormatting(
    id: string,
    settings: ProcessingConversionSettings,
  ): Promise<void> {
    const entry = state.entries.find((entry) => entry.id === id);
    if (
      !availability.desktopAvailable ||
      disposed ||
      entry?.status !== 'completed' ||
      id !== state.selectedId
    )
      return;
    const original = originalSettings(entry);
    const restores =
      entry.canRestoreFormatting && entry.settingsSnapshot?.applicationMode === 'html_cleanup';
    if (
      !restores &&
      Object.keys(original).some((key) => {
        const option = key as keyof ProcessingConversionSettings;
        return settings[option] && !original[option];
      })
    )
      return;
    const changed =
      (state.formatting?.id === id && state.formatting.changed) ||
      Object.keys(original).some((key) => {
        const option = key as keyof ProcessingConversionSettings;
        return settings[option] !== original[option];
      });
    const sequence = ++formattingSequence;
    state.resultCopied = null;
    state.formatting = {
      id,
      settings: { ...settings },
      changed,
      previewHtml: state.formatting?.id === id ? state.formatting.previewHtml : '',
      previewLoading: state.formatting?.id === id ? state.formatting.previewLoading : false,
      hasPlainText: entry.hasPlainText,
      loading: changed,
      error: null,
    };
    if (!changed) return;
    try {
      const preview = await commandApi.previewHistoryFormatting(id, { ...settings });
      if (disposed || sequence !== formattingSequence || state.formatting?.id !== id) return;
      state.formatting.previewHtml = preview.previewHtml;
      state.formatting.hasPlainText = preview.hasPlainText;
    } catch (error) {
      if (disposed || sequence !== formattingSequence || state.formatting?.id !== id) return;
      state.formatting.error = errorMessage(error);
      state.formatting.previewHtml = '';
      state.formatting.hasPlainText = false;
    } finally {
      if (!disposed && sequence === formattingSequence && state.formatting?.id === id)
        state.formatting.loading = false;
    }
  }

  function markResultCopied(id: string, format: 'rich' | 'plain'): void {
    state.resultCopied = { id, format };
    if (resultFeedbackTimer) clearTimeout(resultFeedbackTimer);
    resultFeedbackTimer = setTimeout(() => {
      state.resultCopied = null;
      resultFeedbackTimer = undefined;
    }, applicationConfig.feedback.copyMilliseconds);
  }

  async function restore(id: string): Promise<boolean> {
    if (
      !availability.desktopAvailable ||
      !state.entries.some((entry) => entry.id === id && entry.status === 'completed')
    )
      return false;
    try {
      const formatting = state.formatting?.id === id ? state.formatting : null;
      if (formatting?.loading || formatting?.error) return false;
      if (formatting?.changed)
        await commandApi.copyHistoryFormatted(id, { ...formatting.settings }, false);
      else await commandApi.restoreHistoryEntry(id);
      markResultCopied(id, 'rich');
      return true;
    } catch (error) {
      reportError(error);
      return false;
    }
  }

  async function copyPlainText(id: string): Promise<boolean> {
    if (
      !availability.desktopAvailable ||
      !state.entries.some(
        (entry) =>
          entry.id === id &&
          entry.status === 'completed' &&
          (state.formatting?.id === id ? state.formatting.hasPlainText : entry.hasPlainText),
      )
    )
      return false;
    try {
      const formatting = state.formatting?.id === id ? state.formatting : null;
      if (formatting?.loading || formatting?.error) return false;
      if (formatting?.changed)
        await commandApi.copyHistoryFormatted(id, { ...formatting.settings }, true);
      else await commandApi.copyHistoryPlainText(id);
      markResultCopied(id, 'plain');
      return true;
    } catch (error) {
      reportError(error);
      return false;
    }
  }

  async function copyDebugHtml(id: string, kind: HistoryDebugKind): Promise<boolean> {
    if (
      !availability.desktopAvailable ||
      !state.entries.some(
        (entry) =>
          entry.id === id &&
          (entry.status === 'completed' ||
            (entry.status === 'failed' && entry.hasRawInferenceOutput && kind === 'raw')),
      )
    )
      return false;
    try {
      await commandApi.copyHistoryDebugHtml(id, kind);
      state.debugCopied = { id, kind };
      if (debugFeedbackTimer) clearTimeout(debugFeedbackTimer);
      debugFeedbackTimer = setTimeout(() => {
        state.debugCopied = null;
        debugFeedbackTimer = undefined;
      }, applicationConfig.feedback.copyMilliseconds);
      return true;
    } catch (error) {
      reportError(error);
      return false;
    }
  }

  async function copyFailureDetails(id: string): Promise<boolean> {
    if (
      !availability.desktopAvailable ||
      !state.entries.some((entry) => entry.id === id && entry.status === 'failed')
    )
      return false;
    try {
      await commandApi.copyFailedHistoryDetails(id);
      state.failureCopiedId = id;
      if (failureFeedbackTimer) clearTimeout(failureFeedbackTimer);
      failureFeedbackTimer = setTimeout(() => {
        state.failureCopiedId = null;
        failureFeedbackTimer = undefined;
      }, applicationConfig.feedback.copyMilliseconds);
      return true;
    } catch (error) {
      reportError(error);
      return false;
    }
  }

  async function deleteEntry(id: string): Promise<boolean> {
    if (
      !availability.desktopAvailable ||
      !state.entries.some((entry) => entry.id === id && entry.status !== 'pending')
    )
      return false;
    try {
      await commandApi.deleteHistoryEntry(id);
      await load();
      return true;
    } catch (error) {
      reportError(error);
      return false;
    }
  }

  async function clearAll(): Promise<boolean> {
    if (!availability.desktopAvailable) return false;
    try {
      await commandApi.clearHistory();
      await load();
      return true;
    } catch (error) {
      reportError(error);
      return false;
    }
  }

  function mount(): () => void {
    disposed = false;
    if (!environment.isTauri()) {
      availability.desktopAvailable = false;
      state.loaded = true;
      return () => {
        disposed = true;
        requestSequence += 1;
        pageSequence += 1;
        previewSequence += 1;
        formattingSequence += 1;
      };
    }

    let cleanups: Array<() => void> = [];

    async function initializeListeners(): Promise<void> {
      const results = await Promise.allSettled([
        eventApi.historyChanged.listen(() => {
          if (disposed) return;
          void load();
          void loadFacets();
        }),
        eventApi.historyError.listen((event) => {
          if (!disposed) reportError(event.payload);
        }),
      ]);
      const listeners = results.flatMap((result) =>
        result.status === 'fulfilled' ? [result.value] : [],
      );
      if (disposed) {
        listeners.forEach((cleanup) => cleanup());
        return;
      }
      cleanups = listeners;
      const failure = results.find((result) => result.status === 'rejected');
      if (failure && !disposed) reportError(failure.reason);
    }

    void initializeListeners();
    void load();
    void loadFacets();

    return () => {
      disposed = true;
      requestSequence += 1;
      pageSequence += 1;
      previewSequence += 1;
      formattingSequence += 1;
      cleanups.forEach((cleanup) => cleanup());
      if (debugFeedbackTimer) clearTimeout(debugFeedbackTimer);
      debugFeedbackTimer = undefined;
      if (resultFeedbackTimer) clearTimeout(resultFeedbackTimer);
      resultFeedbackTimer = undefined;
      if (failureFeedbackTimer) clearTimeout(failureFeedbackTimer);
      failureFeedbackTimer = undefined;
      if (searchTimer) clearTimeout(searchTimer);
      searchTimer = undefined;
    };
  }

  return {
    state,
    load,
    loadMore,
    setFilter,
    select,
    open,
    restore,
    copyPlainText,
    changeFormatting,
    copyDebugHtml,
    copyFailureDetails,
    deleteEntry,
    clearAll,
    mount,
  };
}
