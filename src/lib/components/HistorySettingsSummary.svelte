<script lang="ts">
  import ConversionOptions from '$lib/components/ConversionOptions.svelte';
  import type { HistoryViewState } from '$lib/page-types';
  import HistoryMetadataRow from '$lib/components/HistoryMetadataRow.svelte';
  import { docsLinks } from '$lib/docs-links';
  import { providerLabel } from '$lib/history-search';
  import type {
    ConversionSettingsApplicationMode,
    HistoryModelSnapshot,
    HistorySettingsSnapshot,
  } from '$lib/generated/bindings';

  let {
    snapshot,
    advanced,
    formatting = null,
    canRestoreFormatting = false,
    disabled = false,
    onFormattingChange = () => {},
  }: {
    snapshot: HistorySettingsSnapshot | null;
    advanced: boolean;
    formatting?: HistoryViewState['formatting'];
    canRestoreFormatting?: boolean;
    disabled?: boolean;
    onFormattingChange?: (
      settings: import('$lib/generated/bindings').ProcessingConversionSettings,
    ) => void;
  } = $props();
  const restores = $derived(canRestoreFormatting && snapshot?.applicationMode === 'html_cleanup');

  const applicationModeLabels: Record<ConversionSettingsApplicationMode, string> = {
    prompt: 'Prompt only',
    html_cleanup: 'Cleanup only',
    prompt_and_html_cleanup: 'Prompt + cleanup',
  };

  function modelLabel(model: HistoryModelSnapshot | null): string {
    return model ? `${providerLabel(model.provider)} · ${model.model}` : 'Unavailable';
  }

  function conversionLabel(settings: HistorySettingsSnapshot): string {
    return `Tables ${settings.tables ? 'on' : 'off'} · Colors ${settings.colors ? 'on' : 'off'} · Images ${settings.images ? 'on' : 'off'}`;
  }

  function rulesLabel(settings: HistorySettingsSnapshot): string {
    if (!settings.additionalRulesApplied) return 'None';
    return settings.separatePassUsed ? 'Applied in separate pass' : 'Applied in first pass';
  }
</script>

{#if snapshot}
  <HistoryMetadataRow label="Model">
    <span class="break-all">{modelLabel(snapshot.firstModel)}</span>
  </HistoryMetadataRow>
  {#if snapshot.separatePassUsed}
    <HistoryMetadataRow label="Second model">
      <span class="break-all">{modelLabel(snapshot.secondModel)}</span>
    </HistoryMetadataRow>
  {/if}
  {#if formatting}
    {@render options()}
  {:else}
    <HistoryMetadataRow label="Options" value={conversionLabel(snapshot)} />
  {/if}
  {#if advanced}
    <HistoryMetadataRow
      label="Conversion mode"
      value={applicationModeLabels[snapshot.applicationMode]}
      infoLink={docsLinks.conversionApplication}
    />
  {/if}
  {#if advanced || snapshot.additionalRulesApplied}
    <HistoryMetadataRow
      label="Additional Rules"
      value={rulesLabel(snapshot)}
      infoLink={docsLinks.historyRules}
    />
  {/if}
{:else}
  <HistoryMetadataRow label="Settings" value="Unavailable" />
  {#if formatting}{@render options()}{/if}
{/if}

{#snippet options()}
  {#if formatting}
    <HistoryMetadataRow
      label="Options"
      class="grid-cols-[auto_minmax(0,1fr)]"
      infoLink={docsLinks.historyOptions}
    >
      <ConversionOptions
        value={formatting.settings}
        {disabled}
        disabledOptions={{
          tables: !restores && snapshot?.tables === false,
          colors: !restores && snapshot?.colors === false,
          images: !restores && snapshot?.images === false,
        }}
        disabledOptionHint="This option was off when this was captured, so it can’t be turned on."
        onChange={onFormattingChange}
      />
      {#if formatting.loading}<span role="status" class="sr-only">Updating preview…</span>{/if}
      {#if formatting.error}<span role="alert" class="block mt-1 text-sm text-danger"
          >{formatting.error}</span
        >{/if}
    </HistoryMetadataRow>
  {/if}
{/snippet}
