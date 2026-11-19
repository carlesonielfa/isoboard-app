<script lang="ts">
  import { Download, Trash2, X } from '@lucide/svelte';
  import Button from '$lib/components/Button.svelte';
  import ProgressBar from '$lib/components/ProgressBar.svelte';
  import { useLocalModels } from '$lib/local-models-controller.svelte';
  import ModelInfo from '$lib/components/ModelInfo.svelte';
  import ModelLicenseNotice from '$lib/components/ModelLicenseNotice.svelte';
  import { modelSize } from '$lib/model-size';
  let {
    model,
    disabled = false,
    feedbackOnly = false,
  }: {
    model: string;
    disabled?: boolean;
    feedbackOnly?: boolean;
  } = $props();
  const context = useLocalModels();
  const state = $derived(context?.controller.state);
  const status = $derived(state?.snapshot.models.find((item) => item.id === model));
  const pending = $derived(state?.pending[model] ?? false);
  const active = $derived(
    status?.downloadStatus === 'downloading' || status?.downloadStatus === 'verifying',
  );
  const resourceBusy = $derived(status?.inUse || context?.isCapturing());
  const count = $derived(context?.profileCount(model) ?? 0);
  const error = $derived(state?.errors[model] ?? status?.error ?? state?.error);
  const description = $derived(
    active
      ? 'Cancel download; keep partial files for resume'
      : status?.downloaded
        ? `Remove downloaded model${count > 1 ? ` shared by ${count} profiles` : ''}`
        : status?.downloadStatus === 'failed'
          ? 'Retry model download'
          : status?.downloadStatus === 'paused'
            ? 'Resume model download'
            : 'Download model',
  );
</script>

{#if feedbackOnly}
  {#if active && status}
    <div class="grid min-w-0 gap-1 text-xs font-normal" role="status">
      <ProgressBar
        value={status.totalBytes > 0 ? (status.downloadedBytes / status.totalBytes) * 100 : 0}
        label="Model download progress"
      />
      <span
        >{status.downloadStatus === 'verifying' ? 'Verifying' : 'Downloading'} · {modelSize(
          status.downloadedBytes,
        )} / {modelSize(status.totalBytes)}</span
      >
      <span
        >{context?.canNotify()
          ? 'Feel free to switch apps. We’ll notify you when it’s ready.'
          : 'Feel free to switch apps. Keep isoboard running.'}</span
      >
    </div>
  {/if}
  {#if status && !status.downloaded && !active}
    <ModelLicenseNotice name={status.name} url={status.licenseUrl} />
  {/if}
  {#if error}<p role="alert" class="m-0 text-xs font-normal text-danger">{error}</p>{/if}
{:else if status && context}
  <ModelInfo url={status.repositoryUrl} />
  <span title={description} class="inline-flex shrink-0">
    <Button
      variant={!active && !status.downloaded ? 'primary' : 'ghost'}
      size="icon-sm"
      aria-label={description}
      disabled={disabled ||
        pending ||
        status.downloadStatus === 'removing' ||
        (!active && status.downloaded && resourceBusy)}
      onclick={() =>
        void context.controller.act(
          model,
          active ? 'cancel' : status.downloaded ? 'remove' : 'download',
        )}
    >
      {#if active}<X class="size-4" aria-hidden="true" />
      {:else if status.downloaded}<Trash2 class="size-4" aria-hidden="true" />
      {:else}<Download class="size-4" aria-hidden="true" />{/if}
    </Button>
  </span>
{/if}
