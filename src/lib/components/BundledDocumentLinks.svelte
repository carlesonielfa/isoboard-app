<script lang="ts">
  import { resolveResource } from '@tauri-apps/api/path';
  import { openPath } from '@tauri-apps/plugin-opener';
  import Button from '$lib/components/Button.svelte';

  let {
    documents,
    disabled = false,
    variant = 'outline',
    recovery,
  }: {
    documents: { label: string; resource: string }[];
    disabled?: boolean;
    variant?: 'outline' | 'ghost' | 'link';
    recovery: string;
  } = $props();
  let error = $state('');

  async function openDocument(resource: string) {
    error = '';
    try {
      await openPath(await resolveResource(resource));
    } catch {
      error = recovery;
    }
  }
</script>

<div class="flex flex-col gap-2 py-2 text-ink">
  <div class={`flex flex-wrap ${variant === 'link' ? 'gap-x-5 gap-y-2' : 'gap-2'}`}>
    {#each documents as document (document.resource)}
      <Button {variant} size="sm" {disabled} onclick={() => openDocument(document.resource)}
        >{document.label}</Button
      >
    {/each}
  </div>
  {#if error}<p role="alert" class="m-0 text-sm">{error}</p>{/if}
</div>
