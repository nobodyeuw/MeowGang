<script lang="ts">
  import RosterSettings from './settings/RosterSettings.svelte';
  import TrackingSettings from './settings/TrackingSettings.svelte';
  import RaidSettings from './settings/RaidSettings.svelte';
  import SystemSettings from './settings/SystemSettings.svelte';
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
      <div class="content-panel">
        <div class="panel-content">
          <RosterSettings />
        </div>
      </div>
    {/if}

    {#if activeTab === 'todo'}
      <div class="content-panel">
        <div class="panel-content">
          <TrackingSettings />
        </div>
      </div>
    {/if}

    {#if activeTab === 'raid'}
      <div class="content-panel">
        <div class="panel-content">
          <RaidSettings />
        </div>
      </div>
    {/if}

    {#if activeTab === 'system'}
      <div class="content-panel">
        <div class="panel-content">
          <SystemSettings />
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings-container {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .tab-content-area {
    flex: 1;
    overflow-y: auto;
    background: var(--md-sys-color-surface);
  }

  .content-panel {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .panel-content {
    flex: 1;
    min-height: 0;
    padding: 1rem;
  }

  @media (max-width: 768px) {
    .panel-content {
      padding: 0.75rem;
    }
  }
</style>
