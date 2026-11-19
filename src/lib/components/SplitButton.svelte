<script lang="ts">
  import type { ComponentProps, Snippet } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import DropdownMenuButton from '$lib/components/DropdownMenuButton.svelte';
  import { cn } from '$lib/utils.js';

  let {
    children,
    label,
    title = label,
    menuLabel = 'More options',
    menu,
    onclick,
    disabled = false,
    actionDisabled = false,
    size = 'default',
    divided = false,
    class: className = '',
  }: {
    children: Snippet;
    label: string;
    title?: string;
    menuLabel?: string;
    menu: Snippet;
    onclick: () => void;
    disabled?: boolean;
    actionDisabled?: boolean;
    size?: ComponentProps<typeof Button>['size'];
    divided?: boolean;
    class?: string;
  } = $props();
</script>

<div class={cn('inline-flex shrink-0 items-stretch rounded-control', className)}>
  <Button
    variant="ghost"
    {size}
    class={cn(
      'rounded-r-none',
      size === 'toolbar' && 'min-w-0 flex-1 shrink overflow-hidden',
      (size === 'toolbar-icon' || size === 'toolbar') &&
        'rounded-l-[calc(var(--radius-control)-0.1875rem)]',
    )}
    type="button"
    aria-label={label}
    {title}
    disabled={disabled || actionDisabled}
    {onclick}
  >
    {@render children()}
  </Button>
  <DropdownMenuButton
    label={menuLabel}
    {disabled}
    {menu}
    class={cn(
      'h-auto rounded-l-none',
      (size === 'toolbar-icon' || size === 'toolbar') &&
        'rounded-r-[calc(var(--radius-control)-0.1875rem)]',
      divided && 'border-l-border',
    )}
  />
</div>
