import type { InferenceBackend } from '$lib/generated/bindings';

export type SaveInferenceProvider = (provider: InferenceBackend) => Promise<boolean>;

export type InferenceProviderChangeController = {
  sync(authoritativeProvider: InferenceBackend): void;
  select(provider: InferenceBackend, save: SaveInferenceProvider): Promise<boolean>;
};

/**
 * Serializes the visible provider choice against the last accepted provider.
 * Older saves cannot overwrite the result of a newer user selection.
 */
export function createInferenceProviderChangeController(
  setProvider: (provider: InferenceBackend) => void,
): InferenceProviderChangeController {
  let committedProvider: InferenceBackend = 'openai_api';
  let latestSequence = 0;

  return {
    sync(authoritativeProvider: InferenceBackend): void {
      committedProvider = authoritativeProvider;
      latestSequence += 1;
      setProvider(authoritativeProvider);
    },

    async select(provider: InferenceBackend, save: SaveInferenceProvider): Promise<boolean> {
      const sequence = ++latestSequence;
      setProvider(provider);

      let saved: boolean;
      try {
        saved = await save(provider);
      } catch {
        saved = false;
      }

      if (sequence !== latestSequence) return saved;
      if (saved) {
        committedProvider = provider;
      } else {
        setProvider(committedProvider);
      }
      return saved;
    },
  };
}
