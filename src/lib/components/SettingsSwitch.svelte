<script lang="ts">
  import { Switch } from '$lib/components/ui/switch';

  let {
    label,
    checked,
    disabled = false,
    onCheckedChange,
  }: {
    label: string;
    checked: boolean;
    disabled?: boolean;
    onCheckedChange: (checked: boolean) => void;
  } = $props();

  const id = $props.id();
</script>

<div
  class="group flex items-center gap-2 text-sm has-disabled:cursor-not-allowed has-disabled:opacity-45"
>
  <Switch {id} {checked} {disabled} {onCheckedChange} />
  <label for={id} class="cursor-pointer group-has-disabled:cursor-not-allowed">{label}</label>
</div>
