import { applicationConfig, modelDefaults } from '$lib/application-config';
import type {
  InferenceProviderCredentialDraft,
  InferenceProviderDraft,
  ModelSettingsDraft,
} from '$lib/page-types';
import type {
  InferenceBackend,
  InferenceBackendMetadata,
  InferenceOptionsSnapshot,
  ModelSettingsSnapshot,
  ProviderSnapshot,
  ReasoningLevel,
} from '$lib/generated/bindings';

import { DEFAULT_INFERENCE_BACKEND_METADATA } from '$lib/generated/bindings';
export { DEFAULT_INFERENCE_BACKEND_METADATA };
export const INFERENCE_BACKEND_LABELS: Record<InferenceBackend, string> =
  modelDefaults.providerLabels;

const INFERENCE_BACKENDS = new Set<InferenceBackend>([
  'local',
  'openai_api',
  'anthropic_api',
  'gemini_api',
  'openai_compatible',
]);

function asInferenceBackend(value: string): InferenceBackend | null {
  return INFERENCE_BACKENDS.has(value as InferenceBackend) ? (value as InferenceBackend) : null;
}

export type InferenceOptionsForm = {
  temperature: string;
  maxOutputTokens: string;
  requestTimeoutSeconds: string;
  reasoning: ReasoningLevel | null;
};

export function inferenceOptionsFormFromValue(
  options: InferenceOptionsSnapshot | NonNullable<ModelSettingsDraft['options']> | null | undefined,
): InferenceOptionsForm {
  return {
    temperature:
      options?.temperature === null || options?.temperature === undefined
        ? ''
        : String(options.temperature),
    maxOutputTokens:
      options?.maxOutputTokens === null || options?.maxOutputTokens === undefined
        ? ''
        : String(options.maxOutputTokens),
    requestTimeoutSeconds:
      options?.requestTimeoutSeconds === null || options?.requestTimeoutSeconds === undefined
        ? ''
        : String(options.requestTimeoutSeconds),
    reasoning: options?.reasoning ?? null,
  };
}

type NumericOption = {
  key: 'temperature' | 'maxOutputTokens' | 'requestTimeoutSeconds';
  label: string;
  min: number;
  max: number;
  integer?: boolean;
};

const NUMERIC_OPTIONS: NumericOption[] = [
  {
    key: 'temperature',
    label: 'Temperature',
    min: applicationConfig.limits.temperatureMin,
    max: applicationConfig.limits.temperatureMax,
  },
  {
    key: 'maxOutputTokens',
    label: 'Max output tokens',
    min: 1,
    max: applicationConfig.limits.maxOutputTokens,
    integer: true,
  },
  {
    key: 'requestTimeoutSeconds',
    label: 'Request timeout (seconds)',
    min: applicationConfig.limits.requestTimeoutMinSeconds,
    max: applicationConfig.limits.requestTimeoutMaxSeconds,
    integer: true,
  },
];

const NUMERIC_LITERAL = /^[+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?$/i;

function parseOptionalNumeric(value: string, option: NumericOption): number | null | string {
  if (value.trim() === '') return null;
  if (!NUMERIC_LITERAL.test(value.trim())) return `${option.label} must be a valid number.`;
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed < option.min || parsed > option.max) {
    return `${option.label} must be between ${option.min} and ${option.max}.`;
  }
  if (option.integer && !Number.isInteger(parsed)) {
    return `${option.label} must be a whole number.`;
  }
  return parsed;
}

export function inferenceOptionsFromForm(form: InferenceOptionsForm): {
  options: InferenceOptionsSnapshot | null;
  error: string | null;
} {
  const parsed = Object.fromEntries(
    NUMERIC_OPTIONS.map((option) => [option.key, parseOptionalNumeric(form[option.key], option)]),
  ) as Record<NumericOption['key'], number | null | string>;
  const invalid = Object.values(parsed).find((value): value is string => typeof value === 'string');
  if (invalid) return { options: null, error: invalid };
  const options: InferenceOptionsSnapshot = {
    temperature: parsed.temperature as number | null,
    maxOutputTokens: parsed.maxOutputTokens as number | null,
    reasoning: form.reasoning,
    requestTimeoutSeconds: parsed.requestTimeoutSeconds as number | null,
  };
  const isEmpty =
    options.temperature === null &&
    options.maxOutputTokens === null &&
    options.reasoning === null &&
    options.requestTimeoutSeconds === null;
  return { options: isEmpty ? null : options, error: null };
}

