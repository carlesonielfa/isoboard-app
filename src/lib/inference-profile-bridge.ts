import { commands } from '$lib/ipc';
import type { ConfigSnapshot } from '$lib/generated/bindings';
import type { InferenceProviderCredentialDraft } from '$lib/page-types';

export const getInferenceBackendMetadata = commands.getInferenceBackendMetadata;
export const getInferenceModelCatalog = commands.getInferenceModelCatalog;

export function saveInferenceProviderWithCredential(
  draft: InferenceProviderCredentialDraft,
  expectedRevision: number,
): Promise<ConfigSnapshot> {
  return commands.saveProviderWithCredential(
    draft.backend,
    draft.endpoint,
    draft.credential,
    expectedRevision,
  );
}

export const getInferenceCredentialReadiness = commands.getInferenceCredentialReadiness;
export const deleteUnreferencedInferenceCredential = commands.deleteUnreferencedInferenceCredential;
export const clearProviderCredential = commands.clearProviderCredential;
