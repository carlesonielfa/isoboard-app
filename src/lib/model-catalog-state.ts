import type {
  ModelCatalogEntry,
  ModelCatalogSnapshot,
  ModelReasoningOption as NativeModelReasoningOption,
} from '$lib/generated/bindings';
import {
  INFERENCE_REASONING_EFFORTS,
  normalizeCatalogReasoningEffort,
  type ResolvedReasoningEffort,
} from '$lib/inference-options';

export type ModelCapabilityFilter = 'vision-text' | 'text' | 'text-output' | 'all';

export { INFERENCE_REASONING_EFFORTS as MODEL_REASONING_FALLBACK_EFFORTS } from '$lib/inference-options';

/** Resolved UI choice; null means leave the provider's reasoning parameter unset. */
export type ModelReasoningEffortChoice = ResolvedReasoningEffort;

export type ModelReasoningOption = NativeModelReasoningOption;

export type ResolvedModelReasoningControls = {
  visible: boolean;
  effortChoices: ModelReasoningEffortChoice[];
  temperatureAvailable: boolean;
};

export function modelTemperatureAvailable(model: Pick<InferenceModel, 'temperature'>): boolean {
  return model.temperature !== false;
}

function resolveDeclaredEffortChoices(
  options: readonly ModelReasoningOption[],
): ModelReasoningEffortChoice[] {
  const choices: ModelReasoningEffortChoice[] = [];
  const seen = new Set<string>();
  let hasEffortOption = false;
  for (const option of options) {
    if (option.type !== 'effort') continue;
    if (!hasEffortOption) {
      hasEffortOption = true;
      choices.push(null);
      seen.add('not-set');
    }
    for (const declared of option.values) {
      const normalized = normalizeCatalogReasoningEffort(declared);
      if (normalized === undefined) continue;
      // models.dev uses null for the provider's explicit reasoning-off value.
      const choice = normalized === null ? 'none' : normalized === 'default' ? null : normalized;
      const key = choice ?? 'not-set';
      if (seen.has(key)) continue;
      seen.add(key);
      choices.push(choice);
    }
  }
  return choices;
}

/** Resolve catalog controls into the runtime's currently supported editor choices. */
export function resolveModelReasoningControls(
  model: Pick<InferenceModel, 'reasoning' | 'reasoningOptions' | 'temperature'>,
): ResolvedModelReasoningControls {
  const hidden: ResolvedModelReasoningControls = {
    visible: false,
    effortChoices: [],
    temperatureAvailable: modelTemperatureAvailable(model),
  };
  if (model.reasoning === false || model.reasoningOptions?.length === 0) return hidden;

  if (model.reasoningOptions === null) {
    return {
      visible: true,
      effortChoices: [null, ...INFERENCE_REASONING_EFFORTS],
      temperatureAvailable: modelTemperatureAvailable(model),
    };
  }

  const effortChoices = resolveDeclaredEffortChoices(model.reasoningOptions);
  return {
    // Toggle and budget metadata are deferred until the runtime can transport
    // those controls; effort metadata remains selectable.
    visible: effortChoices.length > 1,
    effortChoices,
    temperatureAvailable: modelTemperatureAvailable(model),
  };
}

export type InferenceModel = Omit<
  ModelCatalogEntry,
  | 'family'
  | 'reasoning'
  | 'temperature'
  | 'reasoningOptions'
  | 'inputModalities'
  | 'outputModalities'
  | 'status'
> & {
  family: string | null;
  reasoning: boolean | null;
  temperature: boolean | null;
  reasoningOptions: ModelReasoningOption[] | null;
  inputModalities: string[] | null;
  outputModalities: string[] | null;
  deprecated: boolean;
  provider: string | null;
};

export type InferenceModelCatalogSnapshot = {
  revision: ModelCatalogSnapshot['revision'];
  hash: string | null;
  providers: Record<string, InferenceModel[]>;
  fetchedAt: string | null;
  loading: boolean;
  error: string | null;
  stale: boolean;
  offline: boolean;
};

export type ModelCatalogStatus = 'loading' | 'ready' | 'stale' | 'offline' | 'custom';

