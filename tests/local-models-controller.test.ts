import { expect, test } from 'bun:test';
import { createLocalModelsController } from '../src/lib/local-models-controller.svelte';
import type { LocalModelsChangedEvent } from '../src/lib/generated/bindings';

type Event = { event: string; id: number; payload: LocalModelsChangedEvent };
function snapshot(revision: number): LocalModelsChangedEvent {
  return { revision, models: [] };
}
async function settle() {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

test('one shared subscription rejects stale initial and mutation responses', async () => {
  let handler!: (event: Event) => void;
  let resolveInitial!: (state: LocalModelsChangedEvent) => void;
  let resolveAction!: (state: LocalModelsChangedEvent) => void;
  let listens = 0;
  let cleaned = false;
  const controller = createLocalModelsController({
    isDesktop: () => true,
    stateFactory: (value) => value,
    listen: async (callback) => {
      listens++;
      handler = callback;
      return () => {
        cleaned = true;
      };
    },
    commands: {
      prepareLocalModel: async () => snapshot(5),
      getLocalModelsState: () =>
        new Promise((resolve) => {
          resolveInitial = resolve;
        }),
      downloadLocalModel: () =>
        new Promise((resolve) => {
          resolveAction = resolve;
        }),
      cancelLocalModelDownload: async () => snapshot(5),
      removeLocalModelDownload: async () => snapshot(5),
    },
  });
  const cleanup = controller.mount();
  controller.mount();
  await settle();
  handler({ event: 'local-models-changed', id: 1, payload: snapshot(3) });
  resolveInitial(snapshot(1));
  await settle();
  expect(controller.state.snapshot.revision).toBe(3);
  expect(listens).toBe(1);
  const action = controller.act('qwen3.5-2b', 'download');
  expect(controller.state.pending['qwen3.5-2b']).toBe(true);
  handler({ event: 'local-models-changed', id: 1, payload: snapshot(5) });
  resolveAction(snapshot(4));
  await action;
  expect(controller.state.snapshot.revision).toBe(5);
  expect(controller.state.pending['qwen3.5-2b']).toBe(false);
  cleanup();
  expect(cleaned).toBe(true);
  handler({ event: 'local-models-changed', id: 1, payload: snapshot(6) });
  expect(controller.state.snapshot.revision).toBe(5);
});

test('resource action failures remain beside the shared model', async () => {
  const fail = async () => {
    throw new Error('Wait until capture finishes.');
  };
  const controller = createLocalModelsController({
    stateFactory: (value) => value,
    commands: {
      prepareLocalModel: async () => snapshot(5),
      getLocalModelsState: async () => snapshot(0),
      downloadLocalModel: fail,
      cancelLocalModelDownload: fail,
      removeLocalModelDownload: fail,
    },
  });
  await controller.act('qwen3.5-2b', 'remove');
  expect(controller.state.errors['qwen3.5-2b']).toContain('Wait until capture finishes.');
  expect(controller.state.pending['qwen3.5-2b']).toBe(false);
});

test('model preparation stays pending until completion and allows retry after failure', async () => {
  let finish!: (state: LocalModelsChangedEvent) => void;
  let attempts = 0;
  const controller = createLocalModelsController({
    stateFactory: (value) => value,
    commands: {
      getLocalModelsState: async () => snapshot(0),
      downloadLocalModel: async () => snapshot(0),
      cancelLocalModelDownload: async () => snapshot(0),
      removeLocalModelDownload: async () => snapshot(0),
      prepareLocalModel: async () => {
        if (++attempts === 1) throw new Error('Could not load model. Try again.');
        return new Promise((resolve) => {
          finish = resolve;
        });
      },
    },
  });
  const model = 'isocr1-0.8b';
  expect(await controller.prepare(model)).toBe(false);
  expect(controller.state.errors[model]).toContain('Could not load model');
  const retry = controller.prepare(model);
  expect(controller.state.pending[model]).toBe(true);
  expect(controller.state.errors[model]).toBeNull();
  expect(await controller.prepare(model)).toBe(false);
  expect(attempts).toBe(2);
  finish(snapshot(3));
  expect(await retry).toBe(true);
  expect(controller.state.pending[model]).toBe(false);
  expect(controller.state.snapshot.revision).toBe(3);
});
