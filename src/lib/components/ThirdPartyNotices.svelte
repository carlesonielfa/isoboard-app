<script lang="ts">
  import { productConfig } from '$lib/product-config';
  import BundledDocumentLinks from '$lib/components/BundledDocumentLinks.svelte';
  let { disabled = false }: { disabled?: boolean } = $props();
</script>

<BundledDocumentLinks
  variant="link"
  {disabled}
  documents={productConfig.thirdPartyDocuments}
  recovery="Could not open the document. In Finder, right-click isoboard.app, choose Show Package Contents, then open Contents/Resources/legal."
/>
