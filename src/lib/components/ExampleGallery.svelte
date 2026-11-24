<script lang="ts" module>
  import type { Sparkles } from '@lucide/svelte';

  export type ExampleGalleryTone = 'violet' | 'blue' | 'rose';

  export type ExampleGalleryItem = {
    id: string;
    /** Shown in full and passed back through `onPick`. */
    text: string;
    icon: typeof Sparkles;
    tone: ExampleGalleryTone;
  };

  export type ExampleGalleryCategory<Item extends ExampleGalleryItem = ExampleGalleryItem> = {
    value: string;
    label: string;
    items: readonly Item[];
  };
</script>

<script lang="ts" generics="Item extends ExampleGalleryItem">
  import { Check, ChevronRight, Plus } from '@lucide/svelte';
  import SegmentedControl from '$lib/components/SegmentedControl.svelte';
  import { cn } from '$lib/utils';

  let {
    label,
    heading = 'Examples',
    categories,
    isPicked = () => false,
    onPick,
    disabled = false,
  }: {
    /** Accessible name of the gallery, such as "Example Additional Rules". */
    label: string;
    heading?: string;
    categories: readonly ExampleGalleryCategory<Item>[];
    isPicked?: (item: Item) => boolean;
    onPick: (item: Item) => void;
    disabled?: boolean;
  } = $props();

  const panelId = $props.id();
  let expanded = $state(false);
  let selectedCategory = $state('');

  let activeCategory = $derived(
    categories.find((category) => category.value === selectedCategory) ?? categories[0],
  );

  // Tailwind only generates classes it can find literally, so each tone is spelled out.
  const toneClasses: Record<ExampleGalleryTone, string> = {
    violet: 'text-tone-violet',
    blue: 'text-tone-blue',
    rose: 'text-tone-rose',
  };

  function pick(item: Item): void {
    if (disabled || isPicked(item)) return;
    onPick(item);
  }
</script>

<section class="grid min-w-0 gap-2" aria-label={label}>
  <button
    type="button"
    class="inline-flex w-fit items-center gap-1 rounded-sm text-xs font-semibold text-ink outline-none hover:text-ink/80 focus-visible:ring-2 focus-visible:ring-focus/30 disabled:cursor-not-allowed disabled:opacity-45"
    aria-expanded={expanded}
    aria-controls={panelId}
    {disabled}
    onclick={() => (expanded = !expanded)}
  >
    <ChevronRight
      class={cn(
        'size-3.5 shrink-0 transition-transform duration-150 motion-reduce:transition-none',
        expanded && 'rotate-90',
      )}
      strokeWidth={2}
      aria-hidden="true"
    />
    {heading}
  </button>
  {#if expanded}
    <div
      id={panelId}
      class="grid min-w-0 gap-2 rounded-control border border-border bg-surface-control p-2"
    >
      <SegmentedControl
        aria-label={`${label} categories`}
        options={categories}
        value={activeCategory?.value ?? ''}
        fullWidth
        {disabled}
        onChange={(value) => (selectedCategory = value)}
      />
      {#if activeCategory}
        <ul class="m-0 grid list-none gap-px p-0" aria-label={activeCategory.label}>
          {#each activeCategory.items as item (item.id)}
            {@const picked = isPicked(item)}
            <li class="min-w-0">
              <button
                type="button"
                class={cn(
                  'group flex w-full min-w-0 items-start gap-2.5 rounded-control px-2 py-1.5 text-left text-sm text-ink outline-none transition-colors duration-150 hover:bg-ink/5 focus-visible:ring-2 focus-visible:ring-focus/30 disabled:cursor-not-allowed disabled:opacity-45 motion-reduce:transition-none',
                  picked && 'cursor-default opacity-55 hover:bg-transparent',
                )}
                {disabled}
                aria-disabled={picked || undefined}
                onclick={() => pick(item)}
              >
                <item.icon
                  class={cn('mt-0.5 size-4 shrink-0', toneClasses[item.tone])}
                  strokeWidth={1.75}
                  aria-hidden="true"
                />
                <span class="min-w-0 flex-1 leading-snug">{item.text}</span>
                {#if picked}
                  <Check class="mt-0.5 size-4 shrink-0" strokeWidth={2.25} aria-hidden="true" />
                  <span class="sr-only">, added</span>
                {:else}
                  <Plus
                    class="mt-0.5 size-4 shrink-0 opacity-0 transition-opacity duration-150 group-hover:opacity-100 group-focus-visible:opacity-100 motion-reduce:transition-none"
                    strokeWidth={1.75}
                    aria-hidden="true"
                  />
                {/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</section>
