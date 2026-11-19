<script lang="ts">
  import { SlidersHorizontal, X } from '@lucide/svelte';
  import { parseDate } from '@internationalized/date';
  import ActionInput from '$lib/components/ActionInput.svelte';
  import Button from '$lib/components/Button.svelte';
  import DateRangeCalendar from '$lib/components/DateRangeCalendar.svelte';
  import SegmentedControl from '$lib/components/SegmentedControl.svelte';
  import SettingsSelect from '$lib/components/SettingsSelect.svelte';
  import ToggleChipGroup from '$lib/components/ToggleChipGroup.svelte';
  import * as Popover from '$lib/components/ui/popover/index.js';
  import type { ContentTag, HistoryFacets } from '$lib/generated/bindings';
  import {
    CONTENT_TAG_LABELS,
    EMPTY_HISTORY_FILTER,
    activeFilterCount,
    isFiltered,
    type HistoryViewFilter,
  } from '$lib/history-search';

  let {
    filter,
    facets,
    disabled,
    onChange,
  }: {
    filter: HistoryViewFilter;
    facets: HistoryFacets;
    disabled: boolean;
    onChange: (change: Partial<HistoryViewFilter>, immediate?: boolean) => void;
  } = $props();

  // Select values are strings; this one stands for "no filter".
  const ALL_PROFILES = 'all';
  const activeCount = $derived(activeFilterCount(filter));
  // Name only what the button will clear.
  const clearAllLabel = $derived(
    activeCount === 0
      ? 'Clear search'
      : filter.query.trim()
        ? 'Clear search and filters'
        : 'Clear filters',
  );
  const profileOptions = $derived([
    { value: ALL_PROFILES, label: 'All profiles' },
    ...facets.profiles.map((profile) => ({ value: profile.id, label: profile.name })),
    // Keep a chosen profile selectable after its last entry is deleted.
    ...(filter.profileId && !facets.profiles.some((profile) => profile.id === filter.profileId)
      ? [{ value: filter.profileId, label: filter.profileId }]
      : []),
  ]);
  const outcomeOptions = [
    { value: 'all', label: 'All' },
    { value: 'success', label: 'Results' },
    { value: 'failure', label: 'Failed' },
  ];
  const contentOptions = Object.entries(CONTENT_TAG_LABELS).map(([value, label]) => ({
    value,
    label,
  }));
  const dayFormat = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' });
  const rangeLabel = $derived.by(() => {
    if (!filter.from || !filter.to) return 'Any time';
    const day = (value: string) => dayFormat.format(parseDate(value).toDate('UTC'));
    return filter.from === filter.to ? day(filter.from) : `${day(filter.from)} – ${day(filter.to)}`;
  });

  function clearFilters(): void {
    const { outcome, profileId, from, to, contains } = EMPTY_HISTORY_FILTER;
    onChange({ outcome, profileId, from, to, contains });
  }
</script>

<ActionInput
  variant="flush"
  label="Search history"
  placeholder="Search"
  value={filter.query}
  {disabled}
  oninput={(query) => onChange({ query })}
  onsubmit={() => onChange({ query: filter.query }, true)}
>
  {#snippet trailing()}
    {#if isFiltered(filter)}
      <Button
        variant="ghost"
        size="icon-sm"
        type="button"
        aria-label={clearAllLabel}
        title={clearAllLabel}
        {disabled}
        onclick={() => onChange({ ...EMPTY_HISTORY_FILTER }, true)}
      >
        <X class="size-4 shrink-0" strokeWidth={2} />
      </Button>
    {/if}
    <Popover.Root>
      <Popover.Trigger {disabled}>
        {#snippet child({ props })}
          <Button
            {...props}
            variant="ghost"
            size="icon-sm"
            type="button"
            class={activeCount > 0 ? 'bg-highlight-surface' : ''}
            aria-label={activeCount > 0 ? `Filters, ${activeCount} active` : 'Filters'}
            title="Filters"
          >
            <SlidersHorizontal class="size-4 shrink-0" strokeWidth={2} />
          </Button>
        {/snippet}
      </Popover.Trigger>
      <Popover.Content
        class="flex max-h-(--bits-popover-content-available-height) w-72 flex-col gap-3 overflow-y-auto p-3"
        align="end"
        collisionPadding={8}
        aria-label="History filters"
      >
        <div class="grid gap-1 text-xs font-semibold">
          <span>Outcome</span>
          <SegmentedControl
            aria-label="Outcome"
            options={outcomeOptions}
            value={filter.outcome}
            {disabled}
            fullWidth
            class="text-xs"
            onChange={(value) => onChange({ outcome: value as HistoryViewFilter['outcome'] })}
          />
        </div>
        <SettingsSelect
          label="Profile"
          value={filter.profileId ?? ALL_PROFILES}
          options={profileOptions}
          {disabled}
          onValueChange={(value) => onChange({ profileId: value === ALL_PROFILES ? null : value })}
        />
        <div class="grid gap-1 text-xs">
          <span class="font-semibold">Contains</span>
          <ToggleChipGroup
            aria-label="Contains"
            fullWidth
            options={contentOptions}
            value={filter.contains}
            {disabled}
            onChange={(value) => onChange({ contains: value as ContentTag[] })}
          />
        </div>
        <div class="grid gap-1">
          <div class="flex items-center justify-between gap-2 text-xs">
            <span class="font-semibold">Date</span>
            <span>{rangeLabel}</span>
          </div>
          <DateRangeCalendar
            label="Date range"
            value={{ start: filter.from, end: filter.to }}
            {disabled}
            onValueChange={(range) => onChange({ from: range.start, to: range.end })}
          />
        </div>
        <Button
          variant="outline"
          size="sm"
          type="button"
          class="w-full"
          disabled={disabled || activeCount === 0}
          onclick={clearFilters}
        >
          Clear filters
        </Button>
      </Popover.Content>
    </Popover.Root>
  {/snippet}
</ActionInput>
