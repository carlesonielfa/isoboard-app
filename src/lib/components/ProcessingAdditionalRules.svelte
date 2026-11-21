<script lang="ts">
  import SettingsSwitch from '$lib/components/SettingsSwitch.svelte';
  import { docsLinks } from '$lib/docs-links';

  let {
    value,
    separatePass,
    disabled = false,
    automaticSeparatePass = false,
    onInput,
    onSeparatePassChange,
    onBlur,
  }: {
    value: string;
    separatePass: boolean;
    disabled?: boolean;
    automaticSeparatePass?: boolean;
    onInput: (value: string) => void;
    onSeparatePassChange: (value: boolean) => void;
    onBlur?: (event: FocusEvent) => void;
  } = $props();
</script>

<div class="grid min-w-0 gap-2">
  <label class="grid min-w-0 gap-1 text-xs font-semibold">
    <span>Additional Rules</span>
    <textarea
      class="box-border min-h-24 min-w-0 resize-y rounded-control border border-border bg-surface px-3 py-2 text-sm font-normal text-ink outline-none transition-[border-color,box-shadow] duration-150 placeholder:text-ink/60 placeholder:italic focus:border-border-strong focus:ring-2 focus:ring-focus/30 disabled:cursor-not-allowed disabled:opacity-45"
      rows="3"
      placeholder="For example: translate prose to Spanish; preserve code and URLs."
      {value}
      {disabled}
      aria-label="Additional Rules"
      oninput={(event) => onInput(event.currentTarget.value)}
      onblur={onBlur}></textarea>
  </label>
  <SettingsSwitch
    label="Apply rules in a separate pass"
    checked={automaticSeparatePass || separatePass}
    disabled={disabled || automaticSeparatePass}
    onCheckedChange={onSeparatePassChange}
    infoLink={docsLinks.rulesSeparatePass}
  />
  {#if automaticSeparatePass}
    <p class="m-0 text-xs">
      Required: this model extracts content but cannot apply custom rules. Rules need a separate
      pass with a text-capable model.
    </p>
  {/if}
  {#if separatePass && value.trim()}
    <p class="m-0 text-xs">
      A separate pass makes another model request and may take longer or cost more.
    </p>
  {/if}
</div>
