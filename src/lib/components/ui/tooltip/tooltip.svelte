<script lang="ts">
  import { Tooltip as TooltipPrimitive } from 'bits-ui';

  let {
    open = $bindable(false),
    delayDuration = 300,
    ...restProps
  }: TooltipPrimitive.RootProps & { delayDuration?: number } = $props();
</script>

<!-- Each root brings its own provider so callers do not need an app-level one. -->
<TooltipPrimitive.Provider {delayDuration}>
  <TooltipPrimitive.Root bind:open {...restProps} />
</TooltipPrimitive.Provider>
