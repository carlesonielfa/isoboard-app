<script lang="ts">
  let { value, label }: { value: number; label: string } = $props();
</script>

<progress
  aria-label={label}
  value={Math.max(0, Math.min(100, value))}
  max={100}
  class="mt-1 block h-1 w-full overflow-hidden rounded-full border-0 bg-surface-control [&::-webkit-progress-bar]:rounded-full [&::-webkit-progress-bar]:bg-surface-control [&::-webkit-progress-value]:rounded-full [&::-webkit-progress-value]:bg-ink [&::-moz-progress-bar]:bg-ink"
></progress>
