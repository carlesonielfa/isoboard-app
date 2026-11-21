<script module lang="ts">
  /** What an info button opens: its accessible label and an external URL. */
  export type InfoLinkTarget = { label: string; url: string };
</script>

<script lang="ts">
  import { Info } from '@lucide/svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import Button from '$lib/components/Button.svelte';
  import Tooltip from '$lib/components/Tooltip.svelte';

  let {
    label,
    url,
    size = 'xs',
    errorMessage = 'Could not open the page. Try again.',
    class: className = '',
  }: InfoLinkTarget & {
    /** `xs` sits beside text labels; `sm` matches other icon buttons in a toolbar. */
    size?: 'xs' | 'sm';
    errorMessage?: string;
    class?: string;
  } = $props();

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

<Tooltip content={error ? errorMessage : label}>
  {#snippet trigger(props)}
    <Button
      {...props}
      variant="ghost"
      size={size === 'xs' ? 'icon-xs' : 'icon-sm'}
      type="button"
      class={`shrink-0 ${size === 'xs' ? 'size-5' : ''} ${className}`}
      aria-label={label}
      onclick={(event) => {
        (props.onclick as ((event: MouseEvent) => void) | undefined)?.(event);
        void open();
      }}
    >
      <Info class={size === 'xs' ? 'size-3.5' : 'size-4'} strokeWidth={1.75} aria-hidden="true" />
    </Button>
  {/snippet}
</Tooltip>
{#if error}<span role="alert" class="sr-only">{errorMessage}</span>{/if}
