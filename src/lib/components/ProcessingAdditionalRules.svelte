<script lang="ts">
  import ExampleGallery from '$lib/components/ExampleGallery.svelte';
  import SettingsSwitch from '$lib/components/SettingsSwitch.svelte';
  import {
    additionalRuleExampleCategories,
    appendAdditionalRule,
    hasAdditionalRule,
    type AdditionalRuleExample,
  } from '$lib/additional-rule-examples';
  import { docsLinks } from '$lib/docs-links';

  let {
    value,
    separatePass,
    disabled = false,
    automaticSeparatePass = false,
    onInput,
    onSeparatePassChange,
    onBlur,
    onCommit,
  }: {
    value: string;
    separatePass: boolean;
    disabled?: boolean;
    automaticSeparatePass?: boolean;
    onInput: (value: string) => void;
    onSeparatePassChange: (value: boolean) => void;
    onBlur?: (event: FocusEvent) => void;
    /** Saves the current value after a change made outside the textarea, such as an example. */
    onCommit?: () => void;
  } = $props();

  const textareaId = $props.id();

  function addExample(example: AdditionalRuleExample): void {
    const next = appendAdditionalRule(value, example.text);
    if (next === value) return;
    onInput(next);
    onCommit?.();
  }
</script>

<div class="grid min-w-0 gap-2">
  <div class="grid min-w-0 gap-1">
    <label for={textareaId} class="text-xs font-semibold">Additional Rules</label>
    <textarea
      id={textareaId}
      class="box-border min-h-24 min-w-0 resize-y rounded-control border border-border bg-surface px-3 py-2 text-sm font-normal text-ink outline-none transition-[border-color,box-shadow] duration-150 placeholder:text-ink/60 placeholder:italic focus:border-border-strong focus:ring-2 focus:ring-focus/30 disabled:cursor-not-allowed disabled:opacity-45"
      rows="3"
      placeholder="For example: translate prose to Spanish; preserve code and URLs."
      {value}
      {disabled}
      oninput={(event) => onInput(event.currentTarget.value)}
      onblur={onBlur}></textarea>
  </div>
  <ExampleGallery
    label="Example Additional Rules"
    categories={additionalRuleExampleCategories}
    isPicked={(example) => hasAdditionalRule(value, example.text)}
    onPick={addExample}
    {disabled}
  />

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
