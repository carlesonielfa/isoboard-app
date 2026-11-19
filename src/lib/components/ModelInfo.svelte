<script lang="ts">
  import { Info } from '@lucide/svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import Button from '$lib/components/Button.svelte';

  let { url }: { url: string } = $props();
  let error = $state(false);
  $effect(() => {
    void url;
    error = false;
  });

  async function open() {
    error = false;
    try {
      await openUrl(url);
    } catch {
      error = true;
    }
  }
</script>

<span
  class="inline-flex shrink-0"
  title={error
    ? 'Could not open the model page. Try again.'
    : 'Model details and license on Hugging Face'}
>
  <Button
    variant="ghost"
    size="icon-sm"
    aria-label="Model details and license on Hugging Face"
    onclick={open}
  >
    <Info class="size-4" aria-hidden="true" />
  </Button>
  {#if error}<span role="alert" class="sr-only">Could not open the model page. Try again.</span
    >{/if}
</span>
