<script lang="ts">
  import RosterSettings from './RosterSettings.svelte';
  import TrackingSettings from './TrackingSettings.svelte';
  import RaidSettings from './RaidSettings.svelte';
  import SystemSettings from './SystemSettings.svelte';
  import { createEventDispatcher } from 'svelte';

  // Props from parent
  export let activeSettingsTab: string = 'roster';

  // Event dispatcher for parent communication
  const dispatch = createEventDispatcher();

  // Local state for tab content
  let activeTab = activeSettingsTab;
  
  // Update local state when parent changes
  $: if (activeSettingsTab !== activeTab) {
    activeTab = activeSettingsTab;
  }

  function switchSettingsTab(tab: string) {
    activeTab = tab;
    // Inform parent about the change
    dispatch('tabChange', tab);
  }
</script>

<div class="settings-container">
  <!-- Tab Content (no sub-tabs needed anymore - they're in the header) -->
  <div class="tab-content-area">
    {#if activeTab === 'roster'}
      <RosterSettings />
    {/if}

    {#if activeTab === 'todo'}
      <TrackingSettings />
    {/if}

    {#if activeTab === 'raid'}
      <RaidSettings />
    {/if}

    {#if activeTab === 'system'}
      <SystemSettings />
    {/if}
  </div>
</div>

<style>
  .settings-container {
    --app-control-accent: var(--app-settings-accent);
    --app-control-on-accent: var(--md-sys-color-on-primary);
    --app-control-accent-container: var(--app-settings-accent-soft);
    --app-control-hover-border: var(--app-settings-accent);
    --app-matrix-ilvl-color: var(--app-settings-matrix-ilvl-color);
    --app-matrix-cp-color: var(--app-settings-matrix-cp-color);
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .tab-content-area {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: var(--md-sys-color-surface);
    padding: 1rem;
  }
</style>
