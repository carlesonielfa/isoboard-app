<script lang="ts">
  import type { ComponentProps, Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import { Button as PrimitiveButton } from '$lib/components/ui/button/index.js';

  type ButtonVariant = 'primary' | 'outline' | 'ghost' | 'link';
  type ButtonSize = 'default' | 'sm' | 'toolbar' | 'toolbar-icon' | 'icon' | 'icon-xs' | 'icon-sm';

  type Props = HTMLButtonAttributes & {
    children: Snippet;
    size?: ButtonSize;
    variant?: ButtonVariant;
  };

  let {
    children,
    size = 'default',
    variant = 'outline',
    class: className = '',
    ...buttonProps
  }: Props = $props();

  const primitiveVariants: Record<ButtonVariant, 'default' | 'outline' | 'ghost' | 'link'> = {
    primary: 'default',
    outline: 'outline',
    ghost: 'ghost',
    link: 'link',
  };

  const primitiveSizes: Record<
    ButtonSize,
    'default' | 'sm' | 'toolbar' | 'toolbar-icon' | 'icon' | 'icon-xs' | 'icon-sm'
  > = {
    default: 'default',
    sm: 'sm',
    toolbar: 'toolbar',
    'toolbar-icon': 'toolbar-icon',
    icon: 'icon',
    'icon-xs': 'icon-xs',
    'icon-sm': 'icon-sm',
  };
</script>

<PrimitiveButton
  variant={primitiveVariants[variant]}
  size={primitiveSizes[size]}
  class={variant === 'link' ? `min-h-0 rounded-sm px-0 py-0 ${className}` : className}
  {...buttonProps as ComponentProps<typeof PrimitiveButton>}
>
  {@render children()}
</PrimitiveButton>
