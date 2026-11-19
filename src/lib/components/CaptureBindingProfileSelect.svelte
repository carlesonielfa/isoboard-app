<script lang="ts">
  import type { ProcessingProfileSnapshot } from '$lib/generated/bindings';
  import SettingsSelect from '$lib/components/SettingsSelect.svelte';

  let {
    label,
    profileId,
    profiles,
    disabled,
    onSelect,
  }: {
    label: string;
    profileId: string;
    profiles: ProcessingProfileSnapshot[];
    disabled: boolean;
    onSelect: (profileId: string) => void;
  } = $props();
</script>

<SettingsSelect
  {label}
  labelHidden
  value={profileId}
  {disabled}
  options={[
    ...profiles.map((profile) => ({ value: profile.id, label: profile.name })),
    ...(profileId && !profiles.some((profile) => profile.id === profileId)
      ? [{ value: profileId, label: 'Unavailable profile' }]
      : []),
  ]}
  onValueChange={onSelect}
/>