/** Refresh shared fields without replacing edits local to the canonical form. */
export function reconcileModelSettingsDraft(
  draft: ModelSettingsDraft,
  previous: ModelSettingsDraft,
  incoming: ModelSettingsDraft,
): ModelSettingsDraft {
  const draftOptions = draft.options ?? null;
  const previousOptions = previous.options ?? null;
  const incomingOptions = incoming.options ?? null;
  const reconcileOptions = (): InferenceOptionsSnapshot | null => {
    if (!draftOptions && !previousOptions && !incomingOptions) return null;
    const merged: InferenceOptionsSnapshot = {
      temperature:
        (draftOptions?.temperature ?? null) === (previousOptions?.temperature ?? null)
          ? (incomingOptions?.temperature ?? null)
          : (draftOptions?.temperature ?? null),
      maxOutputTokens:
        (draftOptions?.maxOutputTokens ?? null) === (previousOptions?.maxOutputTokens ?? null)
          ? (incomingOptions?.maxOutputTokens ?? null)
          : (draftOptions?.maxOutputTokens ?? null),
      reasoning:
        (draftOptions?.reasoning ?? null) === (previousOptions?.reasoning ?? null)
          ? (incomingOptions?.reasoning ?? null)
          : (draftOptions?.reasoning ?? null),
      requestTimeoutSeconds:
        (draftOptions?.requestTimeoutSeconds ?? null) ===
        (previousOptions?.requestTimeoutSeconds ?? null)
          ? (incomingOptions?.requestTimeoutSeconds ?? null)
          : (draftOptions?.requestTimeoutSeconds ?? null),
    };
    return merged.temperature === null &&
      merged.maxOutputTokens === null &&
      merged.reasoning === null &&
      merged.requestTimeoutSeconds === null
      ? null
      : merged;
  };
  const result: ModelSettingsDraft = {
    options: reconcileOptions(),
    provider: draft.provider === previous.provider ? incoming.provider : draft.provider,
    model: draft.model === previous.model ? incoming.model : draft.model,
  };
  return result;
}

export function backendMetadataFor(
  backend: InferenceBackend,
  metadata: InferenceBackendMetadata[],
): InferenceBackendMetadata {
  return (
    metadata.find((item) => item.id === backend) ??
    DEFAULT_INFERENCE_BACKEND_METADATA.find((item) => item.id === backend) ??
    DEFAULT_INFERENCE_BACKEND_METADATA[0]
  );
}

/**
 * A provider is connected when it needs no credential, has a stored credential,
 * or has an endpoint for a backend whose API key is optional. The policy is
 * fixed per backend, so the generated metadata is authoritative.
 */
export function providerConnected(
  provider: Pick<ProviderSnapshot, 'backend' | 'endpoint' | 'credentialRef'>,
): boolean {
  const backend = backendMetadataFor(provider.backend, DEFAULT_INFERENCE_BACKEND_METADATA);
  if (backend.credentialKind === 'none' || provider.credentialRef) return true;
  return backend.credentialOptional && Boolean(provider.endpoint?.trim());
}

/**
 * Keep the fixed provider enum visible even when no provider has credentials
 * stored yet. The config map only contributes per-backend settings.
 */
export function synthesizeInferenceProviders(
  metadata: InferenceBackendMetadata[],
  configured: ProviderSnapshot[],
): ProviderSnapshot[] {
  const configuredByBackend = new Map(configured.map((provider) => [provider.backend, provider]));
  return metadata.flatMap((item) => {
    const backend = asInferenceBackend(item.id);
    if (!backend) return [];
    const provider = configuredByBackend.get(backend);
    return {
      id: backend,
      backend,
      endpoint: provider?.endpoint ?? null,
      credentialRef: provider?.credentialRef ?? null,
    };
  });
}

export function inferenceProfileIsValid(draft: ModelSettingsDraft): boolean {
  if (!draft.provider || draft.model.trim().length === 0) {
    return false;
  }
  const options = draft.options ?? null;
  if (!options) return true;
  const temperature = options.temperature ?? null;
  const maxOutputTokens = options.maxOutputTokens ?? null;
  const requestTimeoutSeconds = options.requestTimeoutSeconds ?? null;
  return (
    (temperature === null ||
      (Number.isFinite(temperature) &&
        temperature >= applicationConfig.limits.temperatureMin &&
        temperature <= applicationConfig.limits.temperatureMax)) &&
    (maxOutputTokens === null ||
      (Number.isInteger(maxOutputTokens) &&
        maxOutputTokens > 0 &&
        maxOutputTokens <= applicationConfig.limits.maxOutputTokens)) &&
    (requestTimeoutSeconds === null ||
      (Number.isInteger(requestTimeoutSeconds) &&
        requestTimeoutSeconds >= applicationConfig.limits.requestTimeoutMinSeconds &&
        requestTimeoutSeconds <= applicationConfig.limits.requestTimeoutMaxSeconds))
  );
}

export function inferenceProviderIsValid(
  draft: InferenceProviderDraft,
  metadata: InferenceBackendMetadata[],
): boolean {
  const backend = backendMetadataFor(draft.backend, metadata);
  return !backend.supportsCustomEndpoint || Boolean(draft.endpoint?.trim());
}

export function draftForProfile(profile: ModelSettingsSnapshot): ModelSettingsDraft {
  return {
    provider: profile.provider,
    model: profile.model,
    options: profile.options ? { ...profile.options } : null,
  };
}

export function draftForProvider(provider: ProviderSnapshot): InferenceProviderDraft {
  return {
    backend: provider.backend,
    endpoint: provider.endpoint,
    credentialRef: provider.credentialRef,
  };
}

export function credentialDraftFor(
  draft: InferenceProviderDraft,
  credential: string,
): InferenceProviderCredentialDraft {
  return { ...draft, credential };
}
