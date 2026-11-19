<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    label,
    placeholder,
    value,
    type = 'text',
    disabled = false,
    describedBy,
    invalid = false,
    onblur,
    oninput,
    onsubmit,
    trailing,
    variant = 'default',
  }: {
    label: string;
    placeholder: string;
    value: string;
    type?: 'text' | 'password';
    disabled?: boolean;
    describedBy?: string;
    invalid?: boolean;
    onblur?: () => void;
    oninput: (value: string) => void;
    onsubmit: () => void;
    trailing: Snippet;
    /**
     * `flush` is a compact, square field that spans its container with only a
     * bottom border, for a toolbar row at the edge of a panel.
     */
    variant?: 'default' | 'flush';
  } = $props();

  const variantClasses = {
    default:
      'min-h-11 rounded-control border border-border p-1 focus-within:border-border-strong focus-within:ring-2 focus-within:ring-focus/30',
    flush: 'min-h-9 w-full border-b border-border px-1 focus-within:border-focus',
  } as const;
  const trailingClasses = {
    default: 'size-8 justify-center',
    flush: 'h-8 gap-0.5',
  } as const;
</script>

<form
  class="flex min-w-0 items-center gap-1 bg-surface text-ink transition-[border-color,box-shadow] {variantClasses[
    variant
  ]}"
  aria-label={label}
  onsubmit={(event) => {
    event.preventDefault();
    if (!disabled) onsubmit();
  }}
>
  <input
    class="min-w-0 flex-1 bg-transparent px-2 py-1 text-sm text-ink placeholder:text-ink outline-none disabled:cursor-not-allowed disabled:opacity-45"
    aria-label={label}
    aria-describedby={describedBy}
    aria-invalid={invalid || undefined}
    {onblur}
    {placeholder}
    {type}
    {value}
    {disabled}
    autocomplete="off"
    spellcheck={false}
    oninput={(event) => oninput(event.currentTarget.value)}
  />
  <div class="flex shrink-0 items-center {trailingClasses[variant]}">{@render trailing()}</div>
</form>
