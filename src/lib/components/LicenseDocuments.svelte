<script lang="ts">
  import { productConfig } from '$lib/product-config';
  import BundledDocumentLinks from '$lib/components/BundledDocumentLinks.svelte';

  let { variant = 'ghost' }: { variant?: 'ghost' | 'link' } = $props();
</script>

<BundledDocumentLinks
  {variant}
  documents={productConfig.legalDocuments}
  recovery="Could not open the terms. Open Contents/Resources/legal inside the isoboard app bundle."
/>
