import { applicationConfig } from '$lib/application-config';
import type {
  InferenceCredentialReadinessSnapshot,
  ConversionSettingsApplicationMode,
  ModelSettingsSnapshot,
  LocalModelSnapshot,
  ProcessingConversionSettings,
  ProcessingProfileSnapshot,
  ProviderSnapshot,
} from '$lib/generated/bindings';
import {
  draftForProfile,
  inferenceProfileIsValid,
  providerConnected,
} from '$lib/inference-profile-state';
import type { ProcessingProfileDraft } from '$lib/page-types';

export const DEFAULT_PROCESSING_CONVERSION_SETTINGS: ProcessingConversionSettings = {
  tables: true,
  colors: true,
  images: true,
};

export const PROCESSING_APPLICATION_MODES: ConversionSettingsApplicationMode[] = [
  'prompt',
  'html_cleanup',
  'prompt_and_html_cleanup',
];

export const ADDITIONAL_RULES_MAX_BYTES = applicationConfig.limits.additionalRulesBytes;
export const PROMPT_TEMPLATE_MAX_BYTES = applicationConfig.limits.promptTemplateBytes;
const CONTROL_CHARACTER = /\p{Cc}/u;

export function additionalRulesAreValid(value: unknown): value is string {
  if (typeof value !== 'string') return false;
  if (new TextEncoder().encode(value).byteLength > ADDITIONAL_RULES_MAX_BYTES) return false;
  return !Array.from(value).some(
    (character) =>
      CONTROL_CHARACTER.test(character) &&
      character !== '\n' &&
      character !== '\r' &&
      character !== '\t',
  );
}

export function promptTemplateOverrideFromValue(
  value: string,
  defaultValue: string,
): string | null {
  return value === defaultValue ? null : value;
}

export function promptTemplateOverrideIsValid(value: unknown): value is string | null {
  if (value === null) return true;
  if (typeof value !== 'string' || value.length === 0) return false;
  if (new TextEncoder().encode(value).byteLength > PROMPT_TEMPLATE_MAX_BYTES) return false;
  return !Array.from(value).some(
    (character) =>
      CONTROL_CHARACTER.test(character) &&
      character !== '\n' &&
      character !== '\r' &&
      character !== '\t',
  );
}

export function defaultProcessingProfileDraft(name = 'Default'): ProcessingProfileDraft {
  return {
    name,
    modelSettings: null,
    rulesModelSettings: null,
    rulesModelMode: 'same',
    conversionPromptTemplate: null,
    postprocessingPromptTemplate: null,
    additionalRules: '',
    additionalRulesSeparatePass: false,
    applicationModeOverride: null,
    conversion: { ...DEFAULT_PROCESSING_CONVERSION_SETTINGS },
  };
}

export function processingProfileSnapshotFromDraft(
  id: string,
  draft: ProcessingProfileDraft = defaultProcessingProfileDraft(),
): ProcessingProfileSnapshot {
  return { id, ...draft };
}

/** Convert a saved summary to the complete payload used by atomic edits. */
export function processingProfileDraftFromSnapshot(
  profile: ProcessingProfileSnapshot,
): ProcessingProfileDraft {
  return {
    name: profile.name,
    modelSettings: profile.modelSettings ? draftForProfile(profile.modelSettings) : null,
    rulesModelSettings: profile.rulesModelSettings
      ? draftForProfile(profile.rulesModelSettings)
      : null,
    rulesModelMode: profile.rulesModelMode,
    conversionPromptTemplate: profile.conversionPromptTemplate,
    postprocessingPromptTemplate: profile.postprocessingPromptTemplate,
    additionalRules: profile.additionalRules,
    additionalRulesSeparatePass: profile.additionalRulesSeparatePass,
    applicationModeOverride: profile.applicationModeOverride,
    conversion: { ...profile.conversion },
  };
}

