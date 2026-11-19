import type { ReasoningLevel } from '$lib/generated/bindings';

/** Reasoning effort values supported by the profile protocol. */
export const INFERENCE_REASONING_EFFORTS = [
  'none',
  'minimal',
  'low',
  'medium',
  'high',
  'xhigh',
  'max',
] as const;

export type InferenceReasoningEffort = (typeof INFERENCE_REASONING_EFFORTS)[number];

/** Boolean reasoning values accepted by providers in addition to effort values. */
export const INFERENCE_REASONING_LEVELS = [
  ...INFERENCE_REASONING_EFFORTS,
  'on',
  'off',
] as const satisfies readonly ReasoningLevel[];

/** Raw effort values from models.dev; null means the provider's explicit off value. */
export type CatalogReasoningEffort = InferenceReasoningEffort | 'default' | null;

/** A profile effort choice; null means leave the provider parameter unset. */
export type ResolvedReasoningEffort = InferenceReasoningEffort | null;

const INFERENCE_REASONING_EFFORT_SET = new Set<string>(INFERENCE_REASONING_EFFORTS);

/** Normalize a catalog effort without conflating null, default, or unsupported values. */
export function normalizeCatalogReasoningEffort(
  value: unknown,
): CatalogReasoningEffort | undefined {
  if (value === null) return null;
  if (typeof value !== 'string') return undefined;
  const normalized = value.trim().toLocaleLowerCase();
  if (normalized === 'default') return 'default';
  return INFERENCE_REASONING_EFFORT_SET.has(normalized)
    ? (normalized as InferenceReasoningEffort)
    : undefined;
}
