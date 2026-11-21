<script lang="ts">
  import { Switch } from '$lib/components/ui/switch';
  import SettingsRow from '$lib/components/SettingsRow.svelte';
  import InfoLink, { type InfoLinkTarget } from '$lib/components/InfoLink.svelte';

  let {
    label,
    description,
    checked,
    disabled = false,
    variant = 'inline',
    collapsible = false,
    onCheckedChange,
    infoLink,
  }: {
    label: string;
    /** Shown under the label in the `row` variant. */
    description?: string;
    checked: boolean;
    disabled?: boolean;
    /** `inline` puts the switch before its label; `row` is a `SettingsRow` with a trailing switch. */
    variant?: 'inline' | 'row';
    /** Hides the `row` description behind a chevron. */
    collapsible?: boolean;
    onCheckedChange: (checked: boolean) => void;
    /** Adds an info button after the label that opens related documentation. */
    infoLink?: InfoLinkTarget;
  } = $props();

  const id = $props.id();
</script>

{#if variant === 'row'}
  <SettingsRow {label} labelFor={id} {description} {collapsible} {disabled} {infoLink}>
    {#snippet control()}
      <Switch {id} aria-label={label} {checked} {disabled} {onCheckedChange} />
    {/snippet}
  </SettingsRow>
{:else}
  <div
    class="group flex items-center gap-2 text-sm has-disabled:cursor-not-allowed has-disabled:opacity-45"
  >
    <Switch {id} {checked} {disabled} {onCheckedChange} />
    <label for={id} class="cursor-pointer group-has-disabled:cursor-not-allowed">{label}</label>
    {#if infoLink}<InfoLink {...infoLink} class="-ml-1" />{/if}
  </div>
{/if}
