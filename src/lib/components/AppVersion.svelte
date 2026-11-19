<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { isTauri } from '@tauri-apps/api/core';
  import packageJson from '../../../package.json';

  let version = $state(packageJson.version);

  onMount(() => {
    if (!isTauri()) return;

    void getVersion()
      .then((runtimeVersion) => {
        if (runtimeVersion.trim()) version = runtimeVersion;
      })
      .catch(() => undefined);
  });
</script>

<span aria-label={`isoboard version ${version}`}>v{version}</span>
