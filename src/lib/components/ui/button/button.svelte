<script lang="ts" module>
  /* eslint-disable svelte/no-navigation-without-resolve */
  import { type VariantProps, tv } from 'tailwind-variants';
  import { cn, type WithElementRef } from '$lib/utils.js';
  import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';

  export const buttonVariants = tv({
    base: "focus-visible:border-focus focus-visible:ring-focus/30 aria-invalid:ring-danger/30 aria-invalid:border-danger-border rounded-control border border-transparent bg-clip-padding text-sm font-semibold focus-visible:ring-2 active:not-aria-[haspopup]:translate-y-px [&_svg:not([class*='size-'])]:size-4 group/button inline-flex shrink-0 items-center justify-center whitespace-nowrap transition-[background-color,border-color,color,opacity] duration-150 ease-in-out outline-none select-none disabled:cursor-not-allowed disabled:opacity-45 [&_svg]:pointer-events-none [&_svg]:shrink-0",
    variants: {
      variant: {
        default:
          'border-ink bg-ink text-canvas hover:border-ink/84 hover:bg-ink/84 active:border-ink/68 active:bg-ink/68',
        outline:
          'border-border bg-transparent text-ink hover:border-border-strong hover:bg-ink/5 active:bg-ink/10',
        secondary:
          'border-border bg-surface-control text-ink hover:bg-surface-raised active:bg-surface-raised',
        ghost: 'border-transparent bg-transparent text-ink hover:bg-ink/7 active:bg-ink/10',
        destructive:
          'border-danger-border bg-danger-surface text-ink hover:bg-danger-surface/80 active:bg-danger-surface/70',
        link: 'text-ink underline-offset-4 hover:underline',
      },
      size: {
        default: 'min-h-11 gap-2 px-4 py-2.5',
        xs: "min-h-8 gap-1 rounded-control px-2 text-xs [&_svg:not([class*='size-'])]:size-3",
        sm: "min-h-9 gap-1.5 rounded-control px-2.5 text-sm [&_svg:not([class*='size-'])]:size-3.5",
        lg: 'min-h-12 gap-2 rounded-control px-4',
        toolbar: 'h-toolbar-button min-w-0 max-w-full gap-2 px-2 py-1 text-xs',
        'toolbar-icon': 'h-toolbar-button w-9 p-0',
        icon: 'size-10 p-0',
        'icon-xs': "size-6 rounded-control [&_svg:not([class*='size-'])]:size-3",
        'icon-sm': 'size-8 rounded-control',
        'icon-lg': 'size-11',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  });

  export type ButtonVariant = VariantProps<typeof buttonVariants>['variant'];
  export type ButtonSize = VariantProps<typeof buttonVariants>['size'];

  export type ButtonProps = WithElementRef<HTMLButtonAttributes> &
    WithElementRef<HTMLAnchorAttributes> & {
      variant?: ButtonVariant;
      size?: ButtonSize;
    };
</script>

<script lang="ts">
  import { resolve } from '$app/paths';

  const resolvePath = resolve as unknown as (path: string) => string;

  function resolveHref(path: string): string {
    return resolvePath(path);
  }

  let {
    class: className,
    variant = 'default',
    size = 'default',
    ref = $bindable(null),
    href = undefined,
    type = 'button',
    disabled,
    children,
    ...restProps
  }: ButtonProps = $props();
</script>

{#if href}
  <a
    bind:this={ref}
    data-slot="button"
    class={cn(buttonVariants({ variant, size }), className)}
    href={disabled ? undefined : resolveHref(href)}
    aria-disabled={disabled}
    role={disabled ? 'link' : undefined}
    tabindex={disabled ? -1 : undefined}
    {...restProps}
  >
    {@render children?.()}
  </a>
{:else}
  <button
    bind:this={ref}
    data-slot="button"
    class={cn(buttonVariants({ variant, size }), className)}
    {type}
    {disabled}
    {...restProps}
  >
    {@render children?.()}
  </button>
{/if}
