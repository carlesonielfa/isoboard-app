<script lang="ts">
  import { productConfig } from '$lib/product-config';
  import { ArrowRight, BriefcaseBusiness, UserRound } from '@lucide/svelte';
  import type { LicenseSnapshot } from '$lib/generated/bindings';
  import Button from '$lib/components/Button.svelte';
  import ChoiceCard from '$lib/components/ChoiceCard.svelte';
  import LicenseActivation from '$lib/components/LicenseActivation.svelte';
  import LicenseDocuments from '$lib/components/LicenseDocuments.svelte';
  let {
    snapshot,
    busy,
    error,
    onContinue,
    onActivate,
  }: {
    snapshot: LicenseSnapshot | null;
    busy: boolean;
    error: string;
    onContinue: () => void;
    onActivate: (key: string) => Promise<string | null>;
  } = $props();
  const licensed = $derived(snapshot?.commercial.active ?? false);
  const evaluation = $derived(snapshot?.evaluation ?? 'unavailable');
  const expiry = $derived(
    snapshot?.expiresAt
      ? new Date(snapshot.expiresAt * 1000).toLocaleString(undefined, {
          dateStyle: 'medium',
          timeStyle: 'short',
        })
      : '',
  );
</script>

<section
  class="min-h-0 flex-1 overflow-y-auto px-6 py-6 text-ink"
  aria-label="Initial setup: license"
>
  <div class="mx-auto flex w-full max-w-[44rem] flex-col gap-5">
    <h1 class="m-0 font-display text-2xl font-semibold tracking-tight">
      How will you use isoboard?
    </h1>
    <div class="grid grid-cols-1 gap-4 min-[640px]:grid-cols-2">
      <ChoiceCard title="Personal use" description="Free for personal, non-commercial use.">
        {#snippet icon()}<UserRound class="size-8" strokeWidth={1.5} aria-hidden="true" />{/snippet}
        {#if !licensed}
          <div class="grid gap-1.5">
            {#if evaluation === 'notStarted'}
              <p class="m-0 text-sm font-semibold">
                Includes a {productConfig.licensing.trialDays}-day work evaluation
              </p>
              <p class="m-0 text-xs leading-relaxed">
                Starts when you continue. After {productConfig.licensing.trialDays} days, continued work
                use requires a commercial license.
              </p>
            {:else if evaluation === 'active'}
              <p class="m-0 text-sm font-semibold">Commercial trial ends {expiry}.</p>
            {:else if evaluation === 'expired'}
              <p class="m-0 text-sm font-semibold">Commercial trial ended.</p>
              <p class="m-0 text-xs leading-relaxed">License required for commercial use.</p>
            {:else}
              <p class="m-0 text-xs leading-relaxed">
                Trial status unavailable. Personal use remains free.
              </p>
            {/if}
          </div>
        {/if}
        {#snippet actions()}
          {#if !licensed}<Button
              variant="primary"
              class="min-h-11 w-full justify-between gap-2 px-3"
              disabled={busy}
              onclick={onContinue}
            >
              <span>Continue without a license key</span><ArrowRight
                class="size-4 shrink-0"
                aria-hidden="true"
              />
            </Button>{/if}
        {/snippet}
      </ChoiceCard>
      <ChoiceCard
        title="Commercial use"
        description={licensed
          ? 'Licensed for personal and commercial use.'
          : 'Activate your license to use isoboard for work.'}
      >
        {#snippet icon()}<BriefcaseBusiness
            class="size-8"
            strokeWidth={1.5}
            aria-hidden="true"
          />{/snippet}
        {#snippet actions()}
          {#if licensed}
            <Button
              variant="primary"
              class="min-h-11 w-full justify-between"
              disabled={busy}
              onclick={onContinue}>Continue<ArrowRight class="size-4" aria-hidden="true" /></Button
            >
          {:else}<LicenseActivation disabled={busy} {onActivate} />{/if}
        {/snippet}
      </ChoiceCard>
    </div>
    {#if error}<p class="m-0 text-sm text-danger" role="alert">{error}</p>{/if}
    <div class="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
      <details class="min-w-0 flex-1 pt-3 text-xs leading-relaxed">
        <summary class="cursor-pointer font-medium">What counts as work use?</summary>
        <p class="mb-0 max-w-prose">
          Work use includes employment, freelance or professional activities, and work for an
          organization, including nonprofits and public institutions. Personal study and unpaid
          student coursework are personal use.
        </p>
      </details>
      <LicenseDocuments />
    </div>
  </div>
</section>
