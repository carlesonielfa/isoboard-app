<script lang="ts">
  import Badge from '$lib/components/Badge.svelte';
  import Disclosure from '$lib/components/Disclosure.svelte';
  import type { ChangelogEntry } from '$lib/generated/bindings';

  let { entries }: { entries: ChangelogEntry[] } = $props();
</script>

{#if entries.length > 0}
  <Disclosure label="What's new">
    <div class="grid max-h-80 gap-4 overflow-y-auto pr-2">
      {#each entries as entry (entry.version)}
        <section class="grid gap-2" aria-labelledby={`changelog-${entry.version}`}>
          <div class="flex flex-wrap items-center gap-2">
            <h3 id={`changelog-${entry.version}`} class="m-0 text-sm font-semibold text-ink">
              {entry.version}{entry.label ? ` — ${entry.label}` : ''}
            </h3>
            {#if entry.latest}<Badge tone="highlight">Latest</Badge>{/if}
            {#if entry.current}<Badge>Current</Badge>{/if}
          </div>
          <ul class="m-0 grid list-disc gap-1 pl-5 text-sm text-ink">
            {#each entry.items as item, index (index)}
              <li>
                {#each item as span, spanIndex (spanIndex)}
                  {#if span.style === 'code'}
                    <code class="rounded-sm bg-surface-control px-1 font-mono text-xs"
                      >{span.text}</code
                    >
                  {:else if span.style === 'strong'}
                    <strong>{span.text}</strong>
                  {:else}
                    {span.text}
                  {/if}
                {/each}
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    </div>
  </Disclosure>
{/if}
