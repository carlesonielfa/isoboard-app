import type { BuiltinProfileMetadata, InferenceBackendMetadata } from '$lib/generated/bindings';

export function builtInProfileMetadata(
  metadata: InferenceBackendMetadata[],
  provider: string,
): BuiltinProfileMetadata[] {
  return metadata.find((backend) => backend.id === provider)?.builtInProfiles ?? [];
}
