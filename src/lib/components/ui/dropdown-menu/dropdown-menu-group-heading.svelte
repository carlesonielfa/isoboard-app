<script lang="ts">
  import { DropdownMenu as DropdownMenuPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils';
  let { class: className, ...props }: DropdownMenuPrimitive.GroupHeadingProps = $props();
</script>

<DropdownMenuPrimitive.GroupHeading
  class={cn('px-3 pt-2 pb-1.5 text-xs font-medium text-ink', className)}
  {...props}
/>
