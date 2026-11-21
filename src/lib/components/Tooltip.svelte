<script lang="ts">
  import type { ComponentProps, Snippet } from 'svelte';
  import * as TooltipPrimitive from '$lib/components/ui/tooltip';
  import { cn } from '$lib/utils';

  let {
    content,
    disabled = false,
    focusable = false,
    side = 'top',
    class: className,
    children,
    trigger,
  }: {
    /** Tooltip text, or a snippet for richer content. */
    content: string | Snippet;
    /** Renders the children alone, without a tooltip. */
    disabled?: boolean;
    /** Adds the wrapper to the tab order, for triggers that cannot take focus themselves. */
    focusable?: boolean;
    side?: ComponentProps<typeof TooltipPrimitive.Content>['side'];
    class?: string;
    children?: Snippet;
    /** Renders a focusable control as the trigger itself, so keyboard focus opens the tooltip.
     *  Spread the props onto the control and call their `onclick` from its own handler. */
    trigger?: Snippet<[Record<string, unknown>]>;
  } = $props();
</script>

{#if disabled}
  {#if trigger}{@render trigger({})}{:else}{@render children?.()}{/if}
{:else}
  <TooltipPrimitive.Root>
    <TooltipPrimitive.Trigger>
      {#snippet child({ props })}
        {#if trigger}
          {@render trigger(props)}
        {:else}
          <!-- Disabled controls do not receive pointer events, so the wrapper
             owns hover and focus and the controls inside pass pointers through.
             A focusable wrapper exposes the tooltip to keyboard users. -->
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <span
            {...props}
            tabindex={focusable ? 0 : undefined}
            class={cn(
              'inline-flex rounded-control outline-none focus-visible:ring-2 focus-visible:ring-focus/50 [&_:disabled]:pointer-events-none',
              className,
            )}
          >
            {@render children?.()}
          </span>
        {/if}
      {/snippet}
    </TooltipPrimitive.Trigger>
    <TooltipPrimitive.Content {side}>
      {#if typeof content === 'string'}{content}{:else}{@render content()}{/if}
    </TooltipPrimitive.Content>
  </TooltipPrimitive.Root>
{/if}
