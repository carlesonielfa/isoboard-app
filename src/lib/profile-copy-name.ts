import { applicationConfig } from '$lib/application-config';
const MAX_PROFILE_NAME_CHARACTERS = applicationConfig.limits.profileNameCharacters;

export function nextProfileCopyName(sourceName: string, existingNames: Iterable<string>): string {
  const names = new Set(Array.from(existingNames, (name) => name.trim().toLowerCase()));
  const sourceCharacters = Array.from(sourceName.trim());
  let copyNumber = 1;

  while (true) {
    const suffix = copyNumber === 1 ? ' copy' : ` copy ${copyNumber}`;
    const base = sourceCharacters
      .slice(0, MAX_PROFILE_NAME_CHARACTERS - Array.from(suffix).length)
      .join('')
      .trimEnd();
    const candidate = `${base}${suffix}`;
    if (!names.has(candidate.toLowerCase())) return candidate;
    copyNumber += 1;
  }
}
