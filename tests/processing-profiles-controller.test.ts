/// <reference types="bun-types" />

import { describe, expect, test } from 'bun:test';
import { createProcessingProfilesController } from '../src/lib/processing-profiles-controller.svelte';
import { createConfigSession } from '../src/lib/config-session.svelte';
import {
  defaultProcessingProfileDraft,
  processingProfileSnapshotFromDraft,
  processingProfileDraftFromSnapshot,
} from '../src/lib/processing-profile-state';
import type { ConfigSnapshot, ProcessingProfileSnapshot } from '../src/lib/generated/bindings';
import type { ProcessingProfileDraft_Deserialize } from '../src/lib/generated/bindings';

const plainState = <T>(value: T): T => value;

function sessionEnvironment(snapshotProvider: () => ConfigSnapshot) {
  const eventSource = () => ({ listen: async () => () => undefined });
  return {
    isTauri: () => true,
    commands: {
      getCaptureConfig: async () => snapshotProvider(),
      setUiMode: async () => snapshotProvider(),
    },
    events: {
      configChanged: eventSource(),
      configError: eventSource(),
    },
  };
}

function snapshot(
  revision: number,
  profiles: ProcessingProfileSnapshot[],
  activeProfileId = 'default',
): ConfigSnapshot {
  return {
    version: 1,
    captureBindings: [],
    openShortcut: 'CommandOrControl+Alt+KeyZ',
    uiMode: 'basic',
    soundsEnabled: true,
    keepHistoryOriginals: true,
    automaticUpdateChecks: true,
    overlayPosition: 'bottom_center',
    configPath: 'config.toml',
    processing: {
      activeProfileId,
      profiles,
      promptTemplateDefaults: { conversion: '', postprocessing: '' },
    },
    inference: { providers: [] },
    revision,
    registered: false,
    registrationError: null,
    lastError: null,
  };
}

