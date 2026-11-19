<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';

  // Shown before a model download so the recipient accepts the model's
  // license terms, linked at the revision the app downloads.
  let {
    name,
    url,
    class: className = '',
  }: { name: string; url: string; class?: string } = $props();
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

<p class={['m-0 text-xs font-normal text-ink', className]}>
  By downloading {name}, you accept its
  <button
    type="button"
    class="cursor-pointer underline underline-offset-2 hover:no-underline focus-visible:outline-2 focus-visible:outline-offset-2"
    onclick={open}>license</button
  >.
  {#if error}<span role="alert" class="text-danger"
      >Could not open the license page. Try again.</span
    >{/if}
</p>