/** Refresh saved fields without replacing edits still local to the open form. */
export function reconcileProcessingProfileDraft(
  draft: ProcessingProfileDraft,
  previous: ProcessingProfileDraft,
  incoming: ProcessingProfileDraft,
): ProcessingProfileDraft {
  return {
    name: draft.name === previous.name ? incoming.name : draft.name,
    modelSettings:
      JSON.stringify(draft.modelSettings) === JSON.stringify(previous.modelSettings)
        ? incoming.modelSettings
        : draft.modelSettings,
    rulesModelSettings:
      JSON.stringify(draft.rulesModelSettings) === JSON.stringify(previous.rulesModelSettings)
        ? incoming.rulesModelSettings
        : draft.rulesModelSettings,
    rulesModelMode:
      draft.rulesModelMode === previous.rulesModelMode
        ? incoming.rulesModelMode
        : draft.rulesModelMode,
    conversionPromptTemplate:
      draft.conversionPromptTemplate === previous.conversionPromptTemplate
        ? incoming.conversionPromptTemplate
        : draft.conversionPromptTemplate,
    postprocessingPromptTemplate:
      draft.postprocessingPromptTemplate === previous.postprocessingPromptTemplate
        ? incoming.postprocessingPromptTemplate
        : draft.postprocessingPromptTemplate,
    additionalRules:
      draft.additionalRules === previous.additionalRules
        ? incoming.additionalRules
        : draft.additionalRules,
    additionalRulesSeparatePass:
      draft.additionalRulesSeparatePass === previous.additionalRulesSeparatePass
        ? incoming.additionalRulesSeparatePass
        : draft.additionalRulesSeparatePass,
    applicationModeOverride:
      draft.applicationModeOverride === previous.applicationModeOverride
        ? incoming.applicationModeOverride
        : draft.applicationModeOverride,
    conversion: {
      tables:
        draft.conversion.tables === previous.conversion.tables
          ? incoming.conversion.tables
          : draft.conversion.tables,
      colors:
        draft.conversion.colors === previous.conversion.colors
          ? incoming.conversion.colors
          : draft.conversion.colors,
      images:
        draft.conversion.images === previous.conversion.images
          ? incoming.conversion.images
          : draft.conversion.images,
    },
  };
}

export function processingProfileUsesSeparatePass(draft: ProcessingProfileDraft): boolean {
  return draft.additionalRulesSeparatePass && Boolean(draft.additionalRules.trim());
}

export type ProfileIssue = {
  label: string;
  message: string;
  target: 'model' | 'rules' | 'conversion' | 'providers';
  action: string;
  checking?: boolean;
};

type LocalReadiness = ReadonlyArray<Pick<LocalModelSnapshot, 'id' | 'downloaded' | 'supportsText'>>;
export type CredentialReadiness = Record<string, InferenceCredentialReadinessSnapshot>;

/** Assess only stages used by this capture. Catalog suggestions never invalidate remote IDs. */
export function assessProfile(
  draft: ProcessingProfileDraft,
  providers: ProviderSnapshot[],
  localModels?: LocalReadiness,
  credentials?: CredentialReadiness,
): ProfileIssue | null {
  const stages = [draft.modelSettings];
  if (processingProfileUsesSeparatePass(draft))
    stages.push(draft.rulesModelMode === 'same' ? draft.modelSettings : draft.rulesModelSettings);
  for (const [index, settings] of stages.entries()) {
    const label = index === 0 ? 'Capture' : 'Rules';
    const target = index === 0 ? 'model' : 'rules';
    const action = index === 0 ? 'Choose model' : 'Review rules';
    if (!settings?.model.trim())
      return {
        label: `${label} model required`,
        message: `Choose a model for ${label.toLowerCase()}.`,
        target,
        action,
      };
    if (settings.provider === 'local') {
      const snapshot = settings as ModelSettingsSnapshot;
      const localModel = localModels?.find((model) => model.id === settings.model);
      const downloaded = localModels
        ? (localModel?.downloaded ?? false)
        : snapshot.localModelDownloaded;
      const available = localModels ? Boolean(localModel) : snapshot.localModelDownloadAvailable;
      const supportsText = localModels ? localModel?.supportsText : snapshot.localModelSupportsText;
      if (downloaded === false && available === false)
        return {
          label: `${label} model unavailable`,
          message: `${label} model is unavailable.`,
          target,
          action,
        };
      if (credentials && downloaded !== true && available !== true)
        return {
          label: 'Checking model…',
          message: `Checking availability of the ${label.toLowerCase()} model.`,
          target,
          action,
          checking: true,
        };
      if (index === 0 && supportsText === false) {
        if (draft.additionalRules.trim() && !draft.additionalRulesSeparatePass)
          return {
            label: 'Rules model required',
            message: 'Choose a separate rules model to apply additional rules.',
            target: 'rules',
            action: 'Review rules',
          };
        if (
          !draft.conversionPromptTemplate &&
          draft.applicationModeOverride === 'prompt' &&
          Object.values(draft.conversion).some((value) => !value)
        )
          return {
            label: 'Review conversion',
            message: 'Use cleanup to apply conversion options with this model.',
            target: 'conversion',
            action: 'Review conversion',
          };
      }
      if (index === 1 && supportsText === false)
        return {
          label: 'Rules model incompatible',
          message: 'Choose a text-capable rules model.',
          target,
          action,
        };
    } else {
      const provider = providers.find(
        (p) => p.backend === settings.provider && providerConnected(p),
      );
      if (!provider)
        return {
          label: 'Connect provider',
          message: `${label} model needs a connection in Providers before capture.`,
          target: 'providers',
          action: 'Open Providers',
        };
      if (credentials) {
        const access = credentials[provider.id] ?? credentials[provider.backend];
        // `unknown` means saved but not yet read from the Keychain this session;
        // the first capture reads it, so it does not block capture.
        if (
          !access ||
          access.provider !== provider.backend ||
          access.credentialRef !== provider.credentialRef
        )
          return {
            label: 'Checking provider…',
            message: `Checking provider access for the ${label.toLowerCase()} model.`,
            target: 'providers',
            action: 'Open Providers',
            checking: true,
          };
        if (access.readiness !== 'ready' && access.readiness !== 'unknown')
          return {
            label: 'Check provider access',
            message: `${label} model provider access is ${access.readiness}. Check the connection in Providers.`,
            target: 'providers',
            action: 'Open Providers',
          };
      }
    }
  }
  return null;
}

