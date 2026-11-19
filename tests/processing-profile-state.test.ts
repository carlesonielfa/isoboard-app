/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import {
  assessProfile,
  defaultProcessingProfileDraft,
  additionalRulesAreValid,
  normalizeProcessingProfileDraft,
  nextProcessingProfileName,
  processingProfileDraftIsValid,
  processingProfileSnapshotFromDraft,
  reconcileProcessingProfileDraft,
  processingProfileDraftValidationError,
  shouldApplyProfileSnapshot,
  promptTemplateOverrideFromValue,
  processingProfileDraftFromSnapshot,
  processingProfileReferenceReadinessNotice,
} from '../src/lib/processing-profile-state';

describe('profile snapshot state', () => {
  test('carries nullable prompt overrides through drafts and reconciles them independently', () => {
    const previous = {
      ...defaultProcessingProfileDraft('Saved'),
      conversionPromptTemplate: null,
      postprocessingPromptTemplate: 'custom post template',
    };
    const local = {
      ...previous,
      conversionPromptTemplate: 'custom conversion template',
    };
    const incoming = {
      ...previous,
      postprocessingPromptTemplate: 'refreshed post template',
    };

    expect(reconcileProcessingProfileDraft(local, previous, incoming)).toMatchObject({
      conversionPromptTemplate: 'custom conversion template',
      postprocessingPromptTemplate: 'refreshed post template',
    });
    expect(
      normalizeProcessingProfileDraft({
        ...local,
        conversionPromptTemplate: null,
        postprocessingPromptTemplate: null,
      }),
    ).toMatchObject({ conversionPromptTemplate: null, postprocessingPromptTemplate: null });
    expect(processingProfileDraftIsValid({ ...local, conversionPromptTemplate: null })).toBe(true);
    expect(promptTemplateOverrideFromValue('default template', 'default template')).toBeNull();
    expect(promptTemplateOverrideFromValue('default template\n', 'default template')).toBe(
      'default template\n',
    );
    expect(
      processingProfileDraftValidationError({ ...local, postprocessingPromptTemplate: '' }),
    ).toContain('Prompt templates');
  });

  test('keeps separate-pass assignments optional to save and conditional for capture readiness', () => {
    const draft = {
      ...defaultProcessingProfileDraft(),
      modelSettings: { provider: 'openai_api' as const, model: 'vision', options: null },
      additionalRulesSeparatePass: true,
      additionalRules: ' \n\t ',
      rulesModelSettings: { provider: 'openai_api' as const, model: 'reserved', options: null },
    };
    expect(processingProfileDraftValidationError(draft)).toBeNull();
    const meaningful = { ...draft, additionalRules: 'Translate prose.' };
    expect(processingProfileDraftValidationError(meaningful)).toBeNull();
    expect(
      processingProfileDraftValidationError({ ...draft, additionalRules: 'é'.repeat(32769) }),
    ).toContain('64 KiB');
    expect(processingProfileDraftValidationError({ ...draft, additionalRules: ' \v ' })).toContain(
      'control characters',
    );
  });

  test('provides complete defaults and preserves meaningful multiline rules', () => {
    const draft = defaultProcessingProfileDraft('New profile');
    expect(draft).toEqual({
      name: 'New profile',
      modelSettings: null,
      rulesModelSettings: null,
      rulesModelMode: 'same',
      conversionPromptTemplate: null,
      postprocessingPromptTemplate: null,
      additionalRules: '',
      additionalRulesSeparatePass: false,
      applicationModeOverride: null,
      conversion: { tables: true, colors: true, images: true },
    });

    const rules = 'Translate prose to Spanish;\nPreserve code and URLs.';
    expect(normalizeProcessingProfileDraft({ ...draft, additionalRules: rules })).toMatchObject({
      additionalRules: rules,
    });
    expect(normalizeProcessingProfileDraft({ ...draft, additionalRules: ' \n\t ' })).toMatchObject({
      additionalRules: '',
    });
  });

  test('rejects invalid explicit modes and malformed conversion values', () => {
    const draft = defaultProcessingProfileDraft();
    expect(processingProfileDraftIsValid(draft)).toBe(true);
    expect(
      processingProfileDraftIsValid({
        ...draft,
        applicationModeOverride: 'invalid' as never,
      }),
    ).toBe(false);
    expect(
      processingProfileDraftIsValid({
        ...draft,
        conversion: { ...draft.conversion, tables: 'yes' as never },
      }),
    ).toBe(false);
  });

  test('validates raw additional rules before whitespace normalization', () => {
    expect(additionalRulesAreValid(' \n\r\t ')).toBe(true);
    expect(additionalRulesAreValid(' \v ')).toBe(false);
    expect(additionalRulesAreValid('é'.repeat(32769))).toBe(false);
    expect(additionalRulesAreValid('é'.repeat(32768))).toBe(true);
  });

  test('rejects an older revision while accepting the same or newer revision', () => {
    expect(shouldApplyProfileSnapshot(4, 5)).toBe(false);
    expect(shouldApplyProfileSnapshot(5, 5)).toBe(true);
    expect(shouldApplyProfileSnapshot(6, 5)).toBe(true);
    expect(shouldApplyProfileSnapshot(undefined, 0)).toBe(true);
    expect(shouldApplyProfileSnapshot(undefined, 5)).toBe(false);
  });

  test('generates unique default names across repeated creates and renames', () => {
    let profiles = [processingProfileSnapshotFromDraft('default')];

    const firstName = nextProcessingProfileName(profiles);
    expect(firstName).toBe('New profile');
    profiles = [
      ...profiles,
      processingProfileSnapshotFromDraft('profile-1', defaultProcessingProfileDraft(firstName)),
    ];

    const secondName = nextProcessingProfileName(profiles);
    expect(secondName).toBe('New profile 2');
    profiles = [
      ...profiles,
      processingProfileSnapshotFromDraft('profile-2', defaultProcessingProfileDraft(secondName)),
    ];

    profiles = profiles.map((profile) =>
      profile.id === 'profile-1' ? { ...profile, name: 'New profile 3' } : profile,
    );
    expect(nextProcessingProfileName(profiles)).toBe('New profile');

    profiles = [
      ...profiles,
      processingProfileSnapshotFromDraft('profile-3', defaultProcessingProfileDraft('New profile')),
    ];
    expect(nextProcessingProfileName(profiles)).toBe('New profile 4');
  });
});

