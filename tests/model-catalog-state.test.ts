/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import type { ModelCatalogEntry, ModelCatalogSnapshot } from '../src/lib/generated/bindings';
import {
  isCatalogSuggestionEligible,
  modelCatalogStatus,
  modelMatchesCapability,
  modelsForProvider,
  projectModelCatalogSnapshot,
  modelTemperatureAvailable,
  resolveModelReasoningControls,
  sortModels,
  type InferenceModel,
} from '../src/lib/model-catalog-state';

const nativeModel = (
  partial: Partial<ModelCatalogEntry> & Pick<ModelCatalogEntry, 'id'>,
): ModelCatalogEntry => {
  const { id, ...overrides } = partial;
  return {
    id,
    name: id,
    family: null,
    reasoning: null,
    temperature: null,
    reasoningOptions: null,
    releaseDate: null,
    lastUpdated: null,
    inputModalities: [],
    outputModalities: [],
    status: null,
    ...overrides,
  };
};

const nativeCatalog = (partial: Partial<ModelCatalogSnapshot> = {}): ModelCatalogSnapshot => ({
  revision: 0,
  providers: [],
  freshness: 'fresh',
  loading: false,
  error: null,
  sourceHash: null,
  lastRefreshAt: null,
  ...partial,
});

const model = (partial: Partial<InferenceModel> & Pick<InferenceModel, 'id'>): InferenceModel => ({
  name: partial.id,
  family: null,
  reasoning: null,
  temperature: null,
  reasoningOptions: null,
  releaseDate: null,
  lastUpdated: null,
  inputModalities: null,
  outputModalities: null,
  deprecated: false,
  provider: 'openai',
  ...partial,
});

