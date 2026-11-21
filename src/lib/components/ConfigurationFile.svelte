<script lang="ts">
  import { FolderOpen } from '@lucide/svelte';
  import { revealItemInDir } from '@tauri-apps/plugin-opener';
  import Button from '$lib/components/Button.svelte';
  import SettingsRow from '$lib/components/SettingsRow.svelte';
  import { docsLinks } from '$lib/docs-links';

  let { path, disabled = false }: { path: string; disabled?: boolean } = $props();
  let error = $state('');

  async function reveal() {
    error = '';
    try {
      await revealItemInDir(path);
    } catch {
      error = 'Could not show the file. Open the path above in your file manager.';
    }
  }
</script>

{#snippet failure()}
  <p class="m-0 text-xs text-danger" role="alert">{error}</p>
{/snippet}

{#if path}
  <SettingsRow
    label="Configuration file"
    collapsible
    below={error ? failure : undefined}
    infoLink={docsLinks.configurationFile}
  >
    {#snippet details()}
      <span class="select-text font-mono [overflow-wrap:anywhere]">{path}</span>
    {/snippet}
    {#snippet control()}
      <Button variant="outline" size="sm" type="button" {disabled} onclick={() => void reveal()}>
        <FolderOpen class="size-4" strokeWidth={1.75} aria-hidden="true" />
        Show file
      </Button>
    {/snippet}
  </SettingsRow>
{/if}
