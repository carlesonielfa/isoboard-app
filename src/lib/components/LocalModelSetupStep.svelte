<script lang="ts">
  import { ArrowRight, Cloud, Cpu, Download, LoaderCircle, ShieldCheck, X } from '@lucide/svelte';
  import Button from '$lib/components/Button.svelte';
  import ModelLicenseNotice from '$lib/components/ModelLicenseNotice.svelte';
  import ProgressBar from '$lib/components/ProgressBar.svelte';
  import SetupProgress from '$lib/components/SetupProgress.svelte';
  import type { LocalModelSnapshot } from '$lib/generated/bindings';
  import { modelSize } from '$lib/model-size';

  let {
    model,
    loading = false,
    busy = false,
    preparing = false,
    configured = false,
    canNotify = false,
    error = null,
    onUse,
    onCancel,
    onExternal,
    onBack,
  }: {
    model?: LocalModelSnapshot;
    loading?: boolean;
    busy?: boolean;
    preparing?: boolean;
    configured?: boolean;
    canNotify?: boolean;
    error?: string | null;
    onUse: () => void | Promise<void>;
    onCancel: () => void | Promise<void>;
    onExternal: () => void;
    onBack: () => void;
  } = $props();

  const active = $derived(
    model?.downloadStatus === 'downloading' || model?.downloadStatus === 'verifying',
  );
  const label = $derived(
    loading
      ? 'Loading…'
      : model?.downloaded
        ? configured
          ? 'Continue'
          : 'Use isOCR™'
        : model?.downloadStatus === 'paused'
          ? 'Resume download'
          : model?.downloadStatus === 'failed'
            ? 'Retry download'
            : 'Download isOCR™',
  );
</script>

