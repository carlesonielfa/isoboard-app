<script lang="ts">
  import { ChevronDown } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { buttonVariants } from '$lib/components/ui/button';
  import { cn } from '$lib/utils.js';

  let {
    label = 'More options',
    menu,
    disabled = false,
    class: className = '',
  }: {
    label?: string;
    menu: Snippet;
    disabled?: boolean;
    class?: string;
  } = $props();
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger
    class={cn(buttonVariants({ variant: 'ghost', size: 'icon-sm' }), 'w-6', className)}
    aria-label={label}
    title={label}
    {disabled}
  >
    <ChevronDown class="size-3 shrink-0" aria-hidden="true" />
  </DropdownMenu.Trigger>
  <DropdownMenu.Content align="end">
    {@render menu()}
  </DropdownMenu.Content>
</DropdownMenu.Root>
