<script lang="ts">
  import { productConfig } from '$lib/product-config';
  import { ArrowRight, KeyRound, LoaderCircle } from '@lucide/svelte';
  import Button from '$lib/components/Button.svelte';
  import ActionInput from '$lib/components/ActionInput.svelte';
  let {
    disabled = false,
    onActivate,
  }: {
    disabled?: boolean;
    onActivate: (key: string) => Promise<string | null>;
  } = $props();
  const id = $props.id();
  let key = $state('');
  let error = $state('');
  let submitting = $state(false);
  const hasKey = $derived(key.length > 0);
  const validFormat = $derived(
    key.startsWith(productConfig.licensing.keyPrefix) &&
      /^[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}$/.test(
        key.slice(productConfig.licensing.keyPrefix.length),
      ),
  );

  function checkFormat() {
    if (hasKey && !validFormat)
      error = `Enter the complete key: ${productConfig.licensing.keyPrefix}XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX.`;
  }

  async function activate() {
    if (!hasKey || disabled || submitting) return;
    if (!validFormat) {
      checkFormat();
      return;
    }
    submitting = true;
    error = '';
    try {
      const failure = await onActivate(key.trim());
      if (failure) error = failure;
      else key = '';
    } catch {
      error = 'Could not activate the license. Try again.';
    } finally {
      submitting = false;
    }
  }
</script>

<div class="grid gap-2">
  <ActionInput
    label="License key"
    placeholder="Enter license key"
    type="text"
    value={key}
    disabled={disabled || submitting}
    describedBy={error ? `${id}-error` : undefined}
    invalid={Boolean(error)}
    onblur={checkFormat}
    oninput={(value) => {
      key = value.trim().toUpperCase();
      error = '';
    }}
    onsubmit={() => void activate()}
  >
    {#snippet trailing()}
      {#if hasKey}
        <Button
          type="submit"
          variant="primary"
          size="icon-sm"
          disabled={disabled || submitting || !validFormat}
          aria-label="Activate commercial license"
        >
          {#if submitting}<LoaderCircle
              class="size-4 animate-spin motion-reduce:animate-none"
              aria-hidden="true"
            />{:else}<ArrowRight class="size-4" aria-hidden="true" />{/if}
        </Button>
      {:else}
        <KeyRound class="size-4" strokeWidth={1.75} aria-hidden="true" />
      {/if}
    {/snippet}
  </ActionInput>
  {#if error}
    <p class="m-0 text-xs leading-relaxed text-danger" id={`${id}-error`} role="alert">{error}</p>
  {/if}
</div>
