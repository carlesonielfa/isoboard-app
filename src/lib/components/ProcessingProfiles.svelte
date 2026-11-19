<script lang="ts">
  import { tick } from 'svelte';
  import type {
    DesktopAvailability,
    ProcessingProfilesViewState,
    ProcessingProfileDraft,
    ProcessingPromptTemplateDefaults,
  } from '$lib/page-types';
  import type {
    ConfigSnapshot,
    InferenceBackendMetadata,
    ProviderSnapshot,
  } from '$lib/generated/bindings';
  import Button from '$lib/components/Button.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import SidebarTabs from '$lib/components/SidebarTabs.svelte';
  import SidebarTabList from '$lib/components/SidebarTabList.svelte';
  import SidebarTabRow from '$lib/components/SidebarTabRow.svelte';
  import SidebarTabTrigger from '$lib/components/SidebarTabTrigger.svelte';
  import SidebarTabContent from '$lib/components/SidebarTabContent.svelte';
  import ProcessingProfileEditor from '$lib/components/ProcessingProfileEditor.svelte';
  import DropdownMenuButton from '$lib/components/DropdownMenuButton.svelte';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { assessProfile, type CredentialReadiness } from '$lib/processing-profile-state';
  import { useLocalModels } from '$lib/local-models-controller.svelte';
  import { CircleAlert, CopyPlus, Trash2 } from '@lucide/svelte';
  import type { ModelCatalogState } from '$lib/model-catalog-controller.svelte';

  let {
    state: viewState,
    config,
    configLoaded,
    availability,
    onCreate,
    onDuplicate,
    onSave,
    onDelete,
    onSelect,
    backendMetadata = [],
    inferenceProviders = [],
    inferenceReady = false,
    credentials,
    onOpenProviders,
    modelCatalog,
    advanced = false,
    promptTemplateDefaults,
  }: {
    state: ProcessingProfilesViewState;
    config: ConfigSnapshot;
    configLoaded: boolean;
    availability: DesktopAvailability;
    onCreate: () => Promise<string | null>;
    onDuplicate: (id: string) => Promise<string | null>;
    onSave: (id: string, draft: ProcessingProfileDraft) => Promise<boolean>;
    onDelete: (id: string) => void;
    onSelect: (id: string) => Promise<boolean>;
    backendMetadata?: InferenceBackendMetadata[];
    inferenceProviders?: ProviderSnapshot[];
    inferenceReady?: boolean;
    credentials?: CredentialReadiness;
    onOpenProviders?: () => void;
    modelCatalog: ModelCatalogState;
    advanced?: boolean;
    promptTemplateDefaults?: ProcessingPromptTemplateDefaults;
  } = $props();

  const localModels = useLocalModels();
  const issues = $derived(
    new Map(
      config.processing.profiles.map((profile) => [
        profile.id,
        inferenceReady
          ? assessProfile(
              profile,
              inferenceProviders,
              localModels && !localModels.controller.state.loading
                ? localModels.controller.state.snapshot.models
                : undefined,
              credentials,
            )
          : null,
      ]),
    ),
  );
  const busy = $derived(!availability.desktopAvailable || viewState.pending !== null);

  async function focusName(id: string): Promise<void> {
    await tick();
    const input = document.getElementById(`profile-editor-${id}`)?.querySelector('input');
    input?.focus();
    input?.select();
  }

  async function createProfile(): Promise<void> {
    const id = await onCreate();
    if (id && (await onSelect(id))) await focusName(id);
  }

  async function duplicateProfile(id: string): Promise<void> {
    const copyId = await onDuplicate(id);
    if (copyId) await focusName(copyId);
  }
</script>

<div class="flex h-full min-h-0 min-w-0 flex-col">
  {#if configLoaded}
    <p class="m-0 border-b border-border px-4 py-3 text-sm">
      Profiles define the model, formatting, and rules for captures.
    </p>
    <SidebarTabs
      value={config.processing.activeProfileId}
      onValueChange={(id) => {
        if (!busy) void onSelect(id);
      }}
      class="grid min-h-0 flex-1 grid-cols-[14rem_minmax(0,1fr)] grid-rows-[minmax(0,1fr)]"
    >
      <div class="flex min-h-0 min-w-0 flex-col overflow-y-auto border-r border-border">
        <SidebarTabList
          aria-label="Profiles"
          class="flex shrink-0 flex-col items-stretch justify-start gap-1 p-2"
        >
          {#each config.processing.profiles as profile (profile.id)}
            {@const issue = issues.get(profile.id)}
            <SidebarTabRow selected={profile.id === config.processing.activeProfileId}>
              <SidebarTabTrigger
                value={profile.id}
                disabled={busy}
                title={`${profile.name} — ${issue?.message ?? profile.modelSettings?.model ?? 'Choose a model'}`}
                aria-label={`${profile.name}${profile.id === config.processing.activeProfileId ? ', active for capture' : ''}${issue ? `, ${issue.label}` : ''}`}
                activeMarker={profile.id === config.processing.activeProfileId}
                class="gap-2 py-2"
              >
                <span class="grid min-w-0 gap-0.5">
                  <span class="flex min-w-0 items-center gap-1.5 text-sm font-semibold">
                    <span class="truncate">{profile.name}</span>
                    {#if issue && !issue.checking}<CircleAlert
                        class="size-3.5 shrink-0 text-danger"
                        aria-hidden="true"
                      />{/if}
                  </span>
                  <span
                    class={`truncate text-xs ${issue ? (issue.checking ? '' : 'text-danger') : 'font-mono'}`}
                    >{issue?.label ?? (profile.modelSettings?.model || 'Choose a model')}</span
                  >
                </span>
              </SidebarTabTrigger>
              <DropdownMenuButton
                label={`Actions for ${profile.name}`}
                disabled={busy}
                class="mr-1 shrink-0"
              >
                {#snippet menu()}
                  <DropdownMenu.Item
                    disabled={busy}
                    onSelect={() => void duplicateProfile(profile.id)}
                  >
                    <CopyPlus class="size-4" aria-hidden="true" /> Duplicate
                  </DropdownMenu.Item>
                  <DropdownMenu.Item
                    variant="destructive"
                    disabled={busy || config.processing.profiles.length <= 1}
                    onSelect={() => onDelete(profile.id)}
                  >
                    <Trash2 class="size-4" aria-hidden="true" /> Delete
                  </DropdownMenu.Item>
                {/snippet}
              </DropdownMenuButton>
            </SidebarTabRow>
          {/each}
        </SidebarTabList>
        <div class="flex shrink-0 justify-center px-2 pb-2">
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label="Add profile"
            title="Add profile"
            disabled={busy}
            onclick={() => void createProfile()}
          >
            <Icon name="plus" />
          </Button>
        </div>
      </div>
      <div class="min-h-0 min-w-0 overflow-hidden">
        {#each config.processing.profiles as profile (profile.id)}
          <SidebarTabContent value={profile.id} class="m-0 h-full min-h-0 overflow-y-auto p-4">
            <div id={`profile-editor-${profile.id}`} class="grid min-w-0 gap-3">
              <ProcessingProfileEditor
                {profile}
                {backendMetadata}
                {inferenceProviders}
                {inferenceReady}
                {credentials}
                {onOpenProviders}
                disabled={busy}
                embedded
                onSaveProfile={onSave}
                {advanced}
                {promptTemplateDefaults}
                {modelCatalog}
                modelPickerId={`profile-${profile.id}-model`}
              />
            </div>
          </SidebarTabContent>
        {/each}
      </div>
    </SidebarTabs>
  {/if}
</div>