/** Setup keeps its existing configuration-only policy, independently of live access checks. */
export function processingProfileReferenceReadinessNotice(
  draft: ProcessingProfileDraft,
  providers: ProviderSnapshot[],
  localModels?: LocalReadiness,
): string | null {
  return assessProfile(draft, providers, localModels)?.message ?? null;
}

export function processingProfileDraftValidationError(
  draft: ProcessingProfileDraft,
): string | null {
  if (!draft.name.trim()) return 'Enter a profile name.';
  if (!additionalRulesAreValid(draft.additionalRules))
    return `Additional Rules must be at most ${ADDITIONAL_RULES_MAX_BYTES / 1024} KiB of UTF-8 text, with no control characters except line breaks and tabs.`;
  if (
    !promptTemplateOverrideIsValid(draft.conversionPromptTemplate) ||
    !promptTemplateOverrideIsValid(draft.postprocessingPromptTemplate)
  )
    return `Prompt templates must be non-empty and at most ${PROMPT_TEMPLATE_MAX_BYTES / 1024} KiB of UTF-8 text, with no control characters except line breaks and tabs.`;
  return processingProfileDraftIsValid(draft) ? null : 'Check the profile settings.';
}

/** Keep multiline rules intact while treating whitespace-only input as empty. */
export function normalizeProcessingProfileDraft(
  draft: ProcessingProfileDraft,
): ProcessingProfileDraft {
  return {
    ...draft,
    name: draft.name.trim(),
    additionalRules: draft.additionalRules.trim() ? draft.additionalRules : '',
    modelSettings: draft.modelSettings
      ? draftForProfile({ ...draft.modelSettings, options: draft.modelSettings.options ?? null })
      : null,
    rulesModelSettings: draft.rulesModelSettings
      ? draftForProfile({
          ...draft.rulesModelSettings,
          options: draft.rulesModelSettings.options ?? null,
        })
      : null,
    conversionPromptTemplate: draft.conversionPromptTemplate,
    postprocessingPromptTemplate: draft.postprocessingPromptTemplate,
    applicationModeOverride: draft.applicationModeOverride,
    conversion: {
      tables: draft.conversion.tables,
      colors: draft.conversion.colors,
      images: draft.conversion.images,
    },
  };
}

export function processingProfileDraftIsValid(draft: ProcessingProfileDraft): boolean {
  return (
    typeof draft === 'object' &&
    draft !== null &&
    typeof draft.name === 'string' &&
    draft.name.trim().length > 0 &&
    additionalRulesAreValid(draft.additionalRules) &&
    typeof draft.additionalRulesSeparatePass === 'boolean' &&
    ['same', 'independent'].includes(draft.rulesModelMode) &&
    (draft.modelSettings === null || inferenceProfileIsValid(draft.modelSettings)) &&
    (draft.rulesModelSettings === null || inferenceProfileIsValid(draft.rulesModelSettings)) &&
    promptTemplateOverrideIsValid(draft.conversionPromptTemplate) &&
    promptTemplateOverrideIsValid(draft.postprocessingPromptTemplate) &&
    (draft.applicationModeOverride === null ||
      PROCESSING_APPLICATION_MODES.includes(draft.applicationModeOverride)) &&
    typeof draft.conversion?.tables === 'boolean' &&
    typeof draft.conversion?.colors === 'boolean' &&
    typeof draft.conversion?.images === 'boolean'
  );
}

export function shouldApplyProfileSnapshot(
  incomingRevision: number | undefined,
  currentRevision: number,
): boolean {
  return incomingRevision === undefined
    ? currentRevision === 0
    : incomingRevision >= currentRevision;
}

/** Choose the first unused display name for a newly created profile. */
export function nextProcessingProfileName(profiles: ProcessingProfileSnapshot[]): string {
  const names = new Set(profiles.map((profile) => profile.name.trim().toLowerCase()));
  const baseName = 'New profile';
  if (!names.has(baseName.toLowerCase())) return baseName;

  let suffix = 2;
  while (names.has(`${baseName} ${suffix}`.toLowerCase())) suffix += 1;
  return `${baseName} ${suffix}`;
}
