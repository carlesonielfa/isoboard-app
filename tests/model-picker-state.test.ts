/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import {
  modelPickerDisplayValue,
  resolveModelPickerModel,
  modelVisibleInMode,
  localModelSuggested,
} from '../src/lib/model-picker-state';
import type { InferenceModel } from '../src/lib/model-catalog-state';

const model = (id: string, name = id): InferenceModel => ({
  id,
  name,
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
});

describe('model picker state', () => {
  test('resolves exact IDs before unique friendly names', () => {
    const models = [model('id-1', 'Friendly One'), model('Friendly One', 'Other')];
    expect(resolveModelPickerModel('Friendly One', models)).toBe('Friendly One');
    expect(resolveModelPickerModel('id-1', models)).toBe('id-1');
  });

  test('does not resolve ambiguous names and keeps exact IDs in display text', () => {
    const models = [model('one', 'Same'), model('two', 'Same')];
    expect(resolveModelPickerModel('same', models)).toBeNull();
    expect(modelPickerDisplayValue(model('id-1', 'Friendly'), 'id-1')).toBe('Friendly — id-1');
    expect(modelPickerDisplayValue(model('same', 'same'), 'same')).toBe('same');
  });
});

test('model visibility follows Basic, Advanced and Debug mode ordering', () => {
  expect(
    ['basic', 'advanced', 'debug'].map((mode) =>
      modelVisibleInMode('basic', mode as import('../src/lib/generated/bindings').UiMode),
    ),
  ).toEqual([true, true, true]);
  expect(
    ['basic', 'advanced', 'debug'].map((mode) =>
      modelVisibleInMode('advanced', mode as import('../src/lib/generated/bindings').UiMode),
    ),
  ).toEqual([false, true, true]);
  expect(
    ['basic', 'advanced', 'debug'].map((mode) =>
      modelVisibleInMode('debug', mode as import('../src/lib/generated/bindings').UiMode),
    ),
  ).toEqual([false, false, true]);
});

test('text-only stages hide extraction-only local models', () => {
  const ocr = { minimumUiMode: 'basic', supportsText: false } as const;
  const text = { minimumUiMode: 'basic', supportsText: true } as const;
  expect(localModelSuggested(ocr, 'basic', false)).toBe(true);
  expect(localModelSuggested(ocr, 'debug', true)).toBe(false);
  expect(localModelSuggested(text, 'basic', true)).toBe(true);
  expect(
    localModelSuggested({ minimumUiMode: 'advanced', supportsText: true }, 'basic', true),
  ).toBe(false);
  expect(localModelSuggested(undefined, 'basic', false)).toBe(false);
});
