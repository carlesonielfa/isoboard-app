<script lang="ts">
  import { FileSliders, History, Settings, KeyRound, Keyboard } from '@lucide/svelte';
  import type { Component } from 'svelte';

  export type SettingsSectionIconName =
    'history' | 'shortcuts' | 'processing-profiles' | 'provider-access' | 'settings';

  const icons: Record<SettingsSectionIconName, Component> = {
    history: History,
    shortcuts: Keyboard,
    'processing-profiles': FileSliders,
    'provider-access': KeyRound,
    settings: Settings,
  };

  let { name, active = false }: { name: SettingsSectionIconName; active?: boolean } = $props();

  const Icon = $derived(icons[name]);
</script>

<Icon size={20} strokeWidth={active ? 2 : 1.5} class="size-5 shrink-0" aria-hidden="true" />
