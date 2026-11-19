import { getLocalTimeZone, parseDate } from '@internationalized/date';
import type { ContentTag, HistoryFilter } from '$lib/generated/bindings';
import { INFERENCE_BACKEND_LABELS } from '$lib/inference-profile-state';

/** Matches the backend's `SEARCH_MATCH_START` and `SEARCH_MATCH_END`. */
const MATCH_START = '\u{E000}';
const MATCH_END = '\u{E001}';

export type SnippetSegment = { text: string; match: boolean };

/** Splits a search snippet into plain and matched runs, dropping the markers. */
export function snippetSegments(snippet: string): SnippetSegment[] {
  const segments: SnippetSegment[] = [];
  for (const [index, part] of snippet.split(MATCH_START).entries()) {
    const [matched, rest] = index === 0 ? [null, part] : splitOnce(part, MATCH_END);
    if (matched) segments.push({ text: matched, match: true });
    if (rest) segments.push({ text: rest, match: false });
  }
  return segments;
}

function splitOnce(value: string, separator: string): [string, string] {
  const at = value.indexOf(separator);
  return at < 0 ? [value, ''] : [value.slice(0, at), value.slice(at + separator.length)];
}

// History keeps providers this version may no longer support.
const removedProviderLabels: Record<string, string> = {
  openai_subscription: 'ChatGPT subscription',
};

export function providerLabel(provider: string): string {
  return (
    (INFERENCE_BACKEND_LABELS as Record<string, string>)[provider] ??
    removedProviderLabels[provider] ??
    provider
  );
}

/** What the History view can narrow by; empty values do not filter. */
export type HistoryViewFilter = {
  query: string;
  outcome: 'all' | 'success' | 'failure';
  profileId: string | null;
  /** First and last local calendar day to include, as `YYYY-MM-DD`. */
  from: string | null;
  to: string | null;
  /** Content every listed result must contain. */
  contains: ContentTag[];
};

export const CONTENT_TAG_LABELS: Record<ContentTag, string> = {
  tables: 'Tables',
  images: 'Images',
  styling: 'Styling',
  code: 'Code',
};

export const EMPTY_HISTORY_FILTER: HistoryViewFilter = {
  query: '',
  outcome: 'all',
  profileId: null,
  from: null,
  to: null,
  contains: [],
};

/** Filters other than the search text, which the filter menu shows and clears. */
export function activeFilterCount(filter: HistoryViewFilter): number {
  return [
    filter.outcome !== 'all',
    filter.profileId !== null,
    filter.from !== null,
    filter.contains.length > 0,
  ].filter(Boolean).length;
}

export function isFiltered(filter: HistoryViewFilter): boolean {
  return filter.query.trim() !== '' || activeFilterCount(filter) > 0;
}

/**
 * Converts the view filter to the backend's. A day range covers local
 * midnight of `from` up to, but not including, local midnight after `to`.
 */
export function historyFilter(
  filter: HistoryViewFilter,
  timeZone = getLocalTimeZone(),
): HistoryFilter {
  return {
    query: filter.query.trim() || null,
    outcome: filter.outcome === 'all' ? null : filter.outcome,
    profileId: filter.profileId,
    since: filter.from ? parseDate(filter.from).toDate(timeZone).toISOString() : null,
    until: filter.to ? parseDate(filter.to).add({ days: 1 }).toDate(timeZone).toISOString() : null,
    contains: filter.contains,
  };
}
