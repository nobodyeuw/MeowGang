<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import Dashboard from '$lib/components/Dashboard.svelte';
  import Todo from '$lib/components/Todo.svelte';
  import Settings from '$lib/components/Settings.svelte';
  import UpdateTab from '$lib/components/UpdateTab.svelte';
  import EncounterSyncStatus from '$lib/components/EncounterSyncStatus.svelte';
  import ProgressionPlanner from '$lib/components/ProgressionPlanner.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import { initializeApp, activeFilterCharId, nextDailyReset, updateAvailable, latestAppVersion, currentAppVersion, isUpdateChecking, checkForAppUpdates } from '$lib/store';
  import { invoke } from '@tauri-apps/api/core';
  import { GAME_TASKS } from '$lib/data/tasks';
  import { RAIDS } from '$lib/data/raids';
  import { GAME_CLASSES } from '$lib/data/classes';

  import { testSyncRoster } from '$lib/store';
  import { initializeGoldLogging } from '$lib/init/gold-logging-init';
  import { listen } from '@tauri-apps/api/event';

  let activeTab = 'dashboard';
  let sidebarOpen = false;
  let headerContent = '';
  let activeSettingsTab = 'roster';
  let nextResetTime = '';
  let resetCountdown = '';

  // Handle URL parameters
  $: urlParams = new URLSearchParams($page.url.search);
  $: tabFromUrl = urlParams.get('tab') || 'dashboard';
  $: charFromUrl = urlParams.get('char');

  // Update active tab when URL changes
  $: if (tabFromUrl !== activeTab) {
    activeTab = tabFromUrl;
  }

  // Update active filter character when URL changes
  $: if (charFromUrl) {
    activeFilterCharId.set(parseInt(charFromUrl));
  }

  // Make test function available globally for debugging
  onMount(() => {
    (window as any).testSyncRoster = testSyncRoster;
  });

  // Initialize app on mount
  onMount(() => {
    const updateCountdownFromKnownReset = async () => {
      if (!nextResetTime) {
        await refreshNextResetTimeFromBackend();
      }
      updateResetCountdown();
    };

    (async () => {
      await initializeApp();
      checkForAppUpdates().catch((error) => console.warn('Update check failed:', error));
      
      // Initialize gold logging system
      initializeGoldLogging();
      
      // Update rested values immediately on app start
      try {
        console.log('?? Updating rested values on app start...');
        const restedResult = await invoke('update_rested_values_now');
        console.log('?? Rested values updated:', restedResult);
      } catch (restedError) {
        console.error('?? Failed to update rested values:', restedError);
      }
      
      // Set up gold event listeners (console only)
      await setupGoldEventListeners();
      
      // Check and ensure data completeness on app start
      try {
        console.log('?? Checking data completeness...');
        
        const completenessResult = await invoke('ensure_character_data_complete', { 
          data: { 
            tasks: GAME_TASKS, 
            raids: RAIDS
          } 
        });
        console.log('?? Data completeness check:', completenessResult);
        
        // Update reset timestamps
        const resetResult = await invoke('update_reset_timestamps');
        console.log('?? Reset timestamps updated:', resetResult);
        
        // Initialize reset countdown from bootstrap snapshot
        nextResetTime = $nextDailyReset;
        await updateCountdownFromKnownReset();
        
        // Debug: Trigger encounters sync on app start
        console.log('?? Triggering encounters sync on app start...');
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          const syncResult = await invoke('sync_encounters_to_completions');
          console.log('?? Encounters sync result:', syncResult);
        } catch (syncError) {
          console.error('?? Encounters sync failed:', syncError);
        }
        
      } catch (error) {
        console.error('?? Data completeness check failed:', error);
      }
    })();
    
    // Update countdown every second from cached reset timestamp.
    const countdownInterval = setInterval(updateResetCountdown, 1000);
    // Refresh backend reset timestamp only once per minute.
    const resetRefreshInterval = setInterval(refreshNextResetTimeFromBackend, 60000);
    
    // Cleanup on unmount
    return () => {
      clearInterval(countdownInterval);
      clearInterval(resetRefreshInterval);
    };
  });

  // Setup gold event listeners for console output only
  async function setupGoldEventListeners() {
    try {
      // Listen for gold processing events
      await listen('gold-logs-processed', (event) => {
        const data = event.payload as any;
        console.log(`?? Gold Processing: ${data.processed_count} entries processed (${data.trigger})`);
      });
      
      console.log('?? Gold event listeners setup complete');
    } catch (error) {
      console.error('?? Failed to setup gold event listeners:', error);
    }
  }

  function switchTab(tab: string) {
    console.log('Switching to tab:', tab);
    activeTab = tab;
    sidebarOpen = false;
    
    // Update URL using SvelteKit navigation
    const searchParams = new URLSearchParams();
    searchParams.set('tab', tab);
    
    // Remove char parameter when switching away from todo tab
    if (tab !== 'todo') {
      searchParams.delete('char');
    }
    
    goto(`/?${searchParams.toString()}`);
  }

  function setHeaderContent(content: string) {
    headerContent = content;
  }

  function toggleSidebar() {
    sidebarOpen = !sidebarOpen;
  }

  // Update reset countdown
  function updateResetCountdown() {
    if (!nextResetTime) {
      resetCountdown = 'Reset timer unavailable';
      return;
    }

    const now = new Date();
    const reset = new Date(nextResetTime);
    const diff = reset.getTime() - now.getTime();

    if (diff > 0) {
      const hours = Math.floor(diff / (1000 * 60 * 60));
      const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
      const seconds = Math.floor((diff % (1000 * 60)) / 1000);
      
      if (hours > 0) {
        resetCountdown = `Next daily reset in: ${hours}h ${minutes}m ${seconds}s`;
      } else if (minutes > 0) {
        resetCountdown = `Next daily reset in: ${minutes}m ${seconds}s`;
      } else {
        resetCountdown = `Next daily reset in: ${seconds}s`;
      }
    } else {
      resetCountdown = 'Daily reset should have occurred!';
    }
  }

  async function refreshNextResetTimeFromBackend() {
    try {
      const resetTime = await invoke('get_next_daily_reset_time');
      nextResetTime = resetTime as string;
      updateResetCountdown();
    } catch (error) {
      console.error('Failed to get next reset time:', error);
      resetCountdown = 'Reset timer unavailable';
    }
  }
