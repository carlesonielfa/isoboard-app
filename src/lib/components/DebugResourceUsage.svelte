<script lang="ts">
  import { applicationConfig } from '$lib/application-config';
  import { onMount } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { commands } from '$lib/ipc';
  import { useLocalModels } from '$lib/local-models-controller.svelte';
  import { modelSize } from '$lib/model-size';

  const localModels = useLocalModels();
  let residentBytes = $state<number | null>(null);
  const loadedModels = $derived(
    (localModels?.controller.state.snapshot.models ?? []).filter(
      (model) => model.runtimeStatus === 'ready' || model.runtimeStatus === 'processing',
    ).length,
  );

  onMount(() => {
    if (!isTauri()) return;
    let disposed = false;
    let pending = false;
    async function refresh() {
      if (disposed || pending || document.hidden) return;
      pending = true;
      try {
        const usage = await commands.getAppMemoryUsage();
        if (!disposed) residentBytes = usage.residentBytes;
      } catch {
        if (!disposed) residentBytes = null;
      } finally {
        pending = false;
      }
    }
    const onVisibilityChange = () => void refresh();
    void refresh();
    const timer = setInterval(
      () => void refresh(),
      applicationConfig.feedback.resourcePollMilliseconds,
    );
    document.addEventListener('visibilitychange', onVisibilityChange);
    return () => {
      disposed = true;
      clearInterval(timer);
      document.removeEventListener('visibilitychange', onVisibilityChange);
    };
  });
</script>

<span
  class="whitespace-nowrap text-xs text-foreground tabular-nums"
  title="Resident RAM for isoboard's native process, including local inference. Excludes separate WebView helpers and is not a complete GPU-memory measurement. The count is loaded model bundles, not downloaded models."
>
  RAM {residentBytes === null ? 'unavailable' : modelSize(residentBytes)} · {loadedModels}
  {loadedModels === 1 ? 'model' : 'models'}
</span>
