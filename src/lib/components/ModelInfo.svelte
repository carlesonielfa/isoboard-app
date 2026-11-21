<script lang="ts">
  import InfoLink from '$lib/components/InfoLink.svelte';

  let { url }: { url: string } = $props();
</script>

<InfoLink
  {url}
  size="sm"
  label="Model details and license on Hugging Face"
  errorMessage="Could not open the model page. Try again."
/>
