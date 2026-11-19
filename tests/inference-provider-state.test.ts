import { describe, expect, test } from 'bun:test';
import type { InferenceBackend } from '../src/lib/generated/bindings';
import { createInferenceProviderChangeController } from '../src/lib/inference-provider-state';

describe('inference provider change controller', () => {
  test('restores the committed provider when persistence fails', async () => {
    let selectedProvider: InferenceBackend = 'openai_api';
    const controller = createInferenceProviderChangeController((provider) => {
      selectedProvider = provider;
    });

    controller.sync(selectedProvider);
    const saved = await controller.select('anthropic_api', async () => false);

    expect(saved).toBe(false);
    expect(selectedProvider).toBe('openai_api');
  });

  test('restores the committed provider when persistence rejects', async () => {
    let selectedProvider: InferenceBackend = 'openai_api';
    const controller = createInferenceProviderChangeController((provider) => {
      selectedProvider = provider;
    });

    controller.sync(selectedProvider);
    const saved = await controller.select('anthropic_api', async () => {
      throw new Error('save failed');
    });

    expect(saved).toBe(false);
    expect(selectedProvider).toBe('openai_api');
  });

  test('keeps a newer return selection after an older save completes', async () => {
    let selectedProvider: InferenceBackend = 'openai_api';
    const persistedProviders: InferenceBackend[] = [];
    let resolveFirst: (() => void) | undefined;
    let resolveSecond: (() => void) | undefined;
    const controller = createInferenceProviderChangeController((provider) => {
      selectedProvider = provider;
    });

    controller.sync(selectedProvider);
    const first = controller.select('anthropic_api', () => {
      persistedProviders.push('anthropic_api');
      return new Promise<boolean>((resolve) => (resolveFirst = () => resolve(true)));
    });
    const second = controller.select('openai_api', () => {
      persistedProviders.push('openai_api');
      return new Promise<boolean>((resolve) => (resolveSecond = () => resolve(true)));
    });

    expect(persistedProviders).toEqual(['anthropic_api', 'openai_api']);
    resolveFirst?.();
    await first;
    expect(selectedProvider).toBe('openai_api');

    resolveSecond?.();
    await second;
    expect(selectedProvider).toBe('openai_api');
  });
});
