<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  import { docsLinks, openDocs } from '$lib/docs-links';

  let error = $state('');
  let opening = $state(false);

  async function open() {
    if (opening) return;
    opening = true;
    error = '';
    if (!(await openDocs(docsLinks.home.url))) {
      error = `Could not open the documentation. Visit ${docsLinks.home.url} in your browser.`;
    }
    opening = false;
  }
</script>

<Button
  variant="link"
  size="sm"
  type="button"
  disabled={opening}
  aria-label={docsLinks.home.label}
  onclick={() => void open()}>Documentation</Button
>
{#if error}<p class="m-0 basis-full text-xs text-danger" role="alert">{error}</p>{/if}
