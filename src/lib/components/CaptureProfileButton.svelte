<script lang="ts">
  import { Check } from '@lucide/svelte';
  import CaptureIcon from '$lib/components/CaptureIcon.svelte';
  import SplitButton from '$lib/components/SplitButton.svelte';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import type { ProcessingProfileSnapshot } from '$lib/generated/bindings';
  import type { CaptureReadiness } from '$lib/capture-readiness';

  let {
    profiles,
    activeProfileId,
    readiness,
    disabled = false,
    highlighted = false,
    onCapture,
    onSelectProfile,
  }: {
    profiles: ProcessingProfileSnapshot[];
    activeProfileId: string;
    readiness: CaptureReadiness;
    disabled?: boolean;
    highlighted?: boolean;
    onCapture: () => void;
    onSelectProfile: (id: string) => void | Promise<boolean>;
  } = $props();
  const activeProfile = $derived(profiles.find((profile) => profile.id === activeProfileId));
</script>

<SplitButton
  label={activeProfile ? `Capture using ${activeProfile.name}` : 'Capture'}
  title={readiness.ready
    ? `Capture using ${activeProfile?.name ?? 'the active profile'}`
    : readiness.detail}
  menuLabel={`Select capture profile${activeProfile ? `: ${activeProfile.name}` : ''}`}
  size="toolbar"
  divided
  class={`max-w-full border border-border bg-surface-control p-0.5 transition-shadow duration-700 ${highlighted && readiness.ready && !disabled ? 'border-focus/50 shadow-md shadow-focus/25 motion-safe:animate-attention-glow' : ''}`}
  {disabled}
  actionDisabled={!readiness.ready || !activeProfile}
  onclick={onCapture}
>
  <CaptureIcon />
  <span class="min-w-0 truncate">{activeProfile?.name ?? 'Capture'}</span>
  {#snippet menu()}
    <DropdownMenu.Group>
      <DropdownMenu.GroupHeading>Capture with profile...</DropdownMenu.GroupHeading>
      {#each profiles as profile (profile.id)}
        <DropdownMenu.Item
          aria-label={`${profile.name}${profile.id === activeProfileId ? ', selected' : ''}`}
          onSelect={() => {
            void onSelectProfile(profile.id);
          }}
        >
          <Check
            class={`size-4 ${profile.id === activeProfileId ? '' : 'opacity-0'}`}
            aria-hidden="true"
          />
          {profile.name}
        </DropdownMenu.Item>
      {/each}
    </DropdownMenu.Group>
  {/snippet}
</SplitButton>
