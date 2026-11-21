<script lang="ts">
  import { productConfig } from '$lib/product-config';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { Heart, KeyRound, Mail, ShoppingCart } from '@lucide/svelte';
  import Badge from '$lib/components/Badge.svelte';
  import SettingsGroup from '$lib/components/SettingsGroup.svelte';
  import SettingsRow from '$lib/components/SettingsRow.svelte';
  import Button from '$lib/components/Button.svelte';
  import LicenseActivation from '$lib/components/LicenseActivation.svelte';
  import LicenseDocuments from '$lib/components/LicenseDocuments.svelte';
  import type { LicensingController } from '$lib/licensing.svelte';
  import { docsLinks } from '$lib/docs-links';
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
  const FREE_STATUS = 'Free for personal use. License required for commercial use.';
  const statusText = $derived.by(() => {
    if (!licensing.state.loaded) return 'Loading license status…';
    if (snapshot?.commercial.active) return 'Licensed for personal and commercial use.';
    if (snapshot?.evaluation === 'active')
      return `Commercial trial ends ${date(snapshot.expiresAt)}. Personal use is free.`;
    if (snapshot?.evaluation === 'expired' || snapshot?.evaluation === 'notStarted')
      return FREE_STATUS;
    return 'License status unavailable. Personal use is free.';
  });
  const verificationError = $derived(
    licensing.state.error || snapshot?.commercial.error || snapshot?.error || '',
  );
  const supportText = $derived.by(() => {
    const expiresAt = snapshot?.commercial.supportExpiresAt;
    if (!expiresAt) return 'Status unavailable.';
    const day = new Date(expiresAt * 1000).toLocaleDateString(undefined, { dateStyle: 'medium' });
    return snapshot?.commercial.supportActive ? `Active until ${day}.` : `Expired on ${day}.`;
  });
</script>

{#snippet statusErrors()}
  {#if verificationError}<p class="m-0 text-xs text-danger" role="alert">
      {verificationError}
    </p>{/if}
  {#if purchaseError}<p class="m-0 text-xs text-danger" role="alert">{purchaseError}</p>{/if}
{/snippet}

{#snippet supportErrors()}
  <p class="m-0 text-xs text-danger" role="alert">{supportError}</p>
{/snippet}

{#snippet freeStatus()}
  <span class="inline-flex flex-wrap items-center gap-x-1.5 gap-y-1">
    <Badge size="sm">
      <Heart class="mr-1.5 size-3.5" strokeWidth={1.75} aria-hidden="true" />Free
    </Badge>
    for personal use. License required for commercial use.
  </span>
{/snippet}

<SettingsGroup title="License" icon={KeyRound} infoLink={docsLinks.license}>
  <SettingsRow
    label={statusText}
    labelContent={statusText === FREE_STATUS ? freeStatus : undefined}
    below={verificationError || purchaseError ? statusErrors : undefined}
  >
    {#snippet control()}
      {#if verificationError}
        <Button
          variant="outline"
          size="sm"
          disabled={licensing.state.busy || snapshot?.commercial.checking}
          onclick={() =>
            snapshot?.commercial.hasSavedKey ? licensing.recheck() : licensing.refresh()}
          >Retry</Button
        >
      {/if}
      {#if snapshot?.evaluation === 'notStarted' && !snapshot.commercial.choiceCompleted}
        <Button
          variant="outline"
          size="sm"
          disabled={licensing.state.busy}
          onclick={() => licensing.continueWithoutKey()}>Continue without a license key</Button
        >
      {/if}
      {#if snapshot?.commercial.hasSavedKey}
        <Button
          variant="outline"
          size="sm"
          disabled={licensing.state.busy || snapshot.commercial.checking}
          onclick={() => licensing.remove()}>Remove license</Button
        >
      {/if}
      {#if licensing.state.loaded && !snapshot?.commercial.active}
        <Button variant="outline" size="sm" disabled={openingCheckout} onclick={buyLicense}>
          <ShoppingCart class="size-4" strokeWidth={1.75} aria-hidden="true" />
          Get a license
        </Button>
      {/if}
    {/snippet}
  </SettingsRow>
  {#if snapshot?.commercial.active}
    <SettingsRow
      label="Priority Email Support"
      description={supportText}
      below={supportError ? supportErrors : undefined}
      infoLink={docsLinks.prioritySupport}
    >
      {#snippet control()}
        <Button variant="outline" size="sm" disabled={openingEmail} onclick={contactSupport}>
          <Mail class="size-4" strokeWidth={1.75} aria-hidden="true" />
          Email support
        </Button>
      {/snippet}
    </SettingsRow>
  {:else if licensing.state.loaded}
    <SettingsRow label="License key" infoLink={docsLinks.licenseKey}>
      {#snippet control()}
        <div class="w-72">
          <LicenseActivation disabled={licensing.state.busy} onActivate={activate} />
        </div>
      {/snippet}
    </SettingsRow>
  {/if}
  {#snippet footer()}
    <LicenseDocuments variant="link" />
  {/snippet}
</SettingsGroup>