export const EMPTY_MODEL_CATALOG: InferenceModelCatalogSnapshot = {
  revision: 0,
  hash: null,
  providers: {},
  fetchedAt: null,
  loading: false,
  error: null,
  stale: false,
  offline: false,
};

const PROVIDER_ALIASES: Record<string, string> = {
  openai_api: 'openai',
  anthropic_api: 'anthropic',
  gemini_api: 'google',
  openai_compatible: 'openai_compatible',
};

export function providerCatalogKey(value: string): string {
  return PROVIDER_ALIASES[value] ?? value;
}

function normalizeModalities(values: readonly string[]): string[] | null {
  const normalized = values.map((value) => value.trim().toLowerCase()).filter(Boolean);
  return normalized.length > 0 ? [...new Set(normalized)] : null;
}

function projectModel(value: ModelCatalogEntry, provider: string): InferenceModel {
  return {
    id: value.id,
    name: value.name,
    family: value.family ?? null,
    reasoning: value.reasoning ?? null,
    temperature: value.temperature ?? null,
    reasoningOptions:
      value.reasoningOptions === undefined || value.reasoningOptions === null
        ? null
        : value.reasoningOptions,
    releaseDate: value.releaseDate,
    lastUpdated: value.lastUpdated,
    inputModalities: normalizeModalities(value.inputModalities),
    outputModalities: normalizeModalities(value.outputModalities),
    deprecated: value.status === 'deprecated',
    provider,
  };
}

/** Project the generated native DTO into the indexed view consumed by the UI. */
export function projectModelCatalogSnapshot(
  value: ModelCatalogSnapshot,
): InferenceModelCatalogSnapshot {
  const providers: Record<string, InferenceModel[]> = {};
  for (const rawProvider of value.providers) {
    const key = providerCatalogKey(rawProvider.id);
    if (!key) continue;
    (providers[key] ??= []).push(...rawProvider.models.map((model) => projectModel(model, key)));
  }
  for (const models of Object.values(providers)) {
    const unique = new Map(models.map((model) => [model.id, model]));
    models.splice(0, models.length, ...sortModels([...unique.values()]));
  }
  return {
    revision: value.revision,
    hash: value.sourceHash,
    providers,
    fetchedAt: value.lastRefreshAt,
    loading: value.loading,
    error: value.error,
    stale: value.freshness === 'stale',
    offline: value.freshness === 'unavailable',
  };
}