describe('profile controller', () => {
  test('passes the native create command boundary and applies repeated created responses', async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    let profiles: ProcessingProfileSnapshot[] = [processingProfileSnapshotFromDraft('default')];
    let revision = 0;
    const createProcessingProfile = async (
      draft: ProcessingProfileDraft_Deserialize,
      expectedRevision: number,
    ) => {
      calls.push({ command: 'create_processing_profile', args: { draft, expectedRevision } });
      const name = String(draft.name);
      const id = `profile-${profiles.length}`;
      profiles = [
        ...profiles,
        processingProfileSnapshotFromDraft(id, defaultProcessingProfileDraft(name)),
      ];
      revision += 1;
      return { snapshot: snapshot(revision, profiles), id };
    };
    const session = createConfigSession({ desktopAvailable: true }, undefined, plainState, {
      ...sessionEnvironment(() => snapshot(0, profiles)),
    });
    session.mount();
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    const controller = createProcessingProfilesController({
      availability: { desktopAvailable: true },
      session,
      commands: {
        createProcessingProfile,
        updateProcessingProfile: async () => snapshot(revision, profiles),
        renameProcessingProfile: async () => snapshot(revision, profiles),
        deleteProcessingProfile: async () => snapshot(revision, profiles),
        selectProcessingProfile: async () => snapshot(revision, profiles),
      },
      stateFactory: plainState,
    });

    const firstId = await controller.createProfile();
    const secondId = await controller.createProfile();

    expect(firstId).toBe('profile-1');
    expect(secondId).toBe('profile-2');
    expect(calls).toEqual([
      {
        command: 'create_processing_profile',
        args: {
          draft: defaultProcessingProfileDraft('New profile'),
          expectedRevision: 0,
        },
      },
      {
        command: 'create_processing_profile',
        args: {
          draft: defaultProcessingProfileDraft('New profile 2'),
          expectedRevision: 1,
        },
      },
    ]);
    expect(session.state.revision).toBe(2);
    expect(session.current.processing.profiles.map((profile) => profile.name)).toEqual([
      'Default',
      'New profile',
      'New profile 2',
    ]);
    expect(controller.state.pending).toBeNull();
  });

  test('duplicates every saved field, then selects the copy as active', async () => {
    const source = processingProfileSnapshotFromDraft('default', {
      ...defaultProcessingProfileDraft('Rules'),
      modelSettings: { provider: 'openai_api' as const, model: 'vision', options: null },
      rulesModelSettings: { provider: 'openai_api' as const, model: 'text', options: null },
      conversionPromptTemplate: 'Conversion override',
      postprocessingPromptTemplate: 'Postprocessing override',
      additionalRules: 'Preserve URLs.',
      additionalRulesSeparatePass: true,
      applicationModeOverride: 'prompt',
      conversion: { tables: false, colors: true, images: false },
    });
    const originalDraft = processingProfileDraftFromSnapshot(source);
    const createdDrafts: ProcessingProfileDraft_Deserialize[] = [];
    const selected: Array<{ id: string; revision: number }> = [];
    const copy = { ...source, id: 'profile-1', name: 'Rules copy' };
    const session = createConfigSession({ desktopAvailable: true }, undefined, plainState, {
      ...sessionEnvironment(() => snapshot(3, [source])),
    });
    session.mount();
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    const controller = createProcessingProfilesController({
      availability: { desktopAvailable: true },
      session,
      commands: {
        createProcessingProfile: async (draft, expectedRevision) => {
          expect(expectedRevision).toBe(3);
          createdDrafts.push(draft);
          return {
            snapshot: snapshot(4, [source, { ...copy, name: draft.name }]),
            id: 'profile-1',
          };
        },
        updateProcessingProfile: async () => snapshot(3, [source]),
        renameProcessingProfile: async () => snapshot(3, [source]),
        deleteProcessingProfile: async () => snapshot(3, [source]),
        selectProcessingProfile: async (id, expectedRevision) => {
          selected.push({ id, revision: expectedRevision });
          return snapshot(5, [source, copy], id);
        },
      },
      stateFactory: plainState,
    });

    expect(await controller.duplicateProfile('default')).toBe('profile-1');
    expect(createdDrafts[0]).toEqual({ ...originalDraft, name: 'Rules copy' });
    expect(selected).toEqual([{ id: 'profile-1', revision: 4 }]);
    expect(session.current.processing.activeProfileId).toBe('profile-1');
    expect(session.current.processing.profiles[1]?.id).toBe('profile-1');
  });

  test('updates a complete draft through the shared revision queue', async () => {
    const draft = {
      name: 'Tables and notes',
      rulesModelMode: 'independent' as const,
      modelSettings: { provider: 'openai_api' as const, model: 'vision-model', options: null },
      rulesModelSettings: { provider: 'openai_api' as const, model: 'text-model', options: null },
      conversionPromptTemplate: null,
      postprocessingPromptTemplate: null,
      additionalRules: 'Translate prose to Spanish;\nPreserve URLs.',
      additionalRulesSeparatePass: true,
      applicationModeOverride: 'html_cleanup' as const,
      conversion: { tables: false, colors: true, images: false },
    };
    const existing = processingProfileSnapshotFromDraft('default', draft);
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const session = createConfigSession({ desktopAvailable: true }, undefined, plainState, {
      ...sessionEnvironment(() => snapshot(4, [existing])),
    });
    session.mount();
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();

    const controller = createProcessingProfilesController({
      availability: { desktopAvailable: true },
      session,
      commands: {
        createProcessingProfile: async () => ({
          snapshot: snapshot(5, [existing]),
          id: existing.id,
        }),
        updateProcessingProfile: async (id, draft, expectedRevision) => {
          calls.push({
            command: 'update_processing_profile',
            args: { id, draft, expectedRevision },
          });
          return snapshot(5, [existing]);
        },
        renameProcessingProfile: async () => snapshot(5, [existing]),
        deleteProcessingProfile: async () => snapshot(5, [existing]),
        selectProcessingProfile: async () => snapshot(5, [existing]),
      },
      stateFactory: plainState,
    });

    expect(
      await controller.saveProfile('default', {
        ...draft,
        additionalRules: `${draft.additionalRules}  `,
      }),
    ).toBe(true);
    expect(calls).toEqual([
      {
        command: 'update_processing_profile',
        args: {
          id: 'default',
          draft: { ...draft, additionalRules: `${draft.additionalRules}  ` },
          expectedRevision: 4,
        },
      },
    ]);

    expect(
      await controller.saveProfile('default', {
        ...draft,
        additionalRules: ' \v ',
      }),
    ).toBe(false);
    expect(calls).toHaveLength(1);
  });

  test('failed whole-form saves preserve canonical fields and permit retry with exact rules', async () => {
    const existing = processingProfileSnapshotFromDraft('default', {
      ...defaultProcessingProfileDraft(),
      applicationModeOverride: 'prompt',
      rulesModelSettings: {
        provider: 'openai_api' as const,
        model: 'reserved-text',
        options: null,
      },
      additionalRulesSeparatePass: true,
    });
    const candidate = {
      ...existing,
      name: 'Changed',
      modelSettings: { provider: 'openai_api' as const, model: 'new-vision', options: null },
      additionalRules: '  Translate prose;\nPreserve URLs.  ',
      conversion: { ...existing.conversion, colors: false },
    };
    const draft = processingProfileDraftFromSnapshot(candidate);
    const session = createConfigSession({ desktopAvailable: true }, undefined, plainState, {
      ...sessionEnvironment(() => snapshot(3, [existing])),
    });
    session.mount();
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    let attempts = 0;
    const controller = createProcessingProfilesController({
      availability: { desktopAvailable: true },
      session,
      commands: {
        createProcessingProfile: async () => ({
          snapshot: snapshot(4, [existing]),
          id: existing.id,
        }),
        updateProcessingProfile: async (_id, receivedDraft, expectedRevision) => {
          expect(receivedDraft).toEqual(draft);
          expect(expectedRevision).toBe(3);
          if (++attempts === 1) throw new Error('Save failed');
          return snapshot(4, [processingProfileSnapshotFromDraft('default', draft)]);
        },
        renameProcessingProfile: async () => snapshot(4, [existing]),
        deleteProcessingProfile: async () => snapshot(4, [existing]),
        selectProcessingProfile: async () => snapshot(4, [existing]),
      },
      stateFactory: plainState,
    });

    expect(await controller.saveProfile('default', draft)).toBe(false);
    expect(session.current.processing.profiles).toEqual([existing]);
    expect(draft.additionalRules).toBe('  Translate prose;\nPreserve URLs.  ');
    expect(controller.state.pending).toBeNull();
    expect(await controller.saveProfile('default', draft)).toBe(true);
    expect(session.current.processing.profiles[0]).toEqual(
      processingProfileSnapshotFromDraft('default', draft),
    );
  });
});
