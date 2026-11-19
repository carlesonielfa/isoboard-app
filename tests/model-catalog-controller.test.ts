/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import { createModelCatalogController } from '../src/lib/model-catalog-controller.svelte';
import type { ModelCatalogSnapshot } from '../src/lib/generated/bindings';

const flush = () => new Promise<void>((resolve) => setTimeout(resolve, 0));
type NativeEvent = { event: `${string}`; id: number; payload: ModelCatalogSnapshot };

describe('model catalog controller', () => {
  test('loads a snapshot and applies later native catalog events', async () => {
    let eventHandler: ((event: { payload: ModelCatalogSnapshot }) => void) | undefined;
    let unsubscribed = false;
    const controller = createModelCatalogController({
      availability: { desktopAvailable: true },
      bridge: {
        getInferenceModelCatalog: async () => ({
          revision: 0,
          sourceHash: 'a',
          freshness: 'fresh',
          loading: false,
          error: null,
          lastRefreshAt: null,
          providers: [
            {
              id: 'openai',
              name: 'OpenAI',
              models: [
                {
                  id: 'gpt-5',
                  name: 'GPT-5',
                  family: null,
                  reasoning: null,
                  temperature: null,
                  reasoningOptions: null,
                  releaseDate: null,
                  lastUpdated: null,
                  inputModalities: [],
                  outputModalities: [],
                  status: null,
                },
              ],
            },
          ],
        }),
      },
      events: {
        inferenceModelCatalogChanged: {
          listen: async (handler: (event: NativeEvent) => void) => {
            eventHandler = (event) =>
              handler({ event: 'inference-model-catalog-changed', id: 1, payload: event.payload });
            return () => {
              unsubscribed = true;
            };
          },
        },
      },
      stateFactory: (value) => value,
    });

    controller.mount();
    await flush();
    expect(controller.state.status).toBe('ready');
    expect(controller.state.snapshot?.hash).toBe('a');
    expect(controller.state.snapshot?.providers.openai?.[0]?.id).toBe('gpt-5');

    eventHandler?.({
      payload: {
        revision: 1,
        sourceHash: 'b',
        freshness: 'stale',
        loading: false,
        error: null,
        lastRefreshAt: null,
        providers: [
          {
            id: 'openai',
            name: 'OpenAI',
            models: [
              {
                id: 'gpt-4',
                name: 'GPT-4',
                family: null,
                reasoning: null,
                temperature: null,
                reasoningOptions: null,
                releaseDate: null,
                lastUpdated: null,
                inputModalities: ['text'],
                outputModalities: ['text'],
                status: null,
              },
            ],
          },
        ],
      },
    });
    expect(controller.state.status).toBe('stale');
    expect(controller.state.snapshot?.hash).toBe('b');

    controller.dispose();
    expect(unsubscribed).toBe(true);
  });

  test('keeps browser mode non-blocking and marks the catalog offline', () => {
    const controller = createModelCatalogController({
      availability: { desktopAvailable: false },
      stateFactory: (value) => value,
    });

    controller.mount();

    expect(controller.state.status).toBe('offline');
    expect(controller.state.snapshot?.offline).toBe(true);
  });

  test('rejects an older initial response after a newer catalog event', async () => {
    let eventHandler: ((event: { payload: ModelCatalogSnapshot }) => void) | undefined;
    let resolveInitial!: (value: ModelCatalogSnapshot) => void;
    const controller = createModelCatalogController({
      availability: { desktopAvailable: true },
      bridge: {
        getInferenceModelCatalog: () => new Promise((resolve) => (resolveInitial = resolve)),
      },
      events: {
        inferenceModelCatalogChanged: {
          listen: async (handler: (event: NativeEvent) => void) => {
            eventHandler = (event) =>
              handler({ event: 'inference-model-catalog-changed', id: 1, payload: event.payload });
            return () => undefined;
          },
        },
      },
      stateFactory: (value) => value,
    });

    controller.mount();
    await flush();
    eventHandler?.({
      payload: {
        revision: 2,
        sourceHash: null,
        freshness: 'fresh',
        loading: false,
        error: null,
        lastRefreshAt: null,
        providers: [
          {
            id: 'openai',
            name: 'OpenAI',
            models: [
              {
                id: 'new',
                name: 'New',
                family: null,
                reasoning: null,
                temperature: null,
                reasoningOptions: null,
                releaseDate: null,
                lastUpdated: null,
                inputModalities: ['text'],
                outputModalities: ['text'],
                status: null,
              },
            ],
          },
        ],
      },
    });
    resolveInitial({
      revision: 1,
      sourceHash: null,
      freshness: 'fresh',
      loading: false,
      error: null,
      lastRefreshAt: null,
      providers: [
        {
          id: 'openai',
          name: 'OpenAI',
          models: [
            {
              id: 'old',
              name: 'Old',
              family: null,
              reasoning: null,
              temperature: null,
              reasoningOptions: null,
              releaseDate: null,
              lastUpdated: null,
              inputModalities: ['text'],
              outputModalities: ['text'],
              status: null,
            },
          ],
        },
      ],
    });
    await flush();

    expect(controller.state.snapshot?.revision).toBe(2);
    expect(controller.state.snapshot?.providers.openai?.[0]?.id).toBe('new');
    controller.dispose();
  });
});
