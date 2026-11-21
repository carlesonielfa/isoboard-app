<script lang="ts">
  import { presentShortcut } from '$lib/shortcut-presentation';

  let { shortcut, class: className = '' }: { shortcut: string; class?: string } = $props();

  const presented = $derived(presentShortcut(shortcut, 'macos'));
</script>

<p class={className}>
  Press your keyboard shortcut (<span class="font-mono text-ink"
    >{presented.display || shortcut}</span
  >) or the top-right Capture button to start a capture.
</p>
