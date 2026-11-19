<script lang="ts">
  import PositionSelect from '$lib/components/PositionSelect.svelte';
  import type { OverlayPosition } from '$lib/generated/bindings';

  let {
    position,
    disabled = false,
    onPositionChange,
  }: {
    position: OverlayPosition;
    disabled?: boolean;
    onPositionChange: (position: OverlayPosition) => Promise<void>;
  } = $props();

  const positions: { value: OverlayPosition; label: string }[] = [
    { value: 'top_left', label: 'Top left' },
    { value: 'top_center', label: 'Top center' },
    { value: 'top_right', label: 'Top right' },
    { value: 'middle_left', label: 'Middle left' },
    { value: 'center', label: 'Center' },
    { value: 'middle_right', label: 'Middle right' },
    { value: 'bottom_left', label: 'Bottom left' },
    { value: 'bottom_center', label: 'Bottom center' },
    { value: 'bottom_right', label: 'Bottom right' },
  ];

  async function changePosition(value: string): Promise<void> {
    const position = positions.find((option) => option.value === value)?.value;
    if (!position || disabled) return;
    await onPositionChange(position);
  }
</script>

<div class="ml-4 flex flex-col gap-2">
  <h3 class="m-0 text-sm font-semibold">Capture overlay</h3>
  <div
    class="ml-4 mr-12 grid min-w-0 grid-cols-[minmax(12rem,max-content)_minmax(0,16rem)] items-center gap-2"
  >
    <span class="min-w-0 px-2 text-sm">Position</span>
    <PositionSelect
      label="Capture overlay position"
      value={position}
      options={positions}
      {disabled}
      onValueChange={changePosition}
    />
  </div>
</div>
