/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import {
  inferenceProfileIsValid,
  synthesizeInferenceProviders,
  reconcileModelSettingsDraft,
  inferenceOptionsFromForm,
  inferenceOptionsFormFromValue,
} from '../src/lib/inference-profile-state';
import { DEFAULT_INFERENCE_BACKEND_METADATA } from '../src/lib/inference-profile-state';

describe('inference profile state', () => {
  test('refreshes a shared model without overwriting a local name or provider edit', () => {
    const previous = {
      provider: 'openai_api' as const,
      model: 'old-model',
      options: null,
    };
    const local = { ...previous, provider: 'anthropic_api' as const };
    expect(
      reconcileModelSettingsDraft(local, previous, {
        ...previous,

        model: 'external-model',
        options: null,
      }),
    ).toEqual({ ...local, model: 'external-model' });
  });
  test('keeps incomplete drafts local until required fields are present', () => {
    expect(
      inferenceProfileIsValid({
        provider: 'openai_api',
        model: '',
        options: null,
      }),
    ).toBe(false);
    expect(
      inferenceProfileIsValid({
        provider: 'openai_api',
        model: 'vision',
        options: null,
      }),
    ).toBe(true);
    expect(
      inferenceProfileIsValid({
        provider: 'openai_api',
        model: 'vision',
        options: null,
      }),
    ).toBe(true);
  });

  test('round-trips provider defaults and rejects incomplete numeric drafts without coercion', () => {
    const form = inferenceOptionsFormFromValue({
      temperature: 0.25,
      maxOutputTokens: 512,
      reasoning: 'medium',
      requestTimeoutSeconds: null,
    });
    expect(inferenceOptionsFromForm(form)).toEqual({
      options: {
        temperature: 0.25,
        maxOutputTokens: 512,
        reasoning: 'medium',
        requestTimeoutSeconds: null,
      },
      error: null,
    });
    expect(
      inferenceOptionsFromForm({
        ...form,
        temperature: '1e',
      }),
    ).toEqual({ options: null, error: 'Temperature must be a valid number.' });
  });

  test('reconciles an externally changed option without overwriting a different local option edit', () => {
    const previous = {
      provider: 'openai_api' as const,
      model: 'model',
      options: {
        temperature: 0.2,
        maxOutputTokens: 256,
        reasoning: null,
        requestTimeoutSeconds: 120,
      },
    };
    const local = {
      ...previous,
      options: { ...previous.options, temperature: 0.9 },
    };
    expect(
      reconcileModelSettingsDraft(local, previous, {
        ...previous,
        options: { ...previous.options, maxOutputTokens: 1024 },
      }),
    ).toEqual({
      ...local,
      options: { ...local.options, maxOutputTokens: 1024 },
    });

    const baselineWithoutOptions = { ...previous, options: null };
    const localTemperature = {
      ...baselineWithoutOptions,
      options: {
        temperature: 0.9,
        maxOutputTokens: null,
        reasoning: null,
        requestTimeoutSeconds: null,
      },
    };
    expect(
      reconcileModelSettingsDraft(localTemperature, baselineWithoutOptions, {
        ...baselineWithoutOptions,
        options: {
          temperature: null,
          maxOutputTokens: 2048,
          reasoning: null,
          requestTimeoutSeconds: null,
        },
      }),
    ).toEqual({
      ...localTemperature,
      options: {
        temperature: 0.9,
        maxOutputTokens: 2048,
        reasoning: null,
        requestTimeoutSeconds: null,
      },
    });
  });

  test('synthesizes fixed providers when the config map is empty', () => {
    const providers = synthesizeInferenceProviders(DEFAULT_INFERENCE_BACKEND_METADATA, []);

    expect(providers.map((provider) => provider.backend)).toEqual([
      'openai_api',
      'anthropic_api',
      'gemini_api',
      'openai_compatible',
      'local',
    ]);
    expect(
      providers.every((provider) => provider.endpoint === null && provider.credentialRef === null),
    ).toBe(true);
  });

  test('overlays configured endpoint and credential on the fixed provider row', () => {
    const providers = synthesizeInferenceProviders(DEFAULT_INFERENCE_BACKEND_METADATA, [
      {
        id: 'openai_compatible',
        backend: 'openai_compatible',
        endpoint: 'https://example.com/v1',
        credentialRef: 'a'.repeat(32),
      },
    ]);

    expect(providers.find((provider) => provider.backend === 'openai_compatible')).toEqual({
      id: 'openai_compatible',
      backend: 'openai_compatible',
      endpoint: 'https://example.com/v1',
      credentialRef: 'a'.repeat(32),
    });
  });
});
