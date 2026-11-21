<script lang="ts">
  import { Copy } from '@lucide/svelte';
  import Button from '$lib/components/Button.svelte';
  import FitHtmlPreview from '$lib/components/FitHtmlPreview.svelte';
  import HistoryMetadataRow from '$lib/components/HistoryMetadataRow.svelte';
  import { docsLinks } from '$lib/docs-links';
  import HistorySettingsSummary from '$lib/components/HistorySettingsSummary.svelte';
  import ResizableSplitPane from '$lib/components/ResizableSplitPane.svelte';
  import type { HistoryDebugKind } from '$lib/generated/bindings';
  import type { HistoryEntry, HistoryViewState } from '$lib/page-types';
  import { CONTENT_TAG_LABELS } from '$lib/history-search';

  let {
    entry,
    formatting,
    onFormattingChange,
    advanced,
    debug,
    debugCopied,
    failureCopiedId,
    disabled,
    onCopyFailureDetails,
    onCopyDebugHtml,
    detailSize = $bindable(null),
  }: {
    entry: HistoryEntry;
    formatting: HistoryViewState['formatting'];
    onFormattingChange: (
      id: string,
      settings: import('$lib/generated/bindings').ProcessingConversionSettings,
    ) => void;
    advanced: boolean;
    debug: boolean;
    debugCopied: HistoryViewState['debugCopied'];
    failureCopiedId: string | null;
    disabled: boolean;
    onCopyFailureDetails: (id: string) => void;
    onCopyDebugHtml: (id: string, kind: HistoryDebugKind) => void;
    detailSize?: number | null;
  } = $props();

  const formattedTime = $derived(formatTimestamp(entry.createdAt));
  const providerResponse = $derived(
    entry.status === 'failed' ? providerResponseFromDetails(entry.errorDetails) : null,
  );
  const imageSize = $derived(
    entry.imageWidth && entry.imageHeight
      ? `${entry.imageWidth} × ${entry.imageHeight}`
      : 'Unavailable',
  );

  /** The backend appends the provider's own error text on a `Provider response:` line. */
  function providerResponseFromDetails(details: string | null): string | null {
    const marker = '\nProvider response: ';
    const index = details?.indexOf(marker) ?? -1;
    return details && index >= 0 ? details.slice(index + marker.length).trim() || null : null;
  }

  function formatTimestamp(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.valueOf())) return value || 'Unknown';
    return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(
      date,
    );
  }

  function copied(kind: HistoryDebugKind): boolean {
    return debugCopied?.id === entry.id && debugCopied.kind === kind;
  }
</script>

