<script lang="ts">
  import { Check, CircleAlert, LoaderCircle, LockKeyhole } from '@lucide/svelte';

  let {
    status,
    label,
    size = 'sm',
  }: {
    status: 'success' | 'busy' | 'locked' | 'error';
    label: string;
    size?: 'sm' | 'xs';
  } = $props();

  const icon = $derived(size === 'xs' ? 'size-3.5' : 'size-4');
</script>

<span
  class={`inline-flex min-w-0 items-center ${size === 'xs' ? 'gap-1.5 text-xs' : 'gap-2 text-sm'} ${status === 'error' ? 'text-danger' : 'text-ink'}`}
  role={status === 'error' ? 'alert' : 'status'}
  aria-live={status === 'error' ? 'assertive' : 'polite'}
>
  {#if status === 'success'}
    <Check class={`${icon} shrink-0 text-success`} aria-hidden="true" />
  {:else if status === 'busy'}
    <LoaderCircle
      class={`${icon} shrink-0 animate-spin motion-reduce:animate-none`}
      aria-hidden="true"
    />
  {:else if status === 'error'}
    <CircleAlert class={`${icon} shrink-0`} aria-hidden="true" />
  {:else}
    <LockKeyhole class={`${icon} shrink-0`} aria-hidden="true" />
  {/if}
  <span class="min-w-0">{label}</span>
</span>
