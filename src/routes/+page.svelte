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
  import SetupGuide from '$lib/components/SetupGuide.svelte';
  import { initializeApp, activeFilterCharId, nextDailyReset, updateAvailable, latestAppVersion, currentAppVersion, isUpdateChecking, checkForAppUpdates, characters } from '$lib/store';
  import { invoke } from '@tauri-apps/api/core';
  import { GAME_TASKS } from '$lib/data/tasks';
  import { RAIDS } from '$lib/data/raids';
  import { GAME_CLASSES } from '$lib/data/classes';

  import { testSyncRoster } from '$lib/store';
  import { listen } from '@tauri-apps/api/event';

  type DiscordAuthState = 'checking' | 'login' | 'authorizing' | 'welcome' | 'approved' | 'denied' | 'error';

  interface DiscordAuthResult {
    approved: boolean;
    user_id?: string;
    username?: string;
    message: string;
  }

  let activeTab = 'dashboard';
  let sidebarOpen = false;
  let headerContent = '';
  let activeSettingsTab = 'roster';
  let activeProgressionTab = 'market_prices';
  let nextResetTime = '';
  let resetCountdown = '';
  let appReady = false;
  let showSetupGuideButton = true;
  let showAuthWelcome = true;
  let discordAuthState: DiscordAuthState = 'checking';
  let discordAuthMessage = 'Checking Discord access...';
  let discordAuthUser = '';
  let appInitializationStarted = false;

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
    const handleSetupGuideButtonChanged = (event: Event) => {
      const customEvent = event as CustomEvent<boolean>;
      showSetupGuideButton = customEvent.detail;
    };

    window.addEventListener('setup-guide-button:changed', handleSetupGuideButtonChanged);

    (async () => {
      await loadSystemPreferences();
      await checkStoredDiscordAuth();
      if (discordAuthState !== 'approved') {
        return;
      }
      await initializeAuthorizedApp();
    })();

    // Update countdown every second from cached reset timestamp.
    const countdownInterval = setInterval(updateResetCountdown, 1000);
    // Refresh backend reset timestamp only once per minute.
    const resetRefreshInterval = setInterval(refreshNextResetTimeFromBackend, 60000);

    // Cleanup on unmount
    return () => {
      window.removeEventListener('setup-guide-button:changed', handleSetupGuideButtonChanged);
      clearInterval(countdownInterval);
      clearInterval(resetRefreshInterval);
    };
  });

  async function checkStoredDiscordAuth() {
    try {
      const result = await invoke<DiscordAuthResult>('verify_stored_discord_auth');
      handleDiscordAuthResult(result);
    } catch (error) {
      discordAuthState = 'login';
      discordAuthMessage = `Discord auth could not be checked: ${error}`;
    }
  }

  async function loginWithDiscord() {
    try {
      discordAuthState = 'authorizing';
      discordAuthMessage = 'Opening Discord login in your browser...';
      const result = await invoke<DiscordAuthResult>('authenticate_discord');
      handleDiscordAuthResult(result);

      if (result.approved && !showAuthWelcome) {
        await initializeAuthorizedApp();
      }
    } catch (error) {
      discordAuthState = 'error';
      discordAuthMessage = `${error}`;
    }
  }

  function handleDiscordAuthResult(result: DiscordAuthResult) {
    discordAuthMessage = result.message;
    discordAuthUser = result.username ?? result.user_id ?? '';

    if (result.approved) {
      discordAuthState = showAuthWelcome ? 'welcome' : 'approved';
    } else if (!result.user_id) {
      discordAuthState = 'login';
    } else {
      discordAuthState = 'denied';
    }

    if (discordAuthState === 'welcome' || discordAuthState === 'denied') {
      refreshTenorEmbeds();
    }
  }

  function retryDiscordLogin() {
    discordAuthState = 'login';
    discordAuthMessage = 'Sign in with Discord to access LOA Tracker.';
    discordAuthUser = '';
  }

  async function proceedFromWelcome() {
    discordAuthState = 'approved';
    await initializeAuthorizedApp();
  }

  function refreshTenorEmbeds() {
    if (typeof window === 'undefined') {
      return;
    }

    window.setTimeout(() => {
      document.querySelector('script[data-tenor-reload="true"]')?.remove();
      const script = document.createElement('script');
      script.src = 'https://tenor.com/embed.js';
      script.async = true;
      script.setAttribute('data-tenor-reload', 'true');
      document.body.appendChild(script);
    }, 0);
  }

  async function initializeAuthorizedApp() {
    if (appInitializationStarted) {
      return;
    }
    appInitializationStarted = true;

    try {
      await initializeApp();
      await loadSystemPreferences();
      appReady = true;
      checkForAppUpdates().catch((error) => console.warn('Update check failed:', error));

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
    } catch (error) {
      appInitializationStarted = false;
      console.error('Failed to initialize authorized app:', error);
    }
  }

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

  function switchSettingsTab(tab: string) {
    activeSettingsTab = tab;
  }

  function startSetupGuide() {
    window.dispatchEvent(new CustomEvent('setup-guide:start'));
  }

  async function loadSystemPreferences() {
    try {
      const settings: any = await invoke('get_system_settings');
      showSetupGuideButton = settings.showSetupGuideButton ?? settings.show_setup_guide_button ?? true;
      showAuthWelcome = settings.showAuthWelcome ?? settings.show_auth_welcome ?? true;
    } catch (error) {
      console.warn('Failed to load system preferences:', error);
    }
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
      const totalMinutes = Math.ceil(diff / (1000 * 60));
      const hours = Math.floor(totalMinutes / 60);
      const minutes = totalMinutes % 60;
      const formatTimePart = (value: number) => value.toString().padStart(2, '0');

      if (hours > 0) {
        resetCountdown = `Next daily reset in: ${formatTimePart(hours)}H ${formatTimePart(minutes)}M`;
      } else {
        resetCountdown = `Next daily reset in: ${formatTimePart(minutes)}M`;
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

  async function updateCountdownFromKnownReset() {
    if (!nextResetTime) {
      await refreshNextResetTimeFromBackend();
    }
    updateResetCountdown();
  }
</script>

<svelte:head>
  <script type="text/javascript" async src="https://tenor.com/embed.js"></script>
</svelte:head>

{#if discordAuthState !== 'approved'}
  <div class="auth-screen">
    <div class="auth-card">
      <div class="auth-topline">
        <div class="auth-brand">
          <img src="/images/LOAtracker_icon.png" alt="" class="auth-icon" />
          <span>LOA Tracker</span>
        </div>
        <span class="auth-badge">Private Access</span>
      </div>
      <h1>
        {#if discordAuthState === 'welcome'}
          Welcome, {discordAuthUser}
        {:else}
          Only for MeowGang members
        {/if}
      </h1>
      <p class="auth-message">
        {#if discordAuthState === 'welcome'}
          Discord access verified.
        {:else if discordAuthState === 'denied'}
          Not approved by our Meowtator
        {:else}
          {discordAuthMessage}
        {/if}
      </p>
      {#if discordAuthState === 'welcome'}
        <div class="welcome-gif-frame">
          <div
            class="tenor-gif-embed"
            data-postid="16242995"
            data-share-method="host"
            data-aspect-ratio="1"
            data-width="100%"
          >
            <a href="https://tenor.com/view/hello-cute-cat-hi-greetings-gif-16242995">Hello Cute GIF</a>
            from <a href="https://tenor.com/search/hello-gifs">Hello GIFs</a>
          </div>
        </div>
      {/if}
      {#if discordAuthState === 'denied'}
        <div class="denied-gif-frame">
          <div
            class="tenor-gif-embed"
            data-postid="17205935"
            data-share-method="host"
            data-aspect-ratio="1"
            data-width="100%"
          >
            <a href="https://tenor.com/view/cat-animation-slap-gif-17205935">Cat Animation Sticker</a>
            from <a href="https://tenor.com/search/cat-stickers">Cat Stickers</a>
          </div>
          <div class="cat-slap-gif" aria-label="Cat slapping animation">
            <span class="cat-face">:3</span>
            <span class="cat-paw"></span>
          </div>
        </div>
      {/if}

      {#if discordAuthState === 'checking'}
        <button class="auth-button" type="button" disabled>Checking...</button>
      {:else if discordAuthState === 'authorizing'}
        <button class="auth-button" type="button" disabled>Waiting for Discord...</button>
      {:else if discordAuthState === 'welcome'}
        <button class="auth-button" type="button" on:click={proceedFromWelcome}>Proceed</button>
      {:else if discordAuthState === 'denied'}
        <button class="auth-button" type="button" on:click={retryDiscordLogin}>Try another account</button>
      {:else}
        <button class="auth-button" type="button" on:click={loginWithDiscord}>Login with Discord</button>
      {/if}
    </div>
  </div>
{:else}
<div class="app">
  <!-- Sidebar -->
  <Sidebar {activeTab} {switchTab} isOpen={sidebarOpen} {discordAuthUser} />

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
          {#if showSetupGuideButton}
            <button class="setup-guide-button" type="button" on:click={startSetupGuide}>Set-Up Guide</button>
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

      <!-- Progression Sub-Tabs (only shown when progression tab is active) -->
      {#if activeTab === 'progression'}
        <div class="settings-sub-tabs">
          <button
            class="settings-tab-button"
            class:active={activeProgressionTab === 'market_prices'}
            on:click={() => activeProgressionTab = 'market_prices'}
          >
            Market Prices
          </button>
          <button
            class="settings-tab-button"
            class:active={activeProgressionTab === 'planner'}
            on:click={() => activeProgressionTab = 'planner'}
          >
            Planner
          </button>
        </div>
      {/if}
    </header>

    <!-- Tab Content -->
    <main class="content" class:outer-scroll={activeTab !== 'todo' && activeTab !== 'settings'}>
      {#if activeTab === 'dashboard'}
        <Dashboard {setHeaderContent} />
      {:else if activeTab === 'todo'}
        <Todo highlightCharId={$activeFilterCharId} />
      {:else if activeTab === 'settings'}
        <Settings activeSettingsTab={activeSettingsTab} on:tabChange={(e: CustomEvent<string>) => activeSettingsTab = e.detail} />
      {:else if activeTab === 'progression'}
        <ProgressionPlanner activeProgressionTab={activeProgressionTab} on:tabChange={(e: CustomEvent<string>) => activeProgressionTab = e.detail} />
      {:else if activeTab === 'updates'}
        <UpdateTab />
      {:else if activeTab === 'encounters'}
        <EncounterSyncStatus />
      {/if}
    </main>
  </div>

  <SetupGuide
    {activeTab}
    {activeSettingsTab}
    {appReady}
    characterCount={$characters.length}
    {switchTab}
    setSettingsTab={switchSettingsTab}
  />
</div>
{/if}

<style>
  :global(body) {
    margin: 0;
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: var(--md-sys-color-background);
    color: var(--md-sys-color-on-background);
  }

  .auth-screen {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1.5rem;
    box-sizing: border-box;
    background:
      linear-gradient(135deg, rgba(255, 140, 0, 0.1), transparent 32%),
      linear-gradient(315deg, rgba(255, 215, 0, 0.05), transparent 38%),
      var(--md-sys-color-background);
  }

  .auth-card {
    width: min(100%, 390px);
    box-sizing: border-box;
    background: color-mix(in srgb, var(--surface-variant) 94%, #000000);
    border: 1px solid rgba(255, 140, 0, 0.25);
    border-radius: 8px;
    padding: 1.15rem;
    text-align: left;
  }

  .auth-topline {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .auth-brand {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.55rem;
    color: var(--on-surface);
    font-size: 0.95rem;
    font-weight: 800;
    line-height: 1;
  }

  .auth-icon {
    width: 34px;
    height: 34px;
    border-radius: 8px;
    object-fit: contain;
    flex: 0 0 34px;
  }

  .auth-badge {
    border: 1px solid rgba(255, 140, 0, 0.25);
    border-radius: 999px;
    padding: 0.28rem 0.55rem;
    color: var(--on-surface-variant);
    font-size: 0.68rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0;
  }

  .auth-card h1 {
    margin: 0 0 0.45rem;
    color: var(--on-surface);
    font-size: 1.25rem;
    line-height: 1.15;
  }

  .auth-message {
    margin: 0;
    color: var(--on-surface-variant);
    line-height: 1.4;
    font-size: 0.85rem;
  }

  .denied-gif-frame,
  .welcome-gif-frame {
    position: relative;
    min-height: 150px;
    margin-top: 1rem;
    border: 1px solid rgba(255, 140, 0, 0.18);
    border-radius: 8px;
    background: color-mix(in srgb, var(--surface-variant) 90%, #000000);
    overflow: hidden;
  }

  .tenor-gif-embed {
    position: relative;
    z-index: 2;
    width: 100%;
    min-height: 150px;
  }

  .tenor-gif-embed a {
    color: transparent;
    font-size: 0;
  }

  .cat-slap-gif {
    position: absolute;
    inset: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.35rem;
  }

  .cat-face {
    width: 44px;
    height: 44px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--primary);
    color: var(--on-primary);
    font-size: 1rem;
    font-weight: 900;
  }

  .cat-paw {
    width: 34px;
    height: 16px;
    border-radius: 999px;
    background: #f4f0ea;
    transform-origin: left center;
    animation: slap 0.72s ease-in-out infinite;
  }

  .cat-paw::after {
    content: '';
    display: block;
    width: 10px;
    height: 10px;
    margin-left: 24px;
    margin-top: 3px;
    border-radius: 50%;
    background: #ffb86c;
  }

  @keyframes slap {
    0%, 100% { transform: translateX(8px) rotate(18deg); }
    48% { transform: translateX(-18px) rotate(-14deg); }
    58% { transform: translateX(-18px) rotate(-14deg); }
  }

  .auth-button {
    width: 100%;
    margin-top: 1rem;
    border: 0;
    border-radius: 8px;
    padding: 0.68rem 1rem;
    background: var(--primary);
    color: var(--on-primary);
    font-size: 0.9rem;
    font-weight: 800;
    cursor: pointer;
  }

  .auth-button:disabled {
    cursor: default;
    opacity: 0.72;
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

  .setup-guide-button {
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    padding: 0.45rem 0.7rem;
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font-size: 0.75rem;
    font-weight: 700;
    white-space: nowrap;
  }

  .setup-guide-button:hover {
    border-color: var(--md-sys-color-primary);
    color: var(--md-sys-color-primary);
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
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--md-sys-color-background);
  }

  .content.outer-scroll {
    overflow-y: auto;
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
