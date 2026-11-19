/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import { createSerializedBooleanWorker } from '../src/lib/serialized-boolean-worker';

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });
  return { promise, resolve, reject };
}

function tick(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

describe('createSerializedBooleanWorker', () => {
  test('serializes overlapping enable and cancel requests', async () => {
    const calls: boolean[] = [];
    const first = deferred<boolean>();
    const second = deferred<boolean>();
    const worker = createSerializedBooleanWorker((value) => {
      calls.push(value);
      return calls.length === 1 ? first.promise : second.promise;
    });

    const enabled = worker.request(true);
    const cancelled = worker.request(false);
    expect(await enabled).toBe(false);
    expect(calls).toEqual([true]);

    first.resolve(true);
    await tick();
    expect(calls).toEqual([true, false]);
    second.resolve(true);
    expect(await cancelled).toBe(true);
  });

  test('sends one corrective request when a stale clear completes', async () => {
    const calls: boolean[] = [];
    const requests = [deferred<boolean>(), deferred<boolean>(), deferred<boolean>()];
    const worker = createSerializedBooleanWorker((value) => {
      calls.push(value);
      return requests[calls.length - 1].promise;
    });

    requests[0].resolve(true);
    expect(await worker.request(true)).toBe(true);
    const cleared = worker.request(false);
    const reenabled = worker.request(true);
    requests[1].resolve(true);
    await tick();
    expect(calls).toEqual([true, false, true]);
    requests[2].resolve(true);
    expect(await cleared).toBe(false);
    expect(await reenabled).toBe(true);
  });

  test('treats native refusal as a bounded failure instead of retrying forever', async () => {
    let calls = 0;
    const worker = createSerializedBooleanWorker(async () => {
      calls += 1;
      return false;
    });

    expect(await worker.request(true)).toBe(false);
    expect(calls).toBe(1);
  });

  test('does not claim a failed clear released native state', async () => {
    let calls = 0;
    const worker = createSerializedBooleanWorker(async (value) => {
      calls += 1;
      return value;
    });

    expect(await worker.request(true)).toBe(true);
    expect(await worker.request(false)).toBe(false);
    expect(calls).toBe(2);
  });
});