describe('model catalog state', () => {
  test('projects provider aliases and sorts release dates newest first', () => {
    const snapshot = projectModelCatalogSnapshot(
      nativeCatalog({
        sourceHash: 'catalog-v1',
        providers: [
          {
            id: 'openai_api',
            name: 'OpenAI',
            models: [
              nativeModel({ id: 'gpt-old', name: 'GPT Old', releaseDate: '2024-01-01' }),
              nativeModel({ id: 'gpt-new', name: 'GPT New', releaseDate: '2025-01-01' }),
            ],
          },
        ],
      }),
    );
    expect(snapshot.hash).toBe('catalog-v1');
    expect(snapshot.providers.openai.map((item) => item.id)).toEqual(['gpt-new', 'gpt-old']);
    expect(snapshot.providers.openai[0]?.name).toBe('GPT New');
  });

  test('projects canonical native metadata and preserves empty versus unknown controls', () => {
    const snapshot = projectModelCatalogSnapshot(
      nativeCatalog({
        sourceHash: 'native-hash',
        freshness: 'stale',
        lastRefreshAt: '2026-09-15T10:00:00Z',
        providers: [
          {
            id: 'gemini_api',
            name: 'Google',
            models: [
              nativeModel({
                id: 'gemini-vision',
                name: 'Gemini Vision',
                family: 'gemini',
                inputModalities: ['text', 'IMAGE'],
                outputModalities: ['text'],
                reasoningOptions: [],
              }),
              nativeModel({
                id: 'unknown-controls',
                reasoning: true,
                reasoningOptions: [{ type: 'unknown', name: 'future-control' }],
              }),
            ],
          },
        ],
      }),
    );
    expect(snapshot.fetchedAt).toBe('2026-09-15T10:00:00Z');
    expect(snapshot.stale).toBe(true);
    expect(snapshot.providers.google?.[0]).toMatchObject({
      inputModalities: ['text', 'image'],
      outputModalities: ['text'],
      family: 'gemini',
      reasoningOptions: [],
    });
    expect(snapshot.providers.google?.[1]?.reasoningOptions).toEqual([
      { type: 'unknown', name: 'future-control' },
    ]);
  });

  test('preserves reasoning metadata and null distinction', () => {
    const snapshot = projectModelCatalogSnapshot(
      nativeCatalog({
        providers: [
          {
            id: 'openai',
            name: 'OpenAI',
            models: [
              nativeModel({
                id: 'declared',
                reasoning: true,
                temperature: false,
                reasoningOptions: [
                  { type: 'effort', values: [null, 'low', 'default', 'unsupported'] },
                  { type: 'toggle' },
                  { type: 'budget_tokens', min: 1024, max: 8192 },
                ],
              }),
              nativeModel({ id: 'disabled', reasoning: false, reasoningOptions: [] }),
              nativeModel({ id: 'unknown' }),
            ],
          },
        ],
      }),
    );
    expect(snapshot.providers.openai?.[0]).toMatchObject({
      reasoning: true,
      temperature: false,
      reasoningOptions: [
        { type: 'effort', values: [null, 'low', 'default', 'unsupported'] },
        { type: 'toggle' },
        { type: 'budget_tokens', min: 1024, max: 8192 },
      ],
    });
    expect(snapshot.providers.openai?.[1]?.reasoningOptions).toEqual([]);
    expect(snapshot.providers.openai?.[2]?.reasoningOptions).toBeNull();
  });

  test('resolves declarative reasoning controls without inventing unknown controls', () => {
    const hidden = { visible: false, effortChoices: [], temperatureAvailable: true };
    expect(resolveModelReasoningControls(model({ id: 'off', reasoning: false }))).toEqual(hidden);
    expect(
      resolveModelReasoningControls(
        model({ id: 'no-control', reasoning: true, reasoningOptions: [] }),
      ),
    ).toEqual(hidden);
    expect(
      resolveModelReasoningControls(
        model({ id: 'unknown', reasoning: null, reasoningOptions: null }),
      ),
    ).toEqual({
      visible: true,
      effortChoices: [null, 'none', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max'],
      temperatureAvailable: true,
    });
    expect(
      resolveModelReasoningControls(
        model({
          id: 'declared',
          reasoning: true,
          reasoningOptions: [{ type: 'effort', values: [null, 'none', 'low', 'default'] }],
        }),
      ),
    ).toEqual({ visible: true, effortChoices: [null, 'none', 'low'], temperatureAvailable: true });
    expect(
      resolveModelReasoningControls(
        model({ id: 'toggle-only', reasoning: true, reasoningOptions: [{ type: 'toggle' }] }),
      ),
    ).toEqual(hidden);
    expect(
      resolveModelReasoningControls(
        model({
          id: 'unknown-control',
          reasoning: true,
          reasoningOptions: [{ type: 'unknown', name: 'future-control' }],
        }),
      ),
    ).toEqual(hidden);
    expect(modelTemperatureAvailable(model({ id: 'temperature-off', temperature: false }))).toBe(
      false,
    );
  });

  test('preserves opaque model IDs exactly, including surrounding whitespace', () => {
    const snapshot = projectModelCatalogSnapshot(
      nativeCatalog({
        providers: [
          {
            id: 'openai',
            name: 'OpenAI',
            models: [nativeModel({ id: '  model-id:v1  ', name: 'Model' })],
          },
        ],
      }),
    );
    expect(snapshot.providers.openai?.[0]?.id).toBe('  model-id:v1  ');
  });

  test('requires text and image metadata for the default vision filter', () => {
    const vision = model({
      id: 'vision',
      inputModalities: ['text', 'image'],
      outputModalities: ['text'],
    });
    const text = model({ id: 'text', inputModalities: ['text'], outputModalities: ['text'] });
    const audio = model({ id: 'audio', inputModalities: ['text'], outputModalities: ['audio'] });
    const unknown = model({ id: 'unknown' });
    expect(modelMatchesCapability(vision, 'vision-text')).toBe(true);
    expect(modelMatchesCapability(text, 'vision-text')).toBe(false);
    expect(modelMatchesCapability(unknown, 'vision-text')).toBe(false);
    expect(modelMatchesCapability(text, 'text')).toBe(true);
    expect(modelMatchesCapability(audio, 'text')).toBe(false);
    expect(modelMatchesCapability(vision, 'text-output')).toBe(true);
    expect(modelMatchesCapability(audio, 'text-output')).toBe(false);
  });

  test('keeps an existing filtered-out model selectable without hiding catalog ordering', () => {
    const snapshot = projectModelCatalogSnapshot(
      nativeCatalog({
        providers: [
          {
            id: 'openai',
            name: 'OpenAI',
            models: [
              nativeModel({
                id: 'new-vision',
                releaseDate: '2025-01-01',
                inputModalities: ['text', 'image'],
              }),
              nativeModel({
                id: 'saved-text',
                releaseDate: '2026-01-01',
                inputModalities: ['text'],
              }),
            ],
          },
        ],
      }),
    );
    expect(
      modelsForProvider(snapshot, 'openai_api', 'vision-text', 'saved-text').map((item) => item.id),
    ).toEqual(['saved-text', 'new-vision']);
    expect(sortModels(snapshot.providers.openai).map((item) => item.id)).toEqual([
      'saved-text',
      'new-vision',
    ]);
  });

  test('excludes non-language suggestions while retaining ordinary multimodal LLMs', () => {
    const snapshot = projectModelCatalogSnapshot(
      nativeCatalog({
        providers: [
          {
            id: 'google',
            name: 'Google',
            models: [
              nativeModel({
                id: 'gemini-embedding-2',
                name: 'Gemini Embedding 2',
                inputModalities: ['text', 'image'],
                outputModalities: ['text'],
              }),
              nativeModel({
                id: 'gpt-image-1',
                name: 'GPT Image 1',
                family: 'gpt-image',
                inputModalities: ['text', 'image'],
                outputModalities: ['text'],
              }),
              nativeModel({
                id: 'gemini-2.5-flash',
                family: 'gemini',
                inputModalities: ['text', 'image'],
                outputModalities: ['text'],
              }),
            ],
          },
        ],
      }),
    );
    expect(modelsForProvider(snapshot, 'google', 'vision-text').map((item) => item.id)).toEqual([
      'gemini-2.5-flash',
    ]);
  });

  test('filters generation-only variants while retaining multimodal language models', () => {
    const snapshot = projectModelCatalogSnapshot(
      nativeCatalog({
        providers: [
          {
            id: 'google',
            name: 'Google',
            models: [
              nativeModel({
                id: 'gemini-2.5-flash-image-preview',
                inputModalities: ['text', 'image'],
                outputModalities: ['text', 'image'],
              }),
              nativeModel({
                id: 'gemini-2.5-flash-preview-tts',
                inputModalities: ['text'],
                outputModalities: ['audio'],
              }),
              nativeModel({
                id: 'gemini-2.5-flash',
                inputModalities: ['text', 'image'],
                outputModalities: ['text'],
              }),
            ],
          },
        ],
      }),
    );
    expect(modelsForProvider(snapshot, 'google', 'vision-text').map((item) => item.id)).toEqual([
      'gemini-2.5-flash',
    ]);
    expect(
      modelsForProvider(snapshot, 'google', 'vision-text', 'gemini-2.5-flash-image-preview').map(
        (item) => item.id,
      ),
    ).toEqual(['gemini-2.5-flash-image-preview', 'gemini-2.5-flash']);
  });

  test('reports unavailable native freshness as offline', () => {
    const snapshot = projectModelCatalogSnapshot(nativeCatalog({ freshness: 'unavailable' }));
    expect(snapshot.offline).toBe(true);
    expect(modelCatalogStatus(snapshot)).toBe('offline');
  });

  test('filters known non-language families while retaining unknown families', () => {
    expect(isCatalogSuggestionEligible(model({ id: 'sora-2', family: 'sora' }))).toBe(false);
    expect(isCatalogSuggestionEligible(model({ id: 'gemini-live', family: 'live' }))).toBe(true);
  });
});
