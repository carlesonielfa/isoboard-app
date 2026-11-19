<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';
  let {
    label,
    value,
    type = 'text',
    disabled = false,
    autocomplete,
    inputmode,
    placeholder,
    errorId,
    error,
    oninput,
    onblur,
  }: {
    label: string;
    value: string;
    type?: 'text' | 'password' | 'number';
    disabled?: boolean;
    autocomplete?: HTMLInputAttributes['autocomplete'];
    inputmode?: HTMLInputAttributes['inputmode'];
    placeholder?: string;
    errorId?: string;
    error?: string;
    oninput: (event: Event & { currentTarget: HTMLInputElement }) => void;
    onblur?: (event: FocusEvent) => void;
  } = $props();
</script>

<label class="grid min-w-0 gap-1 text-xs font-semibold">
  <span>{label}</span>
  <input
    class="box-border min-h-10 min-w-0 rounded-control border border-border bg-surface px-3 py-2 text-sm font-normal text-ink placeholder:text-ink outline-none transition-[background-color,border-color,box-shadow] duration-150 ease-in-out focus:border-border-strong focus:ring-2 focus:ring-focus/30 disabled:cursor-not-allowed disabled:opacity-45"
    {type}
    {value}
    {disabled}
    {autocomplete}
    {inputmode}
    {placeholder}
    aria-invalid={error ? 'true' : undefined}
    aria-describedby={error ? errorId : undefined}
    aria-label={label}
    {oninput}
    {onblur}
  />
  {#if error}
    <span id={errorId} class="text-xs font-normal text-danger" role="alert">{error}</span>
  {/if}
</label>
