<script lang="ts">
  import * as Select from '$lib/components/ui/select/index.js';
  import InfoLink, { type InfoLinkTarget } from '$lib/components/InfoLink.svelte';
  import {
    createSettingsSelectChangeController,
    createSettingsSelectValueChangeHandler,
  } from '$lib/settings-select-state';

  export type SettingsSelectOption = {
    value: string;
    label: string;
    disabled?: boolean;
  };

  type Props = {
    label: string;
    value: string;
    id?: string;
    name?: string;
    required?: boolean;
    invalid?: boolean;
    labelHidden?: boolean;
    disabled?: boolean;
    options: readonly SettingsSelectOption[];
    placeholder?: string;
    onValueChange: (value: string) => void | Promise<void>;
    /** Adds an info button after the label that opens related documentation. */
    infoLink?: InfoLinkTarget;
  };

  let {
    label,
    value,
    id,
    name,
    required = false,
    invalid = false,
    labelHidden = false,
    disabled = false,
    options,
    placeholder: placeholderProp,
    onValueChange,
    infoLink,
  }: Props = $props();

  // svelte-ignore state_referenced_locally
  let selectedValue = $state(value);
  const changeController = createSettingsSelectChangeController(
    () => value,
    (nextValue) => {
      selectedValue = nextValue;
    },
  );
  const selectItems = $derived(options.map((option) => ({ ...option })));
  const placeholder = $derived(
    placeholderProp ?? options.find((option) => option.value === '')?.label ?? 'Select an option',
  );

  $effect(() => {
    const authoritativeValue = value;
    changeController.sync(authoritativeValue);
  });

  const handleValueChange = createSettingsSelectValueChangeHandler(
    () => disabled,
    (value) => changeController.select(value, onValueChange),
  );
</script>

<!-- The trigger carries its own accessible name, so a row with an info button
     uses a div: a label must not contain a second interactive control. -->
<svelte:element this={infoLink ? 'div' : 'label'} class="grid min-w-0 gap-1 text-xs font-semibold">
  {#if infoLink && !labelHidden}
    <span class="flex min-w-0 items-center gap-1"
      >{label}<InfoLink {...infoLink} class="-my-0.5" /></span
    >
  {:else}
    <span class:sr-only={labelHidden}>{label}</span>
  {/if}

  <Select.Root
    type="single"
    bind:value={selectedValue}
    {disabled}
    {name}
    {required}
    items={selectItems}
    onValueChange={handleValueChange}
  >
    <Select.Trigger
      {id}
      class="w-full"
      aria-label={label}
      aria-invalid={invalid || undefined}
      title={invalid && value === '' ? placeholder : undefined}
    >
      <Select.Value {placeholder} />
    </Select.Trigger>
    <Select.Content class="max-h-72">
      <Select.Group>
        {#each options as option (option.value)}
          <Select.Item value={option.value} label={option.label} disabled={option.disabled}>
            {option.label}
          </Select.Item>
        {/each}
      </Select.Group>
    </Select.Content>
  </Select.Root>
</svelte:element>
