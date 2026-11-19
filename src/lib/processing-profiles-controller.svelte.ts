import { commands } from '$lib/ipc';
import {
  defaultProcessingProfileDraft,
  nextProcessingProfileName,
  normalizeProcessingProfileDraft,
  processingProfileDraftFromSnapshot,
  processingProfileDraftIsValid,
} from '$lib/processing-profile-state';
import { nextProfileCopyName } from '$lib/profile-copy-name';
import { errorIdentity, errorMessage, errorRevision, type AppErrorStore } from '$lib/app-error';
import {
  isConfigError,
  type ConfigMutationResponse,
  type ConfigSession,
} from '$lib/config-session.svelte';
import type {
  DesktopAvailability,
  ProcessingProfileDraft,
  ProcessingProfilesViewState,
} from '$lib/page-types';

function createReactiveState(
  initialState: ProcessingProfilesViewState,
): ProcessingProfilesViewState {
  const state = $state(initialState);
  return state;
}

export type ProcessingProfilesControllerOptions = {
  availability: DesktopAvailability;
  session: ConfigSession;
  errors?: AppErrorStore;
  commands?: Pick<
    typeof commands,
    | 'createProcessingProfile'
    | 'updateProcessingProfile'
    | 'renameProcessingProfile'
    | 'deleteProcessingProfile'
    | 'selectProcessingProfile'
  >;
  stateFactory?: <T>(value: T) => T;
};

export function createProcessingProfilesController({
  availability,
  session,
  errors,
  commands: commandApi = commands,
  stateFactory,
}: ProcessingProfilesControllerOptions) {
  const state = stateFactory
    ? stateFactory<ProcessingProfilesViewState>({ pending: null, error: null })
    : createReactiveState({ pending: null, error: null });
  let localErrorRevision = 0;

  async function runOperation(
    operation: NonNullable<ProcessingProfilesViewState['pending']>,
    mutate: (expectedRevision: number) => Promise<ConfigMutationResponse>,
    onResponse?: (response: ConfigMutationResponse) => void,
  ): Promise<boolean> {
    if (!availability.desktopAvailable || state.pending) return false;
    state.pending = operation;
    state.error = null;
    try {
      const response = await session.mutate(mutate);
      onResponse?.(response);
      return true;
    } catch (error) {
      if (!isConfigError(error)) {
        state.error = errorMessage(error);
        errors?.show(
          state.error,
          errorIdentity('config', errorRevision(error, ++localErrorRevision)),
        );
      }
      return false;
    } finally {
      state.pending = null;
    }
  }

  function createProfile(draftOrName?: ProcessingProfileDraft | string): Promise<string | null> {
    const profiles = session.current.processing.profiles;
    const candidate =
      draftOrName !== null && typeof draftOrName === 'object'
        ? draftOrName
        : defaultProcessingProfileDraft(
            (typeof draftOrName === 'string' ? draftOrName.trim() : '') ||
              nextProcessingProfileName(profiles),
          );
    if (!candidate || !processingProfileDraftIsValid(candidate)) return Promise.resolve(null);
    const draft = normalizeProcessingProfileDraft(candidate);
    let createdId: string | null = null;
    return runOperation(
      'create',
      (expectedRevision) => commandApi.createProcessingProfile(draft, expectedRevision),
      (response) => {
        createdId = 'id' in response ? response.id : null;
      },
    ).then((saved) => (saved ? createdId : null));
  }

  async function duplicateProfile(id: string): Promise<string | null> {
    const profiles = session.current.processing.profiles;
    const source = profiles.find((profile) => profile.id === id);
    if (!source) return null;
    const copyId = await createProfile({
      ...processingProfileDraftFromSnapshot(source),
      name: nextProfileCopyName(
        source.name,
        profiles.map((profile) => profile.name),
      ),
    });
    if (!copyId) return null;
    await selectProfile(copyId);
    return copyId;
  }

  function saveProfile(id: string, draft: ProcessingProfileDraft): Promise<boolean> {
    if (!id || !processingProfileDraftIsValid(draft)) return Promise.resolve(false);
    const normalized = normalizeProcessingProfileDraft(draft);
    if (!session.current.processing.profiles.some((profile) => profile.id === id))
      return Promise.resolve(false);
    return runOperation('update', (expectedRevision) =>
      commandApi.updateProcessingProfile(id, normalized, expectedRevision),
    );
  }

  function renameProfile(id: string, name: string): Promise<boolean> {
    const trimmedName = name.trim();
    if (!trimmedName) {
      state.error = 'Enter a profile name.';
      errors?.show(state.error, errorIdentity('profiles', ++localErrorRevision));
      return Promise.resolve(false);
    }
    const profile = session.current.processing.profiles.find((candidate) => candidate.id === id);
    if (!profile || profile.name === trimmedName) return Promise.resolve(true);
    return runOperation('rename', (expectedRevision) =>
      commandApi.renameProcessingProfile(id, trimmedName, expectedRevision),
    );
  }

  function deleteProfile(id: string): Promise<boolean> {
    const profiles = session.current.processing.profiles;
    if (profiles.length <= 1 || !profiles.some((profile) => profile.id === id)) {
      return Promise.resolve(false);
    }
    return runOperation('delete', (expectedRevision) =>
      commandApi.deleteProcessingProfile(id, expectedRevision),
    );
  }

  function selectProfile(id: string): Promise<boolean> {
    const config = session.current.processing;
    if (config.activeProfileId === id) return Promise.resolve(true);
    if (!config.profiles.some((profile) => profile.id === id)) return Promise.resolve(false);
    return runOperation('select', (expectedRevision) =>
      commandApi.selectProcessingProfile(id, expectedRevision),
    );
  }

  return {
    state,
    createProfile,
    duplicateProfile,
    saveProfile,
    renameProfile,
    deleteProfile,
    selectProfile,
  };
}