<article class="h-full min-h-0 overflow-hidden">
  <ResizableSplitPane bind:secondarySize={detailSize} label="Resize preview and details">
    {#snippet primary()}
      <div
        class="h-full min-h-0 p-2"
        aria-busy={(formatting?.loading || formatting?.previewLoading) ?? false}
      >
        {#if entry.status === 'completed'}
          <FitHtmlPreview
            class="rounded-control border-border"
            html={formatting?.previewHtml ?? ''}
            title="Rendered clean HTML"
          />
        {:else}
          <div
            class="flex h-full min-h-0 flex-col gap-3 overflow-y-auto rounded-control border border-border p-4"
          >
            {#if entry.thumbnail}
              <img
                class="max-h-44 max-w-full self-start rounded-control object-contain"
                src={entry.thumbnail}
                alt="Captured thumbnail"
              />
            {/if}
            {#if entry.status === 'pending'}
              <div role="status" aria-live="polite">
                <p class="m-0 font-semibold">Processing image…</p>
                <p class="m-0 mt-1 text-sm">Preparing the clipboard result.</p>
              </div>
            {:else}
              <div role="alert">
                <p class="m-0 font-semibold text-danger">Capture failed</p>
                <p class="m-0 mt-1 break-words text-sm">{entry.errorMessage}</p>
                <p class="m-0 mt-2 text-xs">Error code: {entry.errorCode}</p>
                {#if debug && entry.errorDetails}
                  <pre
                    class="m-0 mt-2 whitespace-pre-wrap break-words text-xs">{entry.errorDetails}</pre>
                {:else if providerResponse}
                  <p class="m-0 mt-2 text-xs font-semibold">Provider response</p>
                  <p
                    class="m-0 mt-1 line-clamp-3 break-words font-mono text-xs"
                    title={providerResponse}
                  >
                    {providerResponse}
                  </p>
                {/if}
              </div>
              <Button
                variant="outline"
                type="button"
                class="self-start"
                {disabled}
                onclick={() => onCopyFailureDetails(entry.id)}
              >
                {failureCopiedId === entry.id ? 'Details copied' : 'Copy error details'}
              </Button>
            {/if}
          </div>
        {/if}
      </div>
    {/snippet}
    {#snippet secondary()}
      <div class="h-full min-h-0 overflow-y-auto px-4 py-3">
        <dl class="m-0 grid gap-1 text-sm">
          <HistoryMetadataRow label="Profile" value={entry.processingProfile} />
          {#if entry.status !== 'pending'}
            <HistorySettingsSummary
              snapshot={entry.settingsSnapshot}
              {advanced}
              formatting={entry.status === 'completed' ? formatting : null}
              canRestoreFormatting={entry.status === 'completed' && entry.canRestoreFormatting}
              {disabled}
              onFormattingChange={(settings) => onFormattingChange(entry.id, settings)}
            />
          {/if}
          {#if entry.status === 'completed' && entry.contentTags.length > 0}
            <HistoryMetadataRow
              label="Contains"
              value={entry.contentTags.map((tag) => CONTENT_TAG_LABELS[tag]).join(', ')}
            />
          {/if}
          <HistoryMetadataRow label="Time">
            <time datetime={entry.createdAt}>{formattedTime}</time>
          </HistoryMetadataRow>

          {#if debug && (entry.status === 'completed' || (entry.status === 'failed' && entry.hasRawInferenceOutput))}
            {#if entry.status === 'completed' && entry.hasFirstInferenceOutput}
              <HistoryMetadataRow label="First-pass model output">
                <Button
                  variant="ghost"
                  size="icon-sm"
                  class={copied('first') ? 'border border-success' : ''}
                  type="button"
                  aria-label={copied('first')
                    ? 'First-pass output copied'
                    : 'Copy first-pass output as plain text'}
                  title={copied('first') ? 'Copied' : 'Copy first-pass output'}
                  {disabled}
                  onclick={() => onCopyDebugHtml(entry.id, 'first')}
                >
                  <Copy class="size-4 shrink-0" strokeWidth={2} />
                </Button>
              </HistoryMetadataRow>
            {:else}
              <HistoryMetadataRow label="Raw HTML">
                <Button
                  variant="ghost"
                  size="icon-sm"
                  class={copied('raw') ? 'border border-success' : ''}
                  type="button"
                  aria-label={copied('raw') ? 'Raw HTML copied' : 'Copy raw HTML as plain text'}
                  title={copied('raw') ? 'Copied' : 'Copy raw HTML'}
                  {disabled}
                  onclick={() => onCopyDebugHtml(entry.id, 'raw')}
                >
                  <Copy class="size-4 shrink-0" strokeWidth={2} />
                </Button>
              </HistoryMetadataRow>
            {/if}
            {#if entry.status === 'completed' && entry.hasSecondInferenceOutput}
              <HistoryMetadataRow label="Second-pass model output">
                <Button
                  variant="ghost"
                  size="icon-sm"
                  class={copied('second') ? 'border border-success' : ''}
                  type="button"
                  aria-label={copied('second')
                    ? 'Second-pass output copied'
                    : 'Copy second-pass output as plain text'}
                  title={copied('second') ? 'Copied' : 'Copy second-pass output'}
                  {disabled}
                  onclick={() => onCopyDebugHtml(entry.id, 'second')}
                >
                  <Copy class="size-4 shrink-0" strokeWidth={2} />
                </Button>
              </HistoryMetadataRow>
            {/if}
            {#if entry.status === 'completed'}
              <HistoryMetadataRow label="Clean HTML">
                <Button
                  variant="ghost"
                  size="icon-sm"
                  class={copied('clean') ? 'border border-success' : ''}
                  type="button"
                  aria-label={copied('clean')
                    ? 'Clean HTML copied'
                    : 'Copy clean HTML as plain text'}
                  title={copied('clean') ? 'Copied' : 'Copy clean HTML'}
                  {disabled}
                  onclick={() => onCopyDebugHtml(entry.id, 'clean')}
                >
                  <Copy class="size-4 shrink-0" strokeWidth={2} />
                </Button>
              </HistoryMetadataRow>
            {/if}
          {/if}
          {#if entry.status === 'completed' && advanced}
            <HistoryMetadataRow label="Image size" value={imageSize} />
            <HistoryMetadataRow
              label="Original screenshot"
              value={entry.hasOriginal ? 'Kept' : 'Not kept'}
              infoLink={docsLinks.keepOriginals}
            />
          {/if}
        </dl>
      </div>
    {/snippet}
  </ResizableSplitPane>
</article>
