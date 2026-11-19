/// <reference types="bun-types" />

import { expect, test } from 'bun:test';
import {
  createHistoryNavigation,
  type HistoryNavigationEnvironment,
} from '../src/lib/history-navigation';
import type { OpenHistoryEvent } from '../src/lib/generated/bindings';

const settle = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

function environment() {
  const state = {
    focused: false,
    pending: { requestId: 1, entryId: 'capture-entry' } as OpenHistoryEvent | null,
    acknowledged: [] as number[],
    errors: [] as unknown[],
    cleanups: 0,
    focus: () => {},
    request: () => {},
  };
  const api: HistoryNavigationEnvironment = {
    available: () => true,
    ready: async () => state.focused,
    pending: async () => state.pending,
    acknowledge: async (requestId) => {
      state.acknowledged.push(requestId);
      if (state.pending?.requestId === requestId) state.pending = null;
      return null;
    },
    listenRequests: async (wake) => {
      state.request = wake;
      return () => {
        state.cleanups += 1;
      };
    },
    listenFocus: async (wake) => {
      state.focus = wake;
      return () => {
        state.cleanups += 1;
      };
    },
  };
  return { state, api };
}

test('recovers a missed open event when the native window gains focus', async () => {
  const { state, api } = environment();
  const opened: Array<string | null> = [];
  const navigation = createHistoryNavigation(
    async (id) => {
      opened.push(id);
    },
    (error) => state.errors.push(error),
    api,
  );
  const cleanup = navigation.mount();
  await settle();
  expect(opened).toEqual([]);
  expect(state.acknowledged).toEqual([]);
  state.focused = true;
  state.focus();
  await settle();
  expect(opened).toEqual(['capture-entry']);
  expect(state.acknowledged).toEqual([1]);
  expect(state.pending).toBeNull();
  expect(state.errors).toEqual([]);
  cleanup();
  expect(state.cleanups).toBe(2);
});

test('mount consumes an existing request and acknowledges only after applying it, retaining newer requests', async () => {
  const { state, api } = environment();
  state.focused = true;
  const opened: Array<string | null> = [];
  let completeFirst!: () => void;
  const navigation = createHistoryNavigation(
    async (id) => {
      opened.push(id);
      if (id === 'capture-entry')
        await new Promise<void>((resolve) => {
          completeFirst = resolve;
        });
    },
    (error) => state.errors.push(error),
    api,
  );
  const cleanup = navigation.mount();
  await settle();
  expect(opened).toEqual(['capture-entry']);
  expect(state.acknowledged).toEqual([]);
  state.pending = { requestId: 2, entryId: 'latest-entry' };
  state.request();
  completeFirst();
  await settle();
  expect(opened).toEqual(['capture-entry', 'latest-entry']);
  expect(state.acknowledged).toEqual([1, 2]);
  expect(state.pending).toBeNull();
  expect(state.errors).toEqual([]);
  cleanup();
});
