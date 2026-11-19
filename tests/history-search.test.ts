/// <reference types="bun-types" />

import { expect, test } from 'bun:test';
import {
  EMPTY_HISTORY_FILTER,
  activeFilterCount,
  historyFilter,
  isFiltered,
  providerLabel,
  snippetSegments,
} from '../src/lib/history-search';

test('splits search snippets into matched and plain runs', () => {
  expect(snippetSegments('…the \u{E000}Invoice\u{E001} for \u{E000}Café\u{E001}')).toEqual([
    { text: '…the ', match: false },
    { text: 'Invoice', match: true },
    { text: ' for ', match: false },
    { text: 'Café', match: true },
  ]);
  expect(snippetSegments('no markers')).toEqual([{ text: 'no markers', match: false }]);
  // A truncated snippet without a closing marker still shows its text.
  expect(snippetSegments('a \u{E000}b')).toEqual([
    { text: 'a ', match: false },
    { text: 'b', match: true },
  ]);
});

test('covers whole local days, including the last one', () => {
  const range = { ...EMPTY_HISTORY_FILTER, from: '2026-03-28', to: '2026-03-29' };
  // Madrid moves to summer time on 29 March, so the second day is 23 hours long.
  expect(historyFilter(range, 'Europe/Madrid')).toMatchObject({
    since: '2026-03-27T23:00:00.000Z',
    until: '2026-03-29T22:00:00.000Z',
  });
  expect(historyFilter({ ...EMPTY_HISTORY_FILTER, query: '   ' }).query).toBeNull();
  expect(historyFilter(EMPTY_HISTORY_FILTER)).toMatchObject({ since: null, until: null });
});

test('counts filters apart from the search text', () => {
  expect(isFiltered(EMPTY_HISTORY_FILTER)).toBe(false);
  expect(activeFilterCount({ ...EMPTY_HISTORY_FILTER, query: 'invoice' })).toBe(0);
  expect(isFiltered({ ...EMPTY_HISTORY_FILTER, query: 'invoice' })).toBe(true);
  expect(
    activeFilterCount({
      ...EMPTY_HISTORY_FILTER,
      outcome: 'failure',
      from: '2026-01-01',
      to: '2026-01-02',
    }),
  ).toBe(2);
  const contents = { ...EMPTY_HISTORY_FILTER, contains: ['tables', 'code'] as const };
  expect(activeFilterCount({ ...contents, contains: [...contents.contains] })).toBe(1);
  expect(historyFilter({ ...contents, contains: [...contents.contains] }).contains).toEqual([
    'tables',
    'code',
  ]);
});

test('names removed providers and falls back to the stored id', () => {
  expect(providerLabel('openai_subscription')).toBe('ChatGPT subscription');
  expect(providerLabel('someday_api')).toBe('someday_api');
});
