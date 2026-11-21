import type { LocalModelSnapshot, UiMode } from '$lib/generated/bindings';
import { modelDisplayName, type InferenceModel } from '$lib/model-catalog-state';

/** Resolve a visible picker label to the exact opaque ID it represents. */
export function resolveModelPickerModel(
  query: string,
  models: readonly InferenceModel[],
): string | null {
  const exactId = models.find((model) => model.id === query);
  if (exactId) return exactId.id;

  const normalizedName = query.trim().toLocaleLowerCase();
  if (!normalizedName) return null;
  const nameMatches = models.filter(
    (model) => modelDisplayName(model).toLocaleLowerCase() === normalizedName,
  );
  return nameMatches.length === 1 ? nameMatches[0].id : null;
}

export function modelPickerDisplayValue(
  model: InferenceModel | undefined,
  fallbackId: string,
): string {
  if (!model) return fallbackId;
  const name = modelDisplayName(model);
  return name === model.id ? name : `${name} — ${model.id}`;
}

/** Minimum mode affects picker suggestions, not saved model identities or readiness. */
export function modelVisibleInMode(minimum: UiMode, current: UiMode): boolean {
  const rank: Record<UiMode, number> = { basic: 0, advanced: 1, debug: 2 };
  return rank[current] >= rank[minimum];
}

/**
 * Local picker suggestions respect the UI mode and, for text-only stages such
 * as the independent rules pass, hide extraction-only models.
 */
export function localModelSuggested(
  model: Pick<LocalModelSnapshot, 'minimumUiMode' | 'supportsText'> | undefined,
  uiMode: UiMode,
  requireTextSupport: boolean,
): boolean {
  if (requireTextSupport && model?.supportsText === false) return false;
  return modelVisibleInMode(model?.minimumUiMode ?? 'debug', uiMode);
}
