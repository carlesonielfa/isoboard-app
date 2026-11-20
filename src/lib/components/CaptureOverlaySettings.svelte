<script lang="ts">
  import { PictureInPicture2 } from '@lucide/svelte';
  import PositionSelect from '$lib/components/PositionSelect.svelte';
  import SettingsGroup from '$lib/components/SettingsGroup.svelte';
  import SettingsRow from '$lib/components/SettingsRow.svelte';
  import SettingsSwitch from '$lib/components/SettingsSwitch.svelte';
  import type { OverlayPosition } from '$lib/generated/bindings';

  let {
    enabled,
    enabledDisabled = false,
    onEnabledChange,
    position,
    disabled = false,
    onPositionChange,
  }: {
    enabled: boolean;
    enabledDisabled?: boolean;
    onEnabledChange: (enabled: boolean) => void;
    position: OverlayPosition;
    disabled?: boolean;
    onPositionChange: (position: OverlayPosition) => Promise<void>;
  } = $props();

  const positionDisabled = $derived(disabled || !enabled);

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
    if (!position || positionDisabled) return;
    await onPositionChange(position);
  }
</script>

<SettingsGroup title="Capture status" icon={PictureInPicture2}>
  <SettingsSwitch
    variant="row"
    label="Show capture status"
    description="Shows progress and results after you capture."
    checked={enabled}
    disabled={enabledDisabled}
    onCheckedChange={onEnabledChange}
  />
  <SettingsRow label="Position" disabled={positionDisabled}>
    {#snippet control()}
      <div class="w-56">
        <PositionSelect
          label="Capture status position"
          value={position}
          options={positions}
          disabled={positionDisabled}
          onValueChange={changePosition}
        />
      </div>
    {/snippet}
  </SettingsRow>
</SettingsGroup>