test('copies owned model settings without snapshot metadata or shared option objects', () => {
  const profile = processingProfileSnapshotFromDraft('original');
  profile.modelSettings = {
    provider: 'local',
    model: 'qwen3.5-2b',
    options: {
      temperature: 0.2,
      maxOutputTokens: null,
      reasoning: null,
      requestTimeoutSeconds: null,
    },
    localModelDownloaded: true,
  };
  profile.rulesModelSettings = { ...profile.modelSettings, model: 'remembered' };
  profile.rulesModelMode = 'same';
  const copy = normalizeProcessingProfileDraft(profile);
  expect(copy.rulesModelSettings?.model).toBe('remembered');
  expect(copy.rulesModelMode).toBe('same');
  expect(copy.modelSettings).not.toHaveProperty('localModelDownloaded');
  copy.modelSettings!.options!.temperature = 1;
  expect(profile.modelSettings.options!.temperature).toBe(0.2);
});

test('editor readiness follows live models without persisting metadata or checking inactive rules', () => {
  const profile = processingProfileSnapshotFromDraft('original');
  profile.modelSettings = {
    provider: 'local',
    model: 'text-model',
    options: null,
    localModelSupportsText: true,
    localModelDownloaded: true,
  };
  profile.rulesModelSettings = { provider: 'local', model: 'ocr-model', options: null };
  profile.additionalRules = 'Translate to Spanish';
  profile.additionalRulesSeparatePass = true;
  const draft = processingProfileDraftFromSnapshot(profile);
  const models = [
    { id: 'text-model', supportsText: true, downloaded: true },
    { id: 'ocr-model', supportsText: false, downloaded: true },
  ];
  const notice = () => processingProfileReferenceReadinessNotice(draft, [], models);
  expect(notice()).toBeNull();
  draft.rulesModelMode = 'independent';
  expect(notice()).toBe('Choose a text-capable rules model.');
  draft.rulesModelSettings!.model = 'removed-model';
  expect(notice()).toBe('Rules model is unavailable.');
  draft.additionalRules = ' \n ';
  expect(notice()).toBeNull();
  draft.additionalRules = 'Translate to Spanish';
  draft.additionalRulesSeparatePass = false;
  expect(notice()).toBeNull();
  draft.modelSettings!.model = 'ocr-model';
  expect(notice()).toBe('Choose a separate rules model to apply additional rules.');
  draft.additionalRules = '';
  draft.applicationModeOverride = 'prompt';
  draft.conversion.tables = false;
  expect(notice()).toBe('Use cleanup to apply conversion options with this model.');
  draft.modelSettings!.model = 'text-model';
  expect(notice()).toBeNull();
  expect(draft.modelSettings).not.toHaveProperty('localModelSupportsText');
  expect(draft.modelSettings).not.toHaveProperty('localModelDownloaded');
});

