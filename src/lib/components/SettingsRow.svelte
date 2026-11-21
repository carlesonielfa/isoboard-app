<script lang="ts">
  import type { Snippet } from 'svelte';
  import { ChevronDown } from '@lucide/svelte';
  import InfoLink, { type InfoLinkTarget } from '$lib/components/InfoLink.svelte';

  let {
    label,
    labelFor,
    description,
    details,
    collapsible = false,
    labelContent,
    leading,
    control,
    below,
    disabled = false,
    infoLink,
  }: {
    label: string;
    /** Id of the control the label activates; omit when the control labels itself. */
    labelFor?: string;
    description?: string;
    /** Rich content shown in place of `description`. */
    details?: Snippet;
    /** Makes the label a toggle that shows or hides the description or details. */
    collapsible?: boolean;
    /** Rich label shown in place of the `label` text; `label` remains the accessible name. */
    labelContent?: Snippet;
    leading?: Snippet;
    control?: Snippet;
    /** Full-width content under the row, such as errors or a disclosure. */
    below?: Snippet;
    /** Dims the whole row for a control that cannot be used right now. */
    disabled?: boolean;
    /** Adds an info button after the label that opens related documentation. */
    infoLink?: InfoLinkTarget;
  } = $props();

  const id = $props.id();
  let expanded = $state(false);
  const hasInfo = $derived(Boolean(details || description));
  const infoVisible = $derived(hasInfo && (!collapsible || expanded));
</script>

<div class={`grid min-w-0 gap-2 px-4 py-2.5 ${disabled ? 'opacity-45' : ''}`}>
  <div class="flex min-h-9 min-w-0 flex-wrap items-center gap-x-4 gap-y-2">
    {#if leading}<div class="shrink-0">{@render leading()}</div>{/if}
    <div class="grid min-w-48 flex-1 gap-0.5">
      <div class="flex min-w-0 items-center gap-1">
        {#if collapsible && hasInfo}
          <button
            type="button"
            class="inline-flex w-fit cursor-pointer items-center gap-1.5 rounded-sm text-left text-sm text-ink outline-none hover:text-ink/80 focus-visible:ring-2 focus-visible:ring-focus/30"
            aria-expanded={expanded}
            aria-controls={`${id}-info`}
            onclick={() => (expanded = !expanded)}
          >
            {label}
            <ChevronDown
              aria-hidden="true"
              class={`size-4 transition-transform duration-150 motion-reduce:transition-none ${expanded ? 'rotate-180' : ''}`}
            />
          </button>
        {:else if labelFor}
          <label for={labelFor} class="cursor-pointer text-sm">{label}</label>
        {:else if labelContent}
          <span class="text-sm" aria-label={label}>{@render labelContent()}</span>
        {:else}
          <span class="text-sm">{label}</span>
        {/if}
        {#if infoLink}<InfoLink {...infoLink} />{/if}
      </div>
      {#if infoVisible}
        <div id={`${id}-info`} class="min-w-0 text-xs leading-relaxed">
          {#if details}{@render details()}{:else}<p class="m-0">{description}</p>{/if}
        </div>
      {/if}
    </div>
    {#if control}
      <div class="flex shrink-0 items-center gap-2">{@render control()}</div>
    {/if}
  </div>
  {#if below}
    <div class="min-w-0">{@render below()}</div>
  {/if}
</div>
