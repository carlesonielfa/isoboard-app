<script lang="ts">
  import { Combobox as ComboboxPrimitive } from 'bits-ui';

  let { ref = $bindable(null), ...restProps }: ComboboxPrimitive.InputProps = $props();
</script>

<ComboboxPrimitive.Input bind:ref {...restProps} />
