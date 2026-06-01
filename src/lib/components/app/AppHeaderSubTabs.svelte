<script lang="ts">
  import type { MeowConnectSection } from '$lib/types/app-shell';

  export let activeTab = 'dashboard';
  export let activeSettingsTab = 'roster';
  export let activeMeowConnectTab: MeowConnectSection = 'together';
  export let meowConnectFeatureEnabled = true;
  export let pendingMeowConnectRequests = 0;
</script>

{#if activeTab === 'settings'}
  <div class="settings-sub-tabs">
    <button
      class="settings-tab-button"
      class:active={activeSettingsTab === 'roster'}
      on:click={() => activeSettingsTab = 'roster'}
    >
      Roster
    </button>
    <button
      class="settings-tab-button"
      class:active={activeSettingsTab === 'todo'}
      on:click={() => activeSettingsTab = 'todo'}
    >
      Tracking
    </button>
    <button
      class="settings-tab-button"
      class:active={activeSettingsTab === 'raid'}
      on:click={() => activeSettingsTab = 'raid'}
    >
      Raids
    </button>
    <button
      class="settings-tab-button"
      class:active={activeSettingsTab === 'system'}
      on:click={() => activeSettingsTab = 'system'}
    >
      System
    </button>
  </div>
{/if}

{#if activeTab === 'meow-connect' && meowConnectFeatureEnabled}
  <div class="settings-sub-tabs">
    <button
      class="settings-tab-button"
      class:active={activeMeowConnectTab === 'together'}
      on:click={() => activeMeowConnectTab = 'together'}
    >
      Raid Together
    </button>
    <button
      class="settings-tab-button"
      class:active={activeMeowConnectTab === 'logs'}
      on:click={() => activeMeowConnectTab = 'logs'}
    >
      Logs
    </button>
    <button
      class="settings-tab-button"
      class:active={activeMeowConnectTab === 'settings'}
      on:click={() => activeMeowConnectTab = 'settings'}
    >
      Settings
      {#if pendingMeowConnectRequests > 0}
        <span class="tab-notification-badge">{pendingMeowConnectRequests}</span>
      {/if}
    </button>
  </div>
{/if}

<style>
  .settings-sub-tabs {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .settings-tab-button {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.5rem 1rem;
    background: var(--md-sys-color-surface-container);
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 0.5rem;
    cursor: pointer;
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--md-sys-color-on-surface-variant);
    transition: all 0.2s ease;
    white-space: nowrap;
  }

  .settings-tab-button:hover {
    background: var(--md-sys-color-surface-container-highest);
    color: var(--md-sys-color-on-surface);
    border-color: var(--md-sys-color-primary);
  }

  .settings-tab-button.active {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border-color: var(--md-sys-color-primary);
  }

  .tab-notification-badge {
    display: grid;
    place-items: center;
    min-width: 1.1rem;
    height: 1.1rem;
    padding: 0 0.22rem;
    border-radius: 999px;
    background: var(--md-sys-color-error);
    color: var(--md-sys-color-on-error);
    font-size: 0.66rem;
    font-weight: 800;
    line-height: 1;
  }

  .settings-tab-button.active .tab-notification-badge {
    background: var(--md-sys-color-on-primary);
    color: var(--md-sys-color-primary);
  }

  @media (max-width: 768px) {
    .settings-sub-tabs {
      gap: 0.25rem;
      flex-wrap: wrap;
    }

    .settings-tab-button {
      padding: 0.4rem 0.8rem;
      font-size: 0.7rem;
    }
  }
</style>