</script>

<div class="app">
  <!-- Sidebar -->
  <Sidebar {activeTab} {switchTab} isOpen={sidebarOpen} />
  
  <!-- Overlay for mobile -->
  {#if sidebarOpen}
    <div class="overlay" role="button" tabindex="0" on:click={toggleSidebar} on:keydown={(e) => e.key === 'Enter' && toggleSidebar()} aria-label="Close sidebar"></div>
  {/if}

  <!-- Main Content -->
  <div class="main-content">
    <!-- Header -->
    <header class="header">
      <button class="menu-toggle" on:click={toggleSidebar} aria-label="Toggle menu">
        <span class="hamburger"></span>
      </button>
      
      <div class="header-title">
        <div class="title-row">
          <h1>LOA Tracker</h1>
          {#if resetCountdown}
            <div class="reset-countdown">{resetCountdown}</div>
          {/if}
        </div>
        {#if headerContent}
          <div class="header-info">{headerContent}</div>
        {/if}

        {#if $updateAvailable}
          <div class="app-alert update-banner">
            <div class="alert-copy">
              <strong>Update available:</strong> version {$latestAppVersion} is ready. Current version: {$currentAppVersion}.
            </div>
            <div class="banner-actions">
              <button class="banner-button" on:click={() => switchTab('updates')}>View updates</button>
              <button class="banner-button secondary" on:click={checkForAppUpdates} disabled={$isUpdateChecking}>
                {$isUpdateChecking ? 'Refreshing…' : 'Refresh'}
              </button>
            </div>
          </div>
        {/if}
      </div>
      
      <!-- Settings Sub-Tabs (only shown when settings tab is active) -->
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
    </header>

    <!-- Tab Content -->
    <main class="content">
      {#if activeTab === 'dashboard'}
        <Dashboard {setHeaderContent} />
      {:else if activeTab === 'todo'}
        <Todo highlightCharId={$activeFilterCharId} />
      {:else if activeTab === 'settings'}
        <Settings activeSettingsTab={activeSettingsTab} on:tabChange={(e) => activeSettingsTab = e.detail} />
      {:else if activeTab === 'progression'}
        <ProgressionPlanner />
      {:else if activeTab === 'updates'}
        <UpdateTab />
      {:else if activeTab === 'encounters'}
        <EncounterSyncStatus />
      {/if}
    </main>
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: var(--md-sys-color-background);
    color: var(--md-sys-color-on-background);
  }

  .app {
    display: flex;
    min-height: 100vh;
    position: relative;
  }

  .overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 101;
    backdrop-filter: blur(2px);
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1rem 1.5rem;
    background: var(--md-sys-color-surface);
    border-bottom: 1px solid var(--md-sys-color-outline);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    position: sticky;
    top: 0;
    z-index: 100;
  }

  .menu-toggle {
    background: none;
    border: none;
    padding: 0.5rem;
    cursor: pointer;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color 0.2s ease;
  }

  .menu-toggle:hover {
    background: var(--md-sys-color-surface-variant);
  }

  .hamburger {
    display: block;
    width: 20px;
    height: 2px;
    background: var(--md-sys-color-on-surface);
    position: relative;
    transition: all 0.3s ease;
  }

  .hamburger::before,
  .hamburger::after {
    content: '';
    position: absolute;
    width: 20px;
    height: 2px;
    background: var(--md-sys-color-on-surface);
    transition: all 0.3s ease;
  }

  .hamburger::before {
    top: -6px;
  }

  .hamburger::after {
    top: 6px;
  }

  .header-title {
    flex: 1;
  }

  .update-banner {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.75rem;
    align-items: center;
    padding: 0.9rem 1rem;
    margin-top: 0.75rem;
    border-radius: 16px;
    background: rgba(255, 210, 0, 0.12);
    border: 1px solid rgba(255, 191, 0, 0.25);
    color: var(--md-sys-color-on-surface);
  }

  .alert-copy {
    font-size: 0.95rem;
  }

  .banner-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .banner-button {
    border: none;
    border-radius: 12px;
    padding: 0.65rem 1rem;
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font-weight: 600;
  }

  .banner-button.secondary {
    background: rgba(0, 0, 0, 0.08);
  }

  .banner-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 0.25rem;
  }

  .header-title h1 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
  }

  .reset-countdown {
    font-size: 0.75rem;
    color: var(--md-sys-color-primary);
    font-weight: 600;
    margin: 0;
    letter-spacing: 0.3px;
    text-transform: uppercase;
  }

  .header-info {
    font-size: 0.875rem;
    color: var(--md-sys-color-on-surface-variant);
    font-weight: 500;
  }

  .settings-sub-tabs {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .settings-tab-button {
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

  @media (max-width: 768px) {
    .header {
      padding: 0.75rem 1rem;
    }

    .header-title h1 {
      font-size: 1.25rem;
    }

    .settings-sub-tabs {
      gap: 0.25rem;
      flex-wrap: wrap;
    }

    .settings-tab-button {
      padding: 0.4rem 0.8rem;
      font-size: 0.7rem;
    }

    .content {
      padding: 0;
    }
  }

  .content {
    flex: 1;
    overflow-y: auto;
    background: var(--md-sys-color-background);
  }

  /* Modern Dark Grey Theme with Orange Accents */
  :root {
    --md-sys-color-primary: #ff6b35;
    --md-sys-color-on-primary: #ffffff;
    --md-sys-color-primary-container: #ff8c42;
    --md-sys-color-on-primary-container: #3d1a00;
    
    --md-sys-color-secondary: #6b7280;
    --md-sys-color-on-secondary: #ffffff;
    --md-sys-color-secondary-container: #8b9dc3;
    --md-sys-color-on-secondary-container: #1a1a1a;
    
    --md-sys-color-tertiary: #ff8c42;
    --md-sys-color-on-tertiary: #ffffff;
    --md-sys-color-tertiary-container: #ffab55;
    --md-sys-color-on-tertiary-container: #3d1a00;
    
    --md-sys-color-surface: #1a1d23;
    --md-sys-color-on-surface: #e8eaed;
    --md-sys-color-surface-variant: #25262b;
    --md-sys-color-on-surface-variant: #d1d5db;
    --md-sys-color-surface-container: #2c3142;
    --md-sys-color-on-surface-container: #e8eaed;
    --md-sys-color-surface-container-highest: #323844;
    
    --md-sys-color-outline: #3c4043;
    --md-sys-color-outline-variant: #4a5568;
    --md-sys-color-background: #141418;
    --md-sys-color-on-background: #e8eaed;
    
    --md-sys-color-error: #cf6679;
    --md-sys-color-on-error: #ffffff;
    --md-sys-color-error-container: #ffb3ba;
    --md-sys-color-on-error-container: #3d1a00;
  }

  @media (max-width: 768px) {
    .header {
      padding: 0.75rem 1rem;
    }

    .header-title h1 {
      font-size: 1.25rem;
    }

    .content {
      padding: 0;
    }
  }
</style>