test('shared assessment identifies recovery destinations and checks only active credentials', () => {
  const draft = defaultProcessingProfileDraft('Capture');
  expect(assessProfile(draft, [])).toMatchObject({
    target: 'model',
    label: 'Capture model required',
  });
  draft.modelSettings = {
    provider: 'openai_api',
    model: 'manual-model-not-in-catalog',
    options: null,
  };
  const providers = [
    { id: 'openai_api', backend: 'openai_api' as const, endpoint: null, credentialRef: 'key' },
  ];
  expect(assessProfile(draft, [], undefined, {})).toMatchObject({
    target: 'providers',
    label: 'Connect provider',
  });
  const credentials = {
    openai_api: {
      provider: 'openai_api' as const,
      credentialRef: 'key',
      readiness: 'locked' as const,
    },
  };
  expect(assessProfile(draft, providers, undefined, credentials)).toMatchObject({
    target: 'providers',
    label: 'Check provider access',
  });
  const unread = { openai_api: { ...credentials.openai_api, readiness: 'unknown' as const } };
  expect(assessProfile(draft, providers, undefined, unread)).toBeNull();
  const ready = { openai_api: { ...credentials.openai_api, readiness: 'ready' as const } };
  draft.rulesModelMode = 'independent';
  draft.additionalRulesSeparatePass = true;
  expect(assessProfile(draft, providers, undefined, ready)).toBeNull();
  draft.additionalRules = 'Translate';
  expect(assessProfile(draft, providers, undefined, ready)).toMatchObject({
    target: 'rules',
    label: 'Rules model required',
  });
  draft.rulesModelSettings = { provider: 'local', model: 'ocr', options: null };
  expect(
    assessProfile(draft, providers, [{ id: 'ocr', downloaded: true, supportsText: false }], ready),
  ).toMatchObject({ target: 'rules', label: 'Rules model incompatible' });
});

test('local availability checks stay neutral and downloadable models remain capture-ready', () => {
  const draft = defaultProcessingProfileDraft('Local');
  draft.modelSettings = { provider: 'local', model: 'local-model', options: null };
  expect(assessProfile(draft, [], undefined, {})).toMatchObject({ checking: true });
  expect(
    assessProfile(draft, [], [{ id: 'local-model', downloaded: false, supportsText: true }], {}),
  ).toBeNull();
  expect(assessProfile(draft, [], [], {})).toMatchObject({ label: 'Capture model unavailable' });
});

test('an OpenAI-compatible endpoint connects without an API key', () => {
  const draft = defaultProcessingProfileDraft('Capture');
  const keyless = {
    provider: 'openai_compatible' as const,
    credentialRef: null,
    readiness: 'ready' as const,
  };
  draft.modelSettings = { provider: 'openai_compatible', model: 'llava', options: null };
  const compatible = (endpoint: string | null) => [
    {
      id: 'openai_compatible',
      backend: 'openai_compatible' as const,
      endpoint,
      credentialRef: null,
    },
  ];
  expect(assessProfile(draft, compatible(null), undefined, {})).toMatchObject({
    label: 'Connect provider',
  });
  expect(
    assessProfile(draft, compatible('http://localhost:11434/v1'), undefined, {
      openai_compatible: keyless,
    }),
  ).toBeNull();
  draft.modelSettings = { provider: 'openai_api', model: 'model', options: null };
  expect(
    assessProfile(
      draft,
      [{ id: 'openai_api', backend: 'openai_api' as const, endpoint: null, credentialRef: null }],
      undefined,
      {},
    ),
  ).toMatchObject({ label: 'Connect provider' });
});
