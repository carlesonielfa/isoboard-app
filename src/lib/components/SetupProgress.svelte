<script lang="ts">
  import type { SetupPath, SetupStep } from '$lib/setup-state';

  let {
    step,
    path = 'local',
    showModelStep = false,
    complete = [],
  }: {
    step: SetupStep;
    path?: SetupPath;
    showModelStep?: boolean;
    complete?: SetupStep[];
  } = $props();

  const steps: { id: SetupStep; label: string }[] = $derived([
    { id: 'permissions', label: 'Permissions' },
    { id: path === 'local' ? 'local' : 'provider', label: 'Set up AI Model' },
    ...(showModelStep ? [{ id: 'profile' as const, label: 'Model' }] : []),
    { id: 'capture', label: 'Start capturing' },
  ]);
</script>

<ol
  class={`m-0 grid list-none gap-1 p-0 ${steps.length === 3 ? 'grid-cols-3' : 'grid-cols-4'}`}
  aria-label="Setup progress"
>
  {#each steps as item, index (item.id)}
    <li class="min-w-0">
      <div
        class={`flex min-h-9 items-center gap-2 rounded-control px-2 py-1.5 text-left ${item.id === step ? 'bg-ink text-canvas' : 'text-ink'}`}
        aria-current={item.id === step ? 'step' : undefined}
      >
        <span
          class={`mt-0.5 flex size-5 shrink-0 items-center justify-center rounded-full border text-[0.65rem] font-semibold ${item.id === step ? 'border-canvas/60' : complete.includes(item.id) ? 'border-success bg-success text-canvas' : 'border-border-strong'}`}
          aria-hidden="true">{complete.includes(item.id) ? '✓' : index + 1}</span
        >
        <span class="min-w-0">
          <span class="block truncate text-xs font-semibold">{item.label}</span>
        </span>
      </div>
    </li>
  {/each}
</ol>
