<script lang="ts">
  import { productConfig } from '$lib/product-config';
  import BundledDocumentLinks from '$lib/components/BundledDocumentLinks.svelte';
</script>

<BundledDocumentLinks
  variant="ghost"
  documents={productConfig.legalDocuments}
  recovery="Could not open the terms. Open Contents/Resources/legal inside the isoboard app bundle."
/>
