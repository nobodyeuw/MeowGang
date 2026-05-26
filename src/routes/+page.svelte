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
  import MeowConnect from '$lib/components/MeowConnect.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import SetupGuide from '$lib/components/SetupGuide.svelte';
  import {
    getStoredSupabaseDiscordAuth,
    signInWithSupabaseDiscord,
    type DiscordAuthResult
  } from '$lib/services/supabase-auth';
  import {
    hasMeowConnectConsent,
    isMeowConnectFeatureEnabled,
    isMeowConnectRealtimeEnabled,
    loadMeowConnectPendingRequests,
    logMeowConnectRequest,
    markMeowConnectActive,
    markMeowConnectConnecting,
    markMeowConnectFailure,
    meowConnectStatus,
    uploadMeowConnectSnapshotIfNeeded,
    type MeowConnectFriendConnection,
    type MeowConnectGroup
  } from '$lib/services/meow-connect';
  import { initializeApp, activeFilterCharId, nextDailyReset, updateAvailable, latestAppVersion, currentAppVersion, isUpdateChecking, checkForAppUpdates, characters } from '$lib/store';
  import { invoke } from '@tauri-apps/api/core';
  import { GAME_TASKS } from '$lib/data/tasks';
  import { RAIDS } from '$lib/data/raids';
  import { GAME_CLASSES } from '$lib/data/classes';

  import { testSyncRoster } from '$lib/store';
  import { listen } from '@tauri-apps/api/event';

  const VALID_TABS = new Set(['dashboard', 'todo', 'settings', 'progression', 'meow-connect', 'updates', 'encounters']);

  type DiscordAuthState = 'checking' | 'login' | 'authorizing' | 'welcome' | 'approved' | 'denied' | 'error';
  type MeowConnectSection = 'together' | 'logs' | 'settings';

  let activeTab = 'dashboard';
  let sidebarOpen = false;
  let headerContent = '';
  let activeSettingsTab = 'roster';
  let activeProgressionTab = 'market_prices';
  let activeMeowConnectTab: MeowConnectSection = 'together';
  let pendingMeowConnectRequests = 0;
  let pendingMeowConnectFriendRequests: MeowConnectFriendConnection[] = [];
  let pendingMeowConnectGroupInvites: MeowConnectGroup[] = [];
  let meowConnectFriendRequestRefreshTimer: ReturnType<typeof setTimeout> | null = null;
  let nextResetTime = '';
  let resetCountdown = '';
  let showHeaderCountdown = true;
  let appReady = false;
  let showSetupGuideButton = true;
  let showAuthWelcome = true;
  let startWithLoaLogsEnabled = false;
  let loaLogsPathConfigured = false;
  let loaLogsReminderShown = false;
  let loaLogsReminderMessage = '';
  let discordAuthState: DiscordAuthState = 'checking';
  let discordAuthMessage = 'Checking Discord access...';
  let discordAuthUser = '';
  let appInitializationStarted = false;
  let meowConnectHeaderState: 'inactive' | 'connecting' | 'active' | 'sleeping' | 'offline' | 'login_required' = 'inactive';
  let meowConnectHeaderMessage = 'MeowConnect is inactive.';
  let meowConnectHeaderLabel = 'Inactive';
  let meowConnectFeatureEnabled = true;
  let meowConnectRealtimeEnabled = true;
  let meowConnectCompletionUploadTimer: ReturnType<typeof setTimeout> | null = null;
  const MEOWCONNECT_PENDING_IDLE_REFRESH_MS = 3 * 60 * 1000;
  const LEGACY_BROWSER_STORAGE_PATTERNS = [
    'party_plans',
    'partyplanner',
    'party-planner',
    'google_script',
    'google-script',
    'apps_script',
    'apps-script',
    'script.google.com'
  ];

  // Handle URL parameters
  $: urlParams = new URLSearchParams($page.url.search);
  $: tabFromUrl = urlParams.get('tab') || 'dashboard';
  $: charFromUrl = urlParams.get('char');

  // Update active tab when URL changes
  $: if (tabFromUrl !== activeTab) {
    activeTab = VALID_TABS.has(tabFromUrl) ? tabFromUrl : 'dashboard';
    if (activeTab !== tabFromUrl) {
      goto('/?tab=dashboard');
    }
  }

  $: if (!meowConnectFeatureEnabled && activeTab === 'meow-connect') {
    switchTab('dashboard');
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
    const handleHeaderCountdownChanged = (event: Event) => {
      const customEvent = event as CustomEvent<boolean>;
      showHeaderCountdown = customEvent.detail;
    };
    const handleMeowConnectConsentChanged = () => {
      refreshMeowConnectHeaderStatus();
      scheduleMeowConnectFriendRequestRefresh();
    };
    const handleMeowConnectFeatureChanged = () => {
      refreshMeowConnectFeatureSettings();
      scheduleMeowConnectFriendRequestRefresh();
      if (!meowConnectFeatureEnabled && activeTab === 'meow-connect') {
        switchTab('dashboard');
      }
    };
    const handleMeowConnectRealtimeChanged = () => {
      refreshMeowConnectFeatureSettings();
    };
    const handleMeowConnectCompletionChanged = () => {
      scheduleMeowConnectCompletionUpload();
      scheduleMeowConnectFriendRequestRefresh();
    };
    const unsubscribeMeowConnectStatus = meowConnectStatus.subscribe((status) => {
      meowConnectHeaderState = status.state;
      meowConnectHeaderMessage = status.message;
      meowConnectHeaderLabel = getMeowConnectHeaderLabel(status.state);
    });
    let unlistenMeowConnectScrape: (() => void) | null = null;

    window.addEventListener('setup-guide-button:changed', handleSetupGuideButtonChanged);
    window.addEventListener('header-countdown:changed', handleHeaderCountdownChanged);
    window.addEventListener('meow-connect-consent-changed', handleMeowConnectConsentChanged);
    window.addEventListener('meow-connect-feature-changed', handleMeowConnectFeatureChanged);
    window.addEventListener('meow-connect-realtime-changed', handleMeowConnectRealtimeChanged);
    window.addEventListener('raid-completed', handleMeowConnectCompletionChanged);
    cleanupLegacyBrowserStorage();
    refreshHeaderCountdownPreference();
    refreshMeowConnectFeatureSettings();
    refreshMeowConnectHeaderStatus();

    (async () => {
      unlistenMeowConnectScrape = await listen('meow-connect-roster-scrape-complete', () => {
        void syncMeowConnectAfterRosterScrape();
      });
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
    const loaLogsStatusInterval = setInterval(clearLoaLogsReminderWhenRunning, 5000);
    const meowConnectFriendRequestInterval = setInterval(refreshMeowConnectFriendRequests, MEOWCONNECT_PENDING_IDLE_REFRESH_MS);

    // Cleanup on unmount
    return () => {
      unlistenMeowConnectScrape?.();
      unsubscribeMeowConnectStatus();
      if (meowConnectCompletionUploadTimer) clearTimeout(meowConnectCompletionUploadTimer);
      if (meowConnectFriendRequestRefreshTimer) clearTimeout(meowConnectFriendRequestRefreshTimer);
      window.removeEventListener('setup-guide-button:changed', handleSetupGuideButtonChanged);
      window.removeEventListener('header-countdown:changed', handleHeaderCountdownChanged);
      window.removeEventListener('meow-connect-consent-changed', handleMeowConnectConsentChanged);
      window.removeEventListener('meow-connect-feature-changed', handleMeowConnectFeatureChanged);
      window.removeEventListener('meow-connect-realtime-changed', handleMeowConnectRealtimeChanged);
      window.removeEventListener('raid-completed', handleMeowConnectCompletionChanged);
      clearInterval(countdownInterval);
      clearInterval(resetRefreshInterval);
      clearInterval(loaLogsStatusInterval);
      clearInterval(meowConnectFriendRequestInterval);
    };
  });

  function refreshHeaderCountdownPreference() {
    showHeaderCountdown = localStorage.getItem('showHeaderCountdown') !== '0';
  }

  function cleanupLegacyBrowserStorage() {
    cleanupLegacyStorageArea(localStorage);
    cleanupLegacyStorageArea(sessionStorage);
  }

  function cleanupLegacyStorageArea(storage: Storage) {
    const keysToRemove: string[] = [];
    for (let index = 0; index < storage.length; index += 1) {
      const key = storage.key(index);
      if (!key) continue;
      const value = storage.getItem(key) || '';
      const combined = `${key}\n${value}`.toLowerCase();
      if (LEGACY_BROWSER_STORAGE_PATTERNS.some((pattern) => combined.includes(pattern))) {
        keysToRemove.push(key);
      }
    }

    for (const key of keysToRemove) {
      storage.removeItem(key);
    }
  }

  async function checkStoredDiscordAuth() {
    try {
      const result = await getStoredSupabaseDiscordAuth();
      if (result) {
        handleDiscordAuthResult(result);
        return;
      }

      discordAuthState = 'login';
      discordAuthMessage = 'Sign in with Discord to access LOA Tracker.';
      discordAuthUser = '';
    } catch (error) {
      discordAuthState = 'login';
      discordAuthMessage = `Discord auth could not be checked: ${error}`;
    }
  }

  async function loginWithDiscord() {
    try {
      discordAuthState = 'authorizing';
      discordAuthMessage = 'Opening Discord login in your browser...';
      const result = await signInWithSupabaseDiscord();
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
      refreshMeowConnectHeaderStatus();
      appReady = true;
      await showLoaLogsReminderIfNeeded();
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
        window.dispatchEvent(new CustomEvent('character-data-complete'));

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

        await syncMeowConnectOnAppStart();
      } catch (error) {
        console.error('?? Data completeness check failed:', error);
      }
    } catch (error) {
      appInitializationStarted = false;
      console.error('Failed to initialize authorized app:', error);
    }
  }

  async function syncMeowConnectOnAppStart() {
    if (!meowConnectFeatureEnabled || !hasMeowConnectConsent()) {
      refreshMeowConnectHeaderStatus();
      return;
    }

    try {
      markMeowConnectConnecting('Checking MeowConnect startup sync.');
      const result = await uploadMeowConnectSnapshotIfNeeded();
      markMeowConnectActive(
        result.uploaded
          ? 'MeowConnect startup sync succeeded.'
          : 'MeowConnect is connected.'
      );
      console.log(
        result.uploaded
          ? `MeowConnect startup sync uploaded ${result.snapshot.characters.length} characters.`
          : 'MeowConnect startup sync skipped; local snapshot unchanged.'
      );
      await refreshMeowConnectFriendRequests();
    } catch (error) {
      markMeowConnectFailure(error);
      console.warn('MeowConnect startup sync failed:', error);
      await refreshMeowConnectFriendRequests();
    }
  }

  async function syncMeowConnectAfterRosterScrape() {
    if (!meowConnectFeatureEnabled || !hasMeowConnectConsent()) {
      refreshMeowConnectHeaderStatus();
      return;
    }

    try {
      markMeowConnectConnecting('Syncing MeowConnect after roster scrape.');
      const result = await uploadMeowConnectSnapshotIfNeeded({ force: true });
      markMeowConnectActive('MeowConnect roster scrape sync succeeded.');
      console.log(
        result.uploaded
          ? `MeowConnect roster scrape sync uploaded ${result.snapshot.characters.length} characters.`
          : 'MeowConnect roster scrape sync skipped.'
      );
      await refreshMeowConnectFriendRequests();
    } catch (error) {
      markMeowConnectFailure(error);
      console.warn('MeowConnect roster scrape sync failed:', error);
      await refreshMeowConnectFriendRequests();
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
    if (tab === 'meow-connect' && !meowConnectFeatureEnabled) {
      tab = 'dashboard';
    }
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

  function openMeowConnectRequests() {
    activeMeowConnectTab = 'settings';
    switchTab('meow-connect');
  }

  function handlePendingRequestsChanged(count: number) {
    pendingMeowConnectRequests = count;
    scheduleMeowConnectFriendRequestRefresh();
  }

  function scheduleMeowConnectFriendRequestRefresh() {
    if (meowConnectFriendRequestRefreshTimer) clearTimeout(meowConnectFriendRequestRefreshTimer);
    meowConnectFriendRequestRefreshTimer = setTimeout(() => {
      meowConnectFriendRequestRefreshTimer = null;
      void refreshMeowConnectFriendRequests();
    }, 500);
  }

  async function refreshMeowConnectFriendRequests() {
    if (!meowConnectFeatureEnabled || !hasMeowConnectConsent() || discordAuthState !== 'approved') {
      pendingMeowConnectFriendRequests = [];
      pendingMeowConnectGroupInvites = [];
      pendingMeowConnectRequests = 0;
      return;
    }

    const startedAt = performance.now();
    logMeowConnectRequest('Header pending request refresh started.');
    try {
      const { friendRequests, groupInvites } = await loadMeowConnectPendingRequests();
      pendingMeowConnectFriendRequests = friendRequests;
      pendingMeowConnectGroupInvites = groupInvites;
      pendingMeowConnectRequests = pendingMeowConnectFriendRequests.length + pendingMeowConnectGroupInvites.length;
      logMeowConnectRequest(
        `Header pending request refresh finished in ${Math.round(performance.now() - startedAt)}ms: friendRequests=${friendRequests.length}, groupInvites=${groupInvites.length}, pending=${pendingMeowConnectRequests}.`,
        'info'
      );
    } catch (error) {
      logMeowConnectRequest(`Header pending request refresh failed: ${error}`, 'warn');
      console.warn('Failed to refresh MeowConnect friend request notifications:', error);
    }
  }

  function getInitials(name: string): string {
    const parts = name.trim().split(/\s+/).filter(Boolean);
    return (parts[0]?.[0] || '?').toUpperCase() + (parts[1]?.[0] || '').toUpperCase();
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
      startWithLoaLogsEnabled = settings.startWithLoaLogs ?? settings.start_with_loa_logs ?? false;
      const loaLogsPath = settings.loaLogsExePath ?? settings.loa_logs_exe_path ?? '';
      loaLogsPathConfigured = typeof loaLogsPath === 'string' && loaLogsPath.trim().length > 0;
    } catch (error) {
      console.warn('Failed to load system preferences:', error);
    }
  }

  async function showLoaLogsReminderIfNeeded() {
    if (!startWithLoaLogsEnabled || loaLogsReminderShown) {
      return;
    }

    try {
      const isRunning = await invoke<boolean>('is_loa_logs_running');
      if (isRunning) {
        loaLogsReminderMessage = '';
        return;
      }

      loaLogsReminderShown = true;
      loaLogsReminderMessage = loaLogsPathConfigured
        ? 'Do not forget to start LOA Logs.exe for maximum efficiency.'
        : 'For better QoL you should install LOA Logs.exe or set the path manually in Settings.';
    } catch (error) {
      console.warn('Failed to check LOA Logs reminder state:', error);
    }
  }

  async function clearLoaLogsReminderWhenRunning() {
    if (!loaLogsReminderMessage) {
      return;
    }

    try {
      const isRunning = await invoke<boolean>('is_loa_logs_running');
      if (isRunning) {
        loaLogsReminderMessage = '';
      }
    } catch (error) {
      console.warn('Failed to refresh LOA Logs reminder state:', error);
    }
  }

  function dismissLoaLogsReminder() {
    loaLogsReminderMessage = '';
  }

  function setHeaderContent(content: string) {
    headerContent = content;
  }

  function refreshMeowConnectHeaderStatus() {
    if (!meowConnectFeatureEnabled || !hasMeowConnectConsent()) {
      meowConnectStatus.set({
        state: 'inactive',
        message: meowConnectFeatureEnabled ? 'MeowConnect is inactive.' : 'MeowConnect is disabled.',
        updatedAt: Date.now()
      });
    }
  }

  function refreshMeowConnectFeatureSettings() {
    meowConnectFeatureEnabled = isMeowConnectFeatureEnabled();
    meowConnectRealtimeEnabled = isMeowConnectRealtimeEnabled();
    refreshMeowConnectHeaderStatus();
  }

  function scheduleMeowConnectCompletionUpload() {
    if (!meowConnectFeatureEnabled || !meowConnectRealtimeEnabled || !hasMeowConnectConsent()) return;
    if (meowConnectCompletionUploadTimer) clearTimeout(meowConnectCompletionUploadTimer);
    meowConnectCompletionUploadTimer = setTimeout(() => {
      void syncMeowConnectAfterCompletionChange();
    }, 1200);
  }

  async function syncMeowConnectAfterCompletionChange() {
    if (!meowConnectFeatureEnabled || !meowConnectRealtimeEnabled || !hasMeowConnectConsent()) return;

    try {
      markMeowConnectConnecting('Syncing MeowConnect completion update.');
      const result = await uploadMeowConnectSnapshotIfNeeded({ force: true });
      markMeowConnectActive(
        result.uploaded
          ? 'MeowConnect completion update synced.'
          : 'MeowConnect completion update checked.'
      );
    } catch (error) {
      markMeowConnectFailure(error);
      console.warn('MeowConnect completion sync failed:', error);
    }
  }

  function getMeowConnectHeaderLabel(state: typeof meowConnectHeaderState): string {
    if (state === 'active') return 'Active';
    if (state === 'connecting') return 'Connecting';
    if (state === 'sleeping') return 'Sleeping';
    if (state === 'offline') return 'Offline';
    if (state === 'login_required') return 'Login required';
    return 'Inactive';
  }

  function toggleSidebar() {
    sidebarOpen = !sidebarOpen;
  }

  // Update reset countdown
  function updateResetCountdown() {
    const weeklyResetWindow = getWeeklyResetCountdownTarget();
    const targetResetTime = weeklyResetWindow ? weeklyResetWindow.toISOString() : nextResetTime;
    const resetLabel = weeklyResetWindow ? 'Next weekly reset in' : 'Next daily reset in';

    if (!targetResetTime) {
      resetCountdown = 'Reset timer unavailable';
      return;
    }

    const now = new Date();
    const reset = new Date(targetResetTime);
    const diff = reset.getTime() - now.getTime();

    if (diff > 0) {
      const totalMinutes = Math.ceil(diff / (1000 * 60));
      const hours = Math.floor(totalMinutes / 60);
      const minutes = totalMinutes % 60;
      const formatTimePart = (value: number) => value.toString().padStart(2, '0');

      if (hours > 0) {
        resetCountdown = `${resetLabel}: ${formatTimePart(hours)}H ${formatTimePart(minutes)}M`;
      } else {
        resetCountdown = `${resetLabel}: ${formatTimePart(minutes)}M`;
      }
    } else {
      resetCountdown = weeklyResetWindow ? 'Weekly reset should have occurred!' : 'Daily reset should have occurred!';
    }
  }

  function getWeeklyResetCountdownTarget(): Date | null {
    const now = new Date();
    const day = now.getUTCDay();
    const currentDailyReset = new Date(now);
    currentDailyReset.setUTCHours(10, 0, 0, 0);

    const isAfterTuesdayDailyReset = day === 2 && now.getTime() >= currentDailyReset.getTime();
    const isBeforeWednesdayWeeklyReset = day === 3 && now.getTime() < currentDailyReset.getTime();

    if (!isAfterTuesdayDailyReset && !isBeforeWednesdayWeeklyReset) {
      return null;
    }

    const weeklyReset = new Date(now);
    weeklyReset.setUTCHours(10, 0, 0, 0);
    if (day === 2) {
      weeklyReset.setUTCDate(weeklyReset.getUTCDate() + 1);
    }

    return weeklyReset;
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
  <Sidebar {activeTab} {switchTab} isOpen={sidebarOpen} {discordAuthUser} showMeowConnect={meowConnectFeatureEnabled} />

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
          <button
            type="button"
            class="app-title-button"
            on:click={() => activeTab !== 'dashboard' && switchTab('dashboard')}
            aria-label="Go to dashboard"
          >
            <img src="/images/LOAtracker_header.png" alt="LOA Tracker" class="app-title-logo" />
          </button>
          {#if showHeaderCountdown && resetCountdown}
            <div class="reset-countdown">{resetCountdown}</div>
          {/if}
          {#if meowConnectFeatureEnabled}
            <div
              class="meowconnect-header-status"
              class:active={meowConnectHeaderState === 'active'}
              class:connecting={meowConnectHeaderState === 'connecting' || meowConnectHeaderState === 'sleeping'}
              class:inactive={meowConnectHeaderState === 'inactive'}
              class:offline={meowConnectHeaderState === 'offline' || meowConnectHeaderState === 'login_required'}
              title={meowConnectHeaderMessage}
            >
              <img src="/images/meowconnect_tab.png" alt="" />
              <span>{meowConnectHeaderLabel}</span>
            </div>
          {/if}
          {#if pendingMeowConnectRequests > 0}
            <button
              type="button"
              class="meowconnect-request-alert"
              title={`${pendingMeowConnectRequests} incoming MeowConnect request${pendingMeowConnectRequests === 1 ? '' : 's'}`}
              on:click={openMeowConnectRequests}
            >
              <span class="request-avatar-stack">
                {#if pendingMeowConnectFriendRequests.length > 0}
                  {#each pendingMeowConnectFriendRequests.slice(0, 3) as request, requestIndex}
                    {#if request.profile.avatarUrl}
                      <img
                        src={request.profile.avatarUrl}
                        alt=""
                        title={request.profile.displayName}
                        style={`--request-avatar-index: ${requestIndex}`}
                      />
                    {:else}
                      <span
                        class="request-avatar-fallback"
                        title={request.profile.displayName}
                        style={`--request-avatar-index: ${requestIndex}`}
                      >
                        {getInitials(request.profile.displayName)}
                      </span>
                    {/if}
                  {/each}
                {:else}
                  <img src="/images/meowconnect_tab.png" alt="" style="--request-avatar-index: 0" />
                {/if}
                {#if pendingMeowConnectRequests > 3}
                  <span class="request-avatar-overflow">+{pendingMeowConnectRequests - 3}</span>
                {/if}
              </span>
              <span>{pendingMeowConnectRequests} request{pendingMeowConnectRequests === 1 ? '' : 's'}</span>
            </button>
          {/if}
          {#if showSetupGuideButton}
            <button class="setup-guide-button" type="button" on:click={startSetupGuide}>Set-Up Guide</button>
          {/if}
        </div>
        {#if headerContent}
          <div class="header-info">{headerContent}</div>
        {/if}

        {#if loaLogsReminderMessage}
          <div class="app-alert loa-logs-reminder">
            <div class="alert-copy">
              <strong>LOA Logs:</strong> {loaLogsReminderMessage}
            </div>
            <button class="banner-button secondary" type="button" on:click={dismissLoaLogsReminder}>Dismiss</button>
          </div>
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

      <!-- MeowConnect Sub-Tabs (only shown when MeowConnect tab is active) -->
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
      {:else if activeTab === 'meow-connect' && meowConnectFeatureEnabled}
        <MeowConnect
          activeSection={activeMeowConnectTab}
          on:pendingRequestsChanged={(event: CustomEvent<number>) => handlePendingRequestsChanged(event.detail)}
        />
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
    padding: 0.55rem 1.5rem;
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

  .app-alert {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.75rem;
    align-items: center;
    padding: 0.9rem 1rem;
    margin-top: 0.75rem;
    border-radius: 16px;
    color: var(--md-sys-color-on-surface);
  }

  .update-banner {
    background: rgba(255, 210, 0, 0.12);
    border: 1px solid rgba(255, 191, 0, 0.25);
  }

  .loa-logs-reminder {
    background: rgba(255, 140, 0, 0.12);
    border: 1px solid rgba(255, 140, 0, 0.24);
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
    margin-bottom: 0.12rem;
  }

  .header-title h1 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
  }

  .app-title-button {
    border: none;
    background: transparent;
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font: inherit;
    font-size: 1.5rem;
    font-weight: 600;
    margin: 0;
    padding: 0;
  }

  .app-title-button:hover {
    color: var(--md-sys-color-primary);
  }

  .app-title-logo {
    width: 200px;
    height: 46px;
    display: block;
    object-fit: contain;
    object-position: center;
  }

  .reset-countdown {
    font-size: 0.75rem;
    color: var(--md-sys-color-primary);
    font-weight: 600;
    margin: 0;
    letter-spacing: 0.3px;
    text-transform: uppercase;
  }

  .meowconnect-header-status {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    margin-left: 0.65rem;
    font-size: 0.75rem;
    font-weight: 700;
    line-height: 1;
    letter-spacing: 0.3px;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .meowconnect-header-status img {
    width: 20px;
    height: 20px;
    object-fit: contain;
    display: block;
  }

  .meowconnect-header-status.active {
    color: var(--md-sys-color-primary);
  }

  .meowconnect-header-status.connecting {
    color: color-mix(in srgb, var(--md-sys-color-primary) 70%, var(--md-sys-color-on-surface-variant));
    opacity: 0.86;
  }

  .meowconnect-header-status.inactive,
  .meowconnect-header-status.offline {
    color: color-mix(in srgb, #ef4444 55%, var(--md-sys-color-on-surface-variant));
    opacity: 0.78;
  }

  .meowconnect-header-status.inactive img,
  .meowconnect-header-status.offline img {
    filter: grayscale(1);
    opacity: 0.46;
  }

  .meowconnect-request-alert {
    display: inline-flex;
    align-items: center;
    gap: 0.38rem;
    min-height: 1.9rem;
    padding: 0.22rem 0.52rem 0.22rem 0.3rem;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 52%, var(--md-sys-color-outline));
    border-radius: 999px;
    background: color-mix(in srgb, var(--md-sys-color-primary) 10%, var(--md-sys-color-surface-container));
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font-size: 0.72rem;
    font-weight: 800;
    line-height: 1;
    white-space: nowrap;
  }

  .meowconnect-request-alert:hover {
    border-color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 16%, var(--md-sys-color-surface-container));
  }

  .request-avatar-stack {
    position: relative;
    display: block;
    width: 2.95rem;
    height: 1.45rem;
  }

  .request-avatar-stack img,
  .request-avatar-fallback {
    position: absolute;
    top: 0;
    left: calc(var(--request-avatar-index, 0) * 0.78rem);
    width: 1.45rem;
    height: 1.45rem;
    border: 2px solid var(--md-sys-color-surface-container);
    border-radius: 50%;
    box-sizing: border-box;
  }

  .request-avatar-stack img {
    object-fit: cover;
  }

  .request-avatar-fallback,
  .request-avatar-overflow {
    display: grid;
    place-items: center;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    font-size: 0.58rem;
    font-weight: 900;
  }

  .request-avatar-overflow {
    position: absolute;
    right: -0.05rem;
    bottom: -0.08rem;
    min-width: 0.95rem;
    height: 0.95rem;
    padding: 0 0.12rem;
    border: 2px solid var(--md-sys-color-surface-container);
    border-radius: 999px;
    box-sizing: border-box;
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
    .header {
      padding: 0.45rem 1rem;
    }

    .header-title h1 {
      font-size: 1.25rem;
    }

    .app-title-logo {
      width: 160px;
      height: 36px;
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
      padding: 0.45rem 1rem;
    }

    .header-title h1 {
      font-size: 1.25rem;
    }

    .app-title-logo {
      width: 160px;
      height: 36px;
    }

    .content {
      padding: 0;
    }
  }
</style>
