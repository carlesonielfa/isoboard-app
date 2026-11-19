<script lang="ts">
  import { ChevronLeft, ChevronRight } from '@lucide/svelte';
  import * as RangeCalendar from '$lib/components/ui/range-calendar/index.js';
  import { parseDate, today, getLocalTimeZone, type DateValue } from '@internationalized/date';

  /** A day range as `YYYY-MM-DD` strings; `start` alone is a range in progress. */
  export type DayRange = { start: string | null; end: string | null };

  let {
    label,
    value,
    disabled = false,
    allowFuture = false,
    onValueChange,
  }: {
    label: string;
    value: DayRange;
    disabled?: boolean;
    /** Days after today are unavailable unless this is set. */
    allowFuture?: boolean;
    /**
     * Called once both ends are chosen. Pressing the first day again makes a
     * one-day range; clearing is left to the caller.
     */
    onValueChange: (value: DayRange) => void;
  } = $props();

  const toDate = (day: string | null) => (day ? parseDate(day) : undefined);
  // The primitive owns the in-progress selection; `value` re-syncs it.
  let selection = $derived({ start: toDate(value.start), end: toDate(value.end) });
  const maxValue = $derived(allowFuture ? undefined : today(getLocalTimeZone()));

  function handleChange(next: { start: DateValue | undefined; end: DateValue | undefined }) {
    if (next.start && next.end)
      onValueChange({ start: next.start.toString(), end: next.end.toString() });
  }
</script>

<RangeCalendar.Root
  bind:value={selection}
  onValueChange={handleChange}
  {maxValue}
  {disabled}
  preventDeselect
  weekdayFormat="short"
  aria-label={label}
  class="w-full text-ink"
>
  {#snippet children({ months, weekdays })}
    <RangeCalendar.Header class="flex items-center justify-between">
      <RangeCalendar.PrevButton
        class="flex size-8 items-center justify-center rounded-control outline-none hover:bg-surface-raised focus-visible:ring-2 focus-visible:ring-focus/50 disabled:opacity-45"
      >
        <ChevronLeft class="size-4" strokeWidth={2} />
      </RangeCalendar.PrevButton>
      <RangeCalendar.Heading class="text-sm font-semibold" />
      <RangeCalendar.NextButton
        class="flex size-8 items-center justify-center rounded-control outline-none hover:bg-surface-raised focus-visible:ring-2 focus-visible:ring-focus/50 disabled:opacity-45"
      >
        <ChevronRight class="size-4" strokeWidth={2} />
      </RangeCalendar.NextButton>
    </RangeCalendar.Header>
    {#each months as month (month.value.toString())}
      <RangeCalendar.Grid class="w-full border-collapse select-none">
        <RangeCalendar.GridHead>
          <RangeCalendar.GridRow class="grid grid-cols-7">
            {#each weekdays as day (day)}
              <RangeCalendar.HeadCell class="text-center text-xs font-normal">
                {day.slice(0, 2)}
              </RangeCalendar.HeadCell>
            {/each}
          </RangeCalendar.GridRow>
        </RangeCalendar.GridHead>
        <RangeCalendar.GridBody>
          {#each month.weeks as week, index (index)}
            <RangeCalendar.GridRow class="grid grid-cols-7">
              {#each week as date (date.toString())}
                <RangeCalendar.Cell {date} month={month.value} class="h-7 p-0">
                  <RangeCalendar.Day
                    class="flex h-7 w-full items-center justify-center rounded-control text-xs outline-none hover:bg-surface-raised focus-visible:ring-2 focus-visible:ring-focus/50 data-[today]:font-semibold data-[outside-month]:invisible data-[unavailable]:line-through data-[disabled]:opacity-45 data-[unavailable]:opacity-45 data-[range-middle]:rounded-none data-[selected]:bg-highlight-surface data-[selection-start]:bg-highlight data-[selection-start]:text-canvas data-[selection-end]:bg-highlight data-[selection-end]:text-canvas data-[highlighted]:bg-highlight-surface"
                  />
                </RangeCalendar.Cell>
              {/each}
            </RangeCalendar.GridRow>
          {/each}
        </RangeCalendar.GridBody>
      </RangeCalendar.Grid>
    {/each}
  {/snippet}
</RangeCalendar.Root>
