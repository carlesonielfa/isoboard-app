<script lang="ts">
  import type { Snippet } from 'svelte';
  import * as Tabs from '$lib/components/ui/tabs/index.js';
  import ContentSurface from '$lib/components/ContentSurface.svelte';
  import MakerMark from '$lib/components/MakerMark.svelte';
  import OverscrollReveal from '$lib/components/OverscrollReveal.svelte';
  import SettingsHeader from '$lib/components/SettingsHeader.svelte';
  import SettingsSectionIcon from '$lib/components/SettingsSectionIcon.svelte';
  import type { SettingsSectionIconName } from '$lib/components/SettingsSectionIcon.svelte';

  export type SettingsSection = SettingsSectionIconName;

  const sections: { value: SettingsSection; label: string }[] = [
    { value: 'history', label: 'History' },
    { value: 'shortcuts', label: 'Shortcuts' },
    { value: 'processing-profiles', label: 'Profiles' },
    { value: 'provider-access', label: 'Providers' },
    { value: 'settings', label: 'Settings' },
  ];

  let {
    value,
    onChange,
    historyContent,
    shortcutsContent,
    processingProfilesContent,
    providerAccessContent,
    settingsContent,
    setupContent,
    captureActions,
    hidden = false,
    sounds = false,
  }: {
    value: SettingsSection;
    onChange: (value: SettingsSection) => void;
    historyContent: Snippet;
    shortcutsContent: Snippet;
    processingProfilesContent: Snippet;
    providerAccessContent: Snippet;
    settingsContent: Snippet;
    setupContent?: Snippet;
    captureActions?: Snippet;
    hidden?: boolean;
    sounds?: boolean;
  } = $props();

  // Scroll panels that reveal the maker mark when scrolled past their edges.
  const scrollers = $state<Partial<Record<SettingsSection, HTMLElement | null>>>({
    shortcuts: null,
    'provider-access': null,
    settings: null,
  });

  function handleValueChange(nextValue: string): void {
    if (sections.some((section) => section.value === nextValue)) {
      onChange(nextValue as SettingsSection);
    }
  }
</script>

<Tabs.Root
  {value}
  onValueChange={handleValueChange}
  orientation="horizontal"
  class={hidden ? 'hidden' : 'flex min-h-0 min-w-0 flex-1 flex-col gap-0'}
>
  <SettingsHeader trailing={captureActions} {sounds}>
    {#snippet navigation()}
      <Tabs.List
        variant="default"
        aria-label="Settings sections"
        class="w-fit max-w-full justify-start gap-0 overflow-x-auto overflow-y-hidden"
      >
        {#each sections as section (section.value)}
          <Tabs.Trigger
            id={`settings-tab-${section.value}`}
            value={section.value}
            aria-controls={`settings-panel-${section.value}`}
            aria-label={section.label}
            title={section.label}
            class="h-toolbar-button !w-auto !min-w-10 shrink-0 gap-0 px-2 py-1 transition-[background-color,color,box-shadow] duration-200 ease-out data-[state=active]:bg-highlight data-[state=active]:text-canvas motion-reduce:transition-none"
          >
            <SettingsSectionIcon name={section.value} active={value === section.value} />
            <span
              aria-hidden="true"
              class={`inline-block overflow-hidden whitespace-nowrap text-left transition-[max-width,opacity,margin] duration-200 ease-out motion-reduce:transition-none ${value === section.value ? 'max-w-40 ml-1.5 opacity-100' : 'max-w-0 ml-0 opacity-0'}`}
              >{section.label}</span
            >
          </Tabs.Trigger>
        {/each}
      </Tabs.List>
    {/snippet}
  </SettingsHeader>

  <ContentSurface>
    <OverscrollReveal scroller={scrollers[value] ?? null}>
      <MakerMark />
    </OverscrollReveal>
    {#if setupContent}
      {@render setupContent()}
    {/if}

    <Tabs.Content
      id="settings-panel-history"
      value="history"
      aria-labelledby="settings-tab-history"
      class="min-h-0 flex-1 overflow-hidden p-0"
    >
      {@render historyContent()}
    </Tabs.Content>
    <Tabs.Content
      bind:ref={scrollers['shortcuts']}
      id="settings-panel-shortcuts"
      value="shortcuts"
      aria-labelledby="settings-tab-shortcuts"
      class="min-h-0 flex-1 overflow-y-auto px-0 py-4"
    >
      <div class="mx-auto min-h-full w-full max-w-[58rem] px-6 pb-4 pt-0.5 sm:px-10">
        {@render shortcutsContent()}
      </div>
    </Tabs.Content>
    <Tabs.Content
      id="settings-panel-processing-profiles"
      value="processing-profiles"
      aria-labelledby="settings-tab-processing-profiles"
      class="min-h-0 flex-1 overflow-hidden p-0"
    >
      {@render processingProfilesContent()}
    </Tabs.Content>

    <Tabs.Content
      bind:ref={scrollers['provider-access']}
      id="settings-panel-provider-access"
      value="provider-access"
      aria-labelledby="settings-tab-provider-access"
      class="min-h-0 flex-1 overflow-y-auto px-0 py-4"
    >
      <div class="mx-auto min-h-full w-full max-w-[58rem] px-6 pb-4 pt-0.5 sm:px-10">
        {@render providerAccessContent()}
      </div>
    </Tabs.Content>
    <Tabs.Content
      bind:ref={scrollers['settings']}
      id="settings-panel-settings"
      value="settings"
      aria-labelledby="settings-tab-settings"
      class="min-h-0 flex-1 overflow-y-auto px-0 py-4"
    >
      <div class="mx-auto min-h-full w-full max-w-[58rem] px-6 pb-4 pt-0.5 sm:px-10">
        {@render settingsContent()}
      </div>
    </Tabs.Content>
  </ContentSurface>
</Tabs.Root>