function dateValue(value: string | null): number {
  if (!value) return 0;
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function sortModels(models: readonly InferenceModel[]): InferenceModel[] {
  return [...models].sort((left, right) => {
    const release = dateValue(right.releaseDate) - dateValue(left.releaseDate);
    if (release) return release;
    const updated = dateValue(right.lastUpdated) - dateValue(left.lastUpdated);
    if (updated) return updated;
    return `${left.name}\u0000${left.id}`.localeCompare(`${right.name}\u0000${right.id}`);
  });
}

function hasModality(model: InferenceModel, modality: string): boolean {
  return (
    model.inputModalities?.some((item) => item === modality || item.includes(modality)) ?? false
  );
}

function hasOutputModality(model: InferenceModel, modality: string): boolean {
  return (
    model.outputModalities?.some((item) => item === modality || item.includes(modality)) ?? false
  );
}

function hasTextOutput(model: InferenceModel): boolean {
  return (
    !model.outputModalities ||
    model.outputModalities.some((item) => item === 'text' || item.includes('text'))
  );
}

export function modelMatchesCapability(
  model: InferenceModel,
  filter: ModelCapabilityFilter,
): boolean {
  if (filter === 'all') return true;
  if (!model.inputModalities) return false;
  const text = hasModality(model, 'text');
  const image = hasModality(model, 'image') || hasModality(model, 'vision');
  if (filter === 'text-output') return text && hasTextOutput(model);
  if (filter === 'text') return text && !image && hasTextOutput(model);
  if (!text || !image) return false;
  // If output metadata exists, keep only models that can return text.
  return hasTextOutput(model);
}

const NON_LANGUAGE_MODEL_FAMILIES = new Set([
  'audio',
  'audio-generation',
  'bge',
  'classifier',
  'classification',
  'clip',
  'cross-encoder',
  'codestral-embed',
  'cohere-embed',
  'dall-e',
  'embedding',
  'embeddings',
  'encoder',
  'elevenlabs',
  'flux',
  'dream-machine',
  'dreamshaper',
  'distilbert',
  'bart',
  'gemini-embedding',
  'image',
  'image-generation',
  'gpt-image',
  'ideogram',
  'imagen',
  'image-encoder',
  'lyria',
  'm2m',
  'melotts',
  'mistral-embed',
  'nemoretriever',
  'music',
  'nano-banana',
  'rerank',
  'reranker',
  're-ranking',
  'retrieval',
  'recraft',
  'resnet',
  'runway',
  'sora',
  'speech',
  'speech-generation',
  'stable-diffusion',
  'text-embedding',
  'text-encoder',
  'titan-embed',
  'video',
  'video-generation',
  'veo',
  'voyage',
  'vision-encoder',
  'whisper',
  'indictrans',
  'plamo',
]);

function normalizedFamily(value: string): string {
  return value
    .trim()
    .toLocaleLowerCase()
    .replace(/[\s_]+/g, '-');
}

function hasNonLanguageModelToken(value: string): boolean {
  return /(?:^|[^a-z0-9])(?:embedding|embed|rerank|moderation)(?=$|[^a-z0-9])/i.test(value);
}

function hasDelimitedToken(value: string, tokens: string[]): boolean {
  const pattern = tokens.join('|');
  return new RegExp(`(?:^|[^a-z0-9])(?:${pattern})(?=$|[^a-z0-9])`, 'i').test(value);
}

function hasImageGenerationIdentifier(model: InferenceModel): boolean {
  return (
    hasOutputModality(model, 'image') &&
    hasDelimitedToken(`${model.id} ${model.name}`, ['image', 'image-gen', 'image-generation'])
  );
}

function hasTextToSpeechIdentifier(model: InferenceModel): boolean {
  return hasDelimitedToken(`${model.id} ${model.name}`, [
    'audio',
    'speech',
    'text-to-speech',
    'text2speech',
    'tts',
  ]);
}

/** Keep catalog suggestions focused on models that can generate language. */
export function isCatalogSuggestionEligible(model: InferenceModel): boolean {
  if (model.family && NON_LANGUAGE_MODEL_FAMILIES.has(normalizedFamily(model.family))) {
    return false;
  }
  if (hasNonLanguageModelToken(`${model.id} ${model.name}`)) return false;
  if (!hasTextOutput(model)) return false;
  if (hasImageGenerationIdentifier(model)) return false;
  return !(hasOutputModality(model, 'audio') && hasTextToSpeechIdentifier(model));
}

export function modelsForProvider(
  snapshot: InferenceModelCatalogSnapshot | null | undefined,
  provider: string,
  filter: ModelCapabilityFilter,
  selectedModel = '',
): InferenceModel[] {
  if (!snapshot) return [];
  const key = providerCatalogKey(provider);
  const models = sortModels(snapshot.providers[key] ?? snapshot.providers[provider] ?? []);
  const visible = models.filter(
    (model) =>
      !model.deprecated &&
      isCatalogSuggestionEligible(model) &&
      modelMatchesCapability(model, filter),
  );
  const selected = models.find((model) => model.id === selectedModel);
  if (selected && !visible.some((model) => model.id === selected.id)) visible.push(selected);
  return sortModels(visible);
}

export function providerHasCatalog(
  snapshot: InferenceModelCatalogSnapshot | null | undefined,
  provider: string,
): boolean {
  if (!snapshot) return false;
  const key = providerCatalogKey(provider);
  return Object.prototype.hasOwnProperty.call(snapshot.providers, key);
}

export function modelDisplayName(model: InferenceModel): string {
  return model.name.trim() || model.id;
}

export function modelCatalogStatus(
  snapshot: InferenceModelCatalogSnapshot | null,
  loading = false,
): ModelCatalogStatus {
  if (loading || snapshot?.loading) return 'loading';
  if (!snapshot) return 'custom';
  if (snapshot.offline) return 'offline';
  if (snapshot.stale) return 'stale';
  return Object.keys(snapshot.providers).length > 0 ? 'ready' : 'custom';
}
