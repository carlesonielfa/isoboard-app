<script lang="ts">
  import * as Select from '$lib/components/ui/select/index.js';
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

<label class="grid min-w-0 gap-1 text-xs font-semibold">
  <span class:sr-only={labelHidden}>{label}</span>

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
</label>
