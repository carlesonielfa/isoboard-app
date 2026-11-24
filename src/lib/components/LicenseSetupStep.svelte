<script lang="ts">
  import { productConfig } from '$lib/product-config';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { isTauri } from '@tauri-apps/api/core';
  import { ArrowRight, BriefcaseBusiness, ShoppingCart, UserRound } from '@lucide/svelte';
  import { commands, type LicenseSnapshot } from '$lib/generated/bindings';
  import Button from '$lib/components/Button.svelte';
  import CheckList from '$lib/components/CheckList.svelte';
  import ChoiceCard from '$lib/components/ChoiceCard.svelte';
  import LicenseActivation from '$lib/components/LicenseActivation.svelte';
  import LicenseDocuments from '$lib/components/LicenseDocuments.svelte';
  import InfoLink from '$lib/components/InfoLink.svelte';
  import { docsLinks } from '$lib/docs-links';
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
  const { trialDays, supportMonths } = productConfig.licensing;
  const personalFeatures = [
    'Every feature in the app',
    'Local isOCR with no usage fees',
    'Bring your own AI provider',
  ];
  const commercialFeatures = [`${supportMonths} months of priority support`, 'No device limit'];
  const licensed = $derived(snapshot?.commercial.active ?? false);
  const evaluation = $derived(snapshot?.evaluation ?? 'unavailable');
  const trialAvailable = $derived(evaluation === 'notStarted' || evaluation === 'active');
  const expiry = $derived(
    snapshot?.expiresAt
      ? new Date(snapshot.expiresAt * 1000).toLocaleString(undefined, {
          dateStyle: 'medium',
          timeStyle: 'short',
        })
      : '',
  );
  // Undefined while loading; null when Polar cannot provide the price.
  let seatPrice = $state<string | null | undefined>(undefined);
  let priceRequested = false;
  $effect(() => {
    if (licensed || priceRequested) return;
    priceRequested = true;
    void loadSeatPrice();
  });

  async function loadSeatPrice() {
    if (!isTauri()) {
      seatPrice = null;
      return;
    }
    try {
      const price = await commands.getCommercialSeatPrice();
      seatPrice = price
        ? new Intl.NumberFormat(undefined, {
            style: 'currency',
            currency: price.currency,
            minimumFractionDigits: price.amountMinor % 100 === 0 ? 0 : 2,
          }).format(price.amountMinor / 100)
        : null;
    } catch {
      seatPrice = null;
    }
  }

  let openingCheckout = $state(false);
  let checkoutError = $state('');

  async function buyLicense() {
    if (openingCheckout) return;
    openingCheckout = true;
    checkoutError = '';
    try {
      await openUrl(productConfig.checkoutUrl);
    } catch {
      checkoutError = 'Could not open checkout. Try again.';
    } finally {
      openingCheckout = false;
    }
  }
</script>

<section
  class="min-h-0 flex-1 overflow-y-auto px-6 py-6 text-ink"
  aria-label="Initial setup: license"
>
  <div class="mx-auto flex w-full max-w-[48rem] flex-col gap-5">
    <h1 class="m-0 flex items-center gap-2 font-display text-2xl font-semibold tracking-tight">
      How will you use isoboard?
      <InfoLink {...docsLinks.licenseChoice} size="sm" />
    </h1>
    <div class="grid grid-cols-1 gap-4 min-[640px]:grid-cols-2">
      <ChoiceCard title="Personal" description="For you, at home and for your own projects.">
        {#snippet icon()}<UserRound class="size-6" strokeWidth={1.5} aria-hidden="true" />{/snippet}
        <div class="grid gap-4">
          <p class="m-0 font-display text-3xl font-semibold tracking-tight">Free</p>
          <CheckList items={personalFeatures} />
        </div>
        {#snippet actions()}
          {#if !licensed}<Button
              variant="primary"
              class="min-h-11 w-full justify-between gap-2 px-3"
              disabled={busy}
              onclick={onContinue}
            >
              <span>Continue for personal use</span><ArrowRight
                class="size-4 shrink-0"
                aria-hidden="true"
              />
            </Button>{/if}
        {/snippet}
      </ChoiceCard>
      <ChoiceCard
        title="Commercial"
        description={licensed
          ? 'Licensed for personal and commercial use.'
          : 'For work, including freelance and organizations.'}
      >
        {#snippet icon()}<BriefcaseBusiness
            class="size-6"
            strokeWidth={1.5}
            aria-hidden="true"
          />{/snippet}
        <div class="grid gap-4">
          {#if !licensed}
            <div class="flex h-9 items-end" aria-live="polite">
              {#if seatPrice === undefined}
                <span
                  class="mb-0.5 h-8 w-28 animate-pulse rounded-md bg-ink/10"
                  aria-label="Loading price"
                ></span>
              {:else if seatPrice}
                <p class="m-0 text-sm">
                  <span class="mr-1 font-display text-3xl font-semibold tracking-tight"
                    >{seatPrice}</span
                  >per user
                </p>
              {:else}
                <p class="m-0 text-xs">Price shown at checkout.</p>
              {/if}
            </div>
          {/if}
          <CheckList items={commercialFeatures} />
        </div>
        {#snippet actions()}
          {#if licensed}
            <Button
              variant="primary"
              class="min-h-11 w-full justify-between"
              disabled={busy}
              onclick={onContinue}>Continue<ArrowRight class="size-4" aria-hidden="true" /></Button
            >
          {:else}
            <div class="grid gap-2.5">
              {#if trialAvailable}
                <Button
                  variant="primary"
                  class="min-h-11 w-full justify-between gap-2 px-3"
                  disabled={busy}
                  onclick={onContinue}
                >
                  <span
                    >{evaluation === 'notStarted'
                      ? `Start ${trialDays}-day trial`
                      : 'Continue trial'}</span
                  ><ArrowRight class="size-4 shrink-0" aria-hidden="true" />
                </Button>
                {#if evaluation === 'active'}<p class="m-0 text-xs">Trial ends {expiry}.</p>{/if}
              {:else if evaluation === 'expired'}
                <p class="m-0 text-xs">Trial ended.</p>
              {/if}
              <Button
                variant={trialAvailable ? 'outline' : 'primary'}
                class="min-h-11 w-full justify-between gap-2 px-3"
                disabled={openingCheckout}
                onclick={() => void buyLicense()}
              >
                <span>Get a license</span><ShoppingCart
                  class="size-4 shrink-0"
                  strokeWidth={1.75}
                  aria-hidden="true"
                />
              </Button>
              {#if checkoutError}<p class="m-0 text-xs text-danger" role="alert">
                  {checkoutError}
                </p>{/if}
              <LicenseActivation disabled={busy} {onActivate} />
            </div>
          {/if}
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