<section class="min-h-0 flex-1 overflow-y-auto px-6 py-6" aria-label="Initial setup: isOCR™">
  <div class="mx-auto flex w-full max-w-[44rem] flex-col gap-4">
    <SetupProgress step="local" complete={['permissions']} />

    <div class="grid gap-3">
      <section
        aria-labelledby="setup-isocr-title"
        class="grid min-w-0 gap-5 rounded-surface border border-border bg-surface-control p-5 min-[640px]:min-h-56 min-[640px]:grid-cols-[minmax(0,1fr)_12rem] min-[640px]:gap-6"
      >
        <div class="grid min-w-0 grid-rows-[auto_1fr_3.25rem] gap-4">
          <div class="flex items-center gap-2 text-sm font-semibold text-ink">
            <ShieldCheck class="size-4" strokeWidth={1.75} aria-hidden="true" />
            <span>No account or API key required</span>
          </div>
          <div class="space-y-2 self-center">
            <h1
              id="setup-isocr-title"
              class="m-0 text-2xl font-semibold tracking-tight text-ink sm:text-3xl"
            >
              Download <strong class="font-bold">isOCR™</strong>
            </h1>
            <p class="m-0 max-w-[36ch] text-sm leading-relaxed text-ink">
              Turn screenshots into editable content locally.
            </p>
          </div>
          <div class="min-h-10">
            {#if active && model}
              <div class="grid gap-1.5 text-xs text-ink" role="status">
                <div class="flex flex-wrap items-center justify-between gap-x-3 gap-y-1">
                  <span
                    >{model.downloadStatus === 'verifying'
                      ? 'Verifying download…'
                      : 'Downloading isOCR™…'}</span
                  >
                  <span class="font-mono"
                    >{Math.round(
                      model.totalBytes > 0 ? (model.downloadedBytes / model.totalBytes) * 100 : 0,
                    )}%</span
                  >
                </div>
                <ProgressBar
                  value={model.totalBytes > 0
                    ? (model.downloadedBytes / model.totalBytes) * 100
                    : 0}
                  label="isOCR™ download progress"
                />
                <span>{modelSize(model.downloadedBytes)} / {modelSize(model.totalBytes)}</span>
              </div>
            {:else}
              <div class="flex min-h-10 items-center gap-2 text-xs text-ink">
                <Cpu class="size-4 shrink-0" strokeWidth={1.75} aria-hidden="true" />
                <span>Recommended for day-to-day use</span>
              </div>
            {/if}
          </div>
        </div>

        <Button
          variant={active ? 'outline' : 'primary'}
          class="min-h-28 w-full flex-col gap-3 whitespace-normal px-4 py-5 min-[640px]:min-h-40"
          type="button"
          aria-label={active
            ? 'Cancel download'
            : !model?.downloaded && model
              ? `${label} · ${modelSize(model.totalBytes)}`
              : label}
          aria-busy={busy || loading}
          disabled={busy || loading || !model || model.downloadStatus === 'removing'}
          onclick={() => void (active ? onCancel() : onUse())}
        >
          {#if busy || loading}
            <LoaderCircle
              class="size-8 animate-spin motion-reduce:animate-none"
              strokeWidth={1.5}
              aria-hidden="true"
            />
          {:else if active}
            <X class="size-8" strokeWidth={1.5} aria-hidden="true" />
          {:else if model?.downloaded}
            <ArrowRight class="size-8" strokeWidth={1.5} aria-hidden="true" />
          {:else}
            <Download class="size-8" strokeWidth={1.5} aria-hidden="true" />
          {/if}
          <span class="grid gap-1 text-center">
            <span class="text-sm font-semibold"
              >{active
                ? 'Cancel download'
                : preparing
                  ? 'Preparing isOCR™…'
                  : busy
                    ? 'Setting up…'
                    : label}</span
            >
            {#if active}
              <span class="text-xs font-normal">Keep progress for later</span>
            {:else if model && !model.downloaded}
              <span class="font-mono text-xs font-normal">{modelSize(model.totalBytes)}</span>
            {/if}
          </span>
        </Button>

        {#if model && !model.downloaded && !active}
          <ModelLicenseNotice name="isOCR™" url={model.licenseUrl} class="min-[640px]:col-span-2" />
        {/if}
        {#if active}
          <p class="m-0 text-sm text-ink min-[640px]:col-span-2" role="status">
            {canNotify
              ? 'Feel free to switch apps. We’ll notify you when it’s ready.'
              : 'Feel free to switch apps. Keep isoboard running.'}
          </p>
        {/if}
        {#if error}
          <p class="m-0 text-sm text-danger min-[640px]:col-span-2" role="alert">{error}</p>
        {:else if !loading && !model}
          <p class="m-0 text-sm text-ink min-[640px]:col-span-2">
            isOCR™ is unavailable on this device. Use an external provider to continue.
          </p>
        {/if}
      </section>

      <Button
        variant="outline"
        class="min-h-20 w-full justify-between gap-4 whitespace-normal bg-surface-control p-4 text-left"
        type="button"
        aria-label="I’ll use an external LLM provider"
        aria-describedby="setup-external-description"
        disabled={busy}
        onclick={onExternal}
      >
        <span class="flex min-w-0 items-center gap-3">
          <span
            class="flex size-10 shrink-0 items-center justify-center rounded-control border border-border bg-surface"
          >
            <Cloud class="size-5" strokeWidth={1.5} aria-hidden="true" />
          </span>
          <span class="grid gap-1">
            <span class="text-sm font-semibold">I’ll use an external LLM provider</span>
            <span id="setup-external-description" class="text-xs font-normal"
              >Choose a model for demanding captures and applying additional rules</span
            >
          </span>
        </span>
        <ArrowRight class="size-5 shrink-0" strokeWidth={1.5} aria-hidden="true" />
      </Button>
    </div>

    <div class="flex justify-start">
      <Button
        class="min-h-9 px-2 py-1.5"
        variant="ghost"
        type="button"
        disabled={busy}
        onclick={onBack}>Back</Button
      >
    </div>
  </div>
</section>
