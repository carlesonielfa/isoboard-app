<script lang="ts">
  import { productConfig } from '$lib/product-config';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { Mail, ShoppingCart } from '@lucide/svelte';
  import SettingsSection from '$lib/components/SettingsSection.svelte';
  import Button from '$lib/components/Button.svelte';
  import LicenseActivation from '$lib/components/LicenseActivation.svelte';
  import LicenseDocuments from '$lib/components/LicenseDocuments.svelte';
  import type { LicensingController } from '$lib/licensing.svelte';
  let { licensing }: { licensing: LicensingController } = $props();
  let purchaseError = $state('');
  let openingCheckout = $state(false);
  let supportError = $state('');
  let openingEmail = $state(false);

  async function contactSupport() {
    if (openingEmail) return;
    openingEmail = true;
    supportError = '';
    try {
      const subject = encodeURIComponent('isoboard support');
      const body = encodeURIComponent('Hi,\n\nI need help with:\n\n\nPurchase email: ');
      await openUrl(`mailto:${productConfig.supportEmail}?subject=${subject}&body=${body}`);
    } catch {
      supportError = `Could not open your email app. Email ${productConfig.supportEmail}.`;
    } finally {
      openingEmail = false;
    }
  }

  async function buyLicense() {
    if (openingCheckout) return;
    openingCheckout = true;
    purchaseError = '';
    try {
      await openUrl(productConfig.checkoutUrl);
    } catch {
      purchaseError = 'Could not open checkout. Try again.';
    } finally {
      openingCheckout = false;
    }
  }
  async function activate(key: string): Promise<string | null> {
    if (await licensing.activate(key)) return null;
    const error = licensing.state.error || 'Could not activate the license. Try again.';
    licensing.state.error = '';
    return error;
  }

  const snapshot = $derived(licensing.state.snapshot);
  function date(seconds: number | null | undefined) {
    if (!seconds) return 'unavailable';
    const value = new Date(seconds * 1000);
    return Number.isNaN(value.getTime())
      ? 'unavailable'
      : value.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
  }
</script>

<SettingsSection id="license-heading" title="License">
  <div class="grid gap-3 py-2 text-sm text-ink">
    {#if !licensing.state.loaded}
      <p class="m-0" role="status">Loading license status…</p>
    {:else if snapshot?.commercial.active}
      <p class="m-0">Licensed for personal and commercial use.</p>
    {:else if snapshot?.evaluation === 'active'}
      <p class="m-0">Commercial trial ends {date(snapshot.expiresAt)}. Personal use is free.</p>
    {:else if snapshot?.evaluation === 'expired' || snapshot?.evaluation === 'notStarted'}
      <p class="m-0">Free for personal use. License required for commercial use.</p>
    {:else}
      <p class="m-0">License status unavailable. Personal use is free.</p>
    {/if}
    {#if snapshot?.evaluation === 'notStarted' && !snapshot.commercial.choiceCompleted}
      <div>
        <Button disabled={licensing.state.busy} onclick={() => licensing.continueWithoutKey()}
          >Continue without a license key</Button
        >
      </div>
    {/if}
    {#if licensing.state.error || snapshot?.commercial.error || snapshot?.error}
      <p class="m-0" role="alert">
        {licensing.state.error || snapshot?.commercial.error || snapshot?.error}
      </p>
      <div>
        <Button
          variant="outline"
          size="sm"
          disabled={licensing.state.busy || snapshot?.commercial.checking}
          onclick={() =>
            snapshot?.commercial.hasSavedKey ? licensing.recheck() : licensing.refresh()}
          >Retry</Button
        >
      </div>
    {/if}
    {#if snapshot?.commercial.active}
      <div class="flex flex-wrap items-center gap-3">
        <p class="m-0 text-sm">
          {#if snapshot.commercial.supportExpiresAt}
            Priority Email Support {snapshot.commercial.supportActive
              ? 'active until'
              : 'expired on'}
            {new Date(snapshot.commercial.supportExpiresAt * 1000).toLocaleDateString(undefined, {
              dateStyle: 'medium',
            })}.
          {:else}
            Priority Email Support status unavailable.
          {/if}
        </p>
        <Button variant="outline" size="sm" disabled={openingEmail} onclick={contactSupport}>
          <Mail class="size-4" strokeWidth={1.75} aria-hidden="true" />
          Email support
        </Button>
      </div>
      {#if supportError}<p class="m-0 text-xs text-danger" role="alert">{supportError}</p>{/if}
    {:else}
      <div class="flex flex-wrap items-start gap-3">
        <Button variant="primary" disabled={openingCheckout} onclick={buyLicense}>
          <ShoppingCart class="size-4" strokeWidth={1.75} aria-hidden="true" />
          Buy a license
        </Button>
        <div class="min-w-52 flex-1">
          <LicenseActivation disabled={licensing.state.busy} onActivate={activate} />
        </div>
      </div>
      {#if purchaseError}<p class="m-0 text-xs text-danger" role="alert">{purchaseError}</p>{/if}
    {/if}
    <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
      {#if snapshot?.commercial.hasSavedKey}
        <Button
          variant="outline"
          size="sm"
          disabled={licensing.state.busy || snapshot.commercial.checking}
          onclick={() => licensing.remove()}>Remove license</Button
        >
      {/if}
      <LicenseDocuments />
    </div>
  </div>
</SettingsSection>
