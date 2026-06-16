<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { classAsset } from '$lib/assets';
  import Sidebar from '$lib/components/app/Sidebar.svelte';
  import SetupGuide from '$lib/components/app/SetupGuide.svelte';
  import AppAuthScreen from '$lib/components/app/AppAuthScreen.svelte';
  import AppContent from '$lib/components/app/AppContent.svelte';
  import AppHeader from '$lib/components/app/AppHeader.svelte';
  import {
    getStoredSupabaseDiscordAuth,
    signInWithSupabaseDiscord,
    type DiscordAuthResult
  } from '$lib/services/supabase-auth';
  import { loadDashboardSnapshot } from '$lib/services/dashboard';
  import {
    getHeaderCountdownPreference,
    getLoaLogsReminderMessage,
    isLoaLogsRunning,
    loadAppSystemPreferences
  } from '$lib/services/app-preferences';
  import {
    ensureCharacterDataComplete,
    getNextDailyResetTime,
    syncEncountersToCompletions,
    updateResetTimestamps,
    updateRestedValuesNow
  } from '$lib/services/app-startup';
  // Temporarily disabled due to Supabase realtime message limits
  // import {
  //   hasMeowConnectConsent,
  //   isMeowConnectFeatureEnabled,
  //   isMeowConnectFriendClearHintsEnabled,
  //   isMeowConnectRealtimeEnabled,
  //   applyFriendClearHintsToLocalSnapshot,
  //   fetchMeowConnectRemoteSnapshots,
  //   loadMeowConnectLocalSnapshot,
  //   loadMeowConnectPendingRequests,
  //   logMeowConnectRequest,
  //   markMeowConnectActive,
  //   markMeowConnectConnecting,
  //   markMeowConnectFailure,
  //   meowConnectStatus,
  //   subscribeMeowConnectChanges,
  //   uploadMeowConnectSnapshotIfNeeded,
  //   type MeowConnectFriendConnection,
  //   type MeowConnectGroup
  // } from '$lib/services/meow-connect';
  import { initializeApp, activeFilterCharId, nextDailyReset, updateAvailable, latestAppVersion, currentAppVersion, isUpdateChecking, checkForAppUpdates, characters, rosters } from '$lib/store';
  import type { AppTab, DiscordAuthState } from '$lib/types/app-shell';
  // Temporarily disabled due to Supabase realtime message limits
  // import type { MeowConnectHeaderState, MeowConnectSection } from '$lib/types/app-shell';
  import { formatResetCountdown, isAppTab } from '$lib/utils/app-shell';
  // Temporarily disabled due to Supabase realtime message limits
  // import { getMeowConnectHeaderLabel } from '$lib/utils/app-shell';
  import { cleanupLegacyBrowserStorage } from '$lib/utils/browser-storage';
  import { reloadTenorEmbeds } from '$lib/utils/tenor';
  import { applyTheme } from '$lib/services/theme-preferences';
  import { hasRaidManagementAccess, hasRaidManagementAccessRemote } from '$lib/services/raid-management';
  import { getGameClassDisplayName, getGameClassIconId } from '$lib/data/classes';

  import { listen } from '@tauri-apps/api/event';

  let activeTab: AppTab = 'dashboard';
  let sidebarOpen = false;
  let headerContent = '';
  let activeSettingsTab = 'roster';
  //let activeMeowConnectTab: MeowConnectSection = 'together';
  //let pendingMeowConnectRequests = 0;
  //let pendingMeowConnectFriendRequests: MeowConnectFriendConnection[] = [];
  //let pendingMeowConnectGroupInvites: MeowConnectGroup[] = [];
  //let meowConnectFriendRequestRefreshTimer: ReturnType<typeof setTimeout> | null = null;
  let nextResetTime = '';
  let resetCountdown = '';
  let showHeaderCountdown = true;
  let appReady = false;
  let showSetupGuideButton = true;
  let showAuthWelcome = true;
  let showHaalsHourglassReminder = true;
  let startWithLoaLogsEnabled = false;
  let loaLogsPathConfigured = false;
  let loaLogsReminderShown = false;
  let loaLogsReminderMessage = '';
  let haalsHourglassReminderCharacters: HaalsHourglassReminderCharacter[] = [];
  let discordAuthState: DiscordAuthState = 'checking';
  let discordAuthMessage = 'Checking Discord access...';
  let discordAuthUser = '';
  let discordAuthUserId = '';
  let raidManagementAccessUserId = '';
  let raidManagementAccessGranted = false;
  let raidManagementAccessLoading = false;
  let appInitializationStarted = false;
  // Temporarily disabled due to Supabase realtime message limits
  // let meowConnectHeaderState: MeowConnectHeaderState = 'inactive';
  // let meowConnectHeaderMessage = 'MeowConnect is inactive.';
  // let meowConnectHeaderLabel = 'Inactive';
  // let meowConnectFeatureEnabled = true;
  // let meowConnectRealtimeEnabled = true;
  // let meowConnectCompletionUploadTimer: ReturnType<typeof setTimeout> | null = null;
  // let meowConnectFriendHintRefreshTimer: ReturnType<typeof setTimeout> | null = null;
  // let unsubscribeMeowConnectRealtime: (() => void) | null = null;
  // const MEOWCONNECT_PENDING_IDLE_REFRESH_MS = 3 * 60 * 1000;
  const HAALS_HOURGLASS_DISMISS_KEY_PREFIX = 'haalsHourglassReminderDismissed';

  interface HaalsHourglassReminderCharacter {
    charId: number;
    name: string;
    className: string;
    iconId: string;
    itemLevel: number;
    combatPower: number;
  }
  // Handle URL parameters
  $: urlParams = new URLSearchParams($page.url.search);
  $: tabFromUrl = urlParams.get('tab') || 'dashboard';
  $: charFromUrl = urlParams.get('char');

  // Update active tab when URL changes
  $: if (tabFromUrl !== activeTab) {
    activeTab = isAppTab(tabFromUrl) ? tabFromUrl : 'dashboard';
    if (activeTab !== tabFromUrl) {
      goto('/?tab=dashboard');
    }
  }

  // Temporarily disabled due to Supabase realtime message limits
  // $: if (!meowConnectFeatureEnabled && activeTab === 'meow-connect') {
  //   switchTab('dashboard');
  // }

  $: raidManagementLocalAccess = hasRaidManagementAccess(discordAuthUserId);
  $: raidManagementVisible =
    discordAuthState === 'approved' && (raidManagementLocalAccess || raidManagementAccessGranted);
  $: if (discordAuthState === 'approved' && discordAuthUserId && raidManagementAccessUserId !== discordAuthUserId) {
    void refreshRaidManagementAccess(discordAuthUserId);
  }
  $: if (!raidManagementVisible && activeTab === 'raid-management') {
    switchTab('dashboard');
  }

  // Update active filter character when URL changes
  $: if (charFromUrl) {
    activeFilterCharId.set(parseInt(charFromUrl));
  }

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
    const handleHaalsHourglassReminderChanged = (event: Event) => {
      const customEvent = event as CustomEvent<boolean>;
      showHaalsHourglassReminder = customEvent.detail;
      if (!showHaalsHourglassReminder) {
        haalsHourglassReminderCharacters = [];
      } else {
        void showHaalsHourglassReminderIfNeeded();
      }
    };
    // Temporarily disabled due to Supabase realtime message limits
    // const handleMeowConnectConsentChanged = () => {
    //   refreshMeowConnectHeaderStatus();
    //   scheduleMeowConnectFriendRequestRefresh();
    // };
    // const handleMeowConnectFeatureChanged = () => {
    //   refreshMeowConnectFeatureSettings();
    //   scheduleMeowConnectFriendRequestRefresh();
    //   if (!meowConnectFeatureEnabled && activeTab === 'meow-connect') {
    //     switchTab('dashboard');
    //   }
    // };
    // const handleMeowConnectRealtimeChanged = () => {
    //   refreshMeowConnectFeatureSettings();
    // };
    // const handleMeowConnectFriendClearHintsChanged = () => {
    //   refreshMeowConnectFeatureSettings();
    // };
    // const handleMeowConnectCompletionChanged = () => {
    //   scheduleMeowConnectCompletionUpload();
    //   scheduleMeowConnectFriendRequestRefresh();
    // };
    const handleTodoTaskStatusChanged = () => {
      void showHaalsHourglassReminderIfNeeded();
    };
    // Temporarily disabled due to Supabase realtime message limits
    // const unsubscribeMeowConnectStatus = meowConnectStatus.subscribe((status) => {
    //   meowConnectHeaderState = status.state;
    //   meowConnectHeaderMessage = status.message;
    //   meowConnectHeaderLabel = getMeowConnectHeaderLabel(status.state);
    // });
    // let unlistenMeowConnectScrape: (() => void) | null = null;

    applyTheme();
    window.addEventListener('setup-guide-button:changed', handleSetupGuideButtonChanged);
    window.addEventListener('header-countdown:changed', handleHeaderCountdownChanged);
    window.addEventListener('haals-hourglass-reminder:changed', handleHaalsHourglassReminderChanged);
    // Temporarily disabled due to Supabase realtime message limits
    // window.addEventListener('meow-connect-consent-changed', handleMeowConnectConsentChanged);
    // window.addEventListener('meow-connect-feature-changed', handleMeowConnectFeatureChanged);
    // window.addEventListener('meow-connect-realtime-changed', handleMeowConnectRealtimeChanged);
    // window.addEventListener('meow-connect-friend-clear-hints-changed', handleMeowConnectFriendClearHintsChanged);
    // window.addEventListener('raid-completed', handleMeowConnectCompletionChanged);
    window.addEventListener('todo-task-status-changed', handleTodoTaskStatusChanged);
    cleanupLegacyBrowserStorage();
    refreshHeaderCountdownPreference();
    // refreshMeowConnectFeatureSettings();
    // refreshMeowConnectHeaderStatus();

    (async () => {
      // Temporarily disabled due to Supabase realtime message limits
      // unlistenMeowConnectScrape = await listen('meow-connect-roster-scrape-complete', () => {
      //   void syncMeowConnectAfterRosterScrape();
      // });
      await loadSystemPreferences();
      await checkStoredDiscordAuth();
      if (discordAuthState !== 'approved') {
        return;
      }
      await initializeAuthorizedApp();
      // startMeowConnectRealtimeHints();
    })();

    // Update countdown every second from cached reset timestamp.
    const countdownInterval = setInterval(updateResetCountdown, 1000);
    // Refresh backend reset timestamp only once per minute.
    const resetRefreshInterval = setInterval(refreshNextResetTimeFromBackend, 60000);
    const loaLogsStatusInterval = setInterval(clearLoaLogsReminderWhenRunning, 5000);
    // Temporarily disabled due to Supabase realtime message limits
    // const meowConnectFriendRequestInterval = setInterval(refreshMeowConnectFriendRequests, MEOWCONNECT_PENDING_IDLE_REFRESH_MS);

    // Cleanup on unmount
    return () => {
      // Temporarily disabled due to Supabase realtime message limits
      // unlistenMeowConnectScrape?.();
      // unsubscribeMeowConnectRealtime?.();
      // unsubscribeMeowConnectStatus();
      // if (meowConnectCompletionUploadTimer) clearTimeout(meowConnectCompletionUploadTimer);
      // if (meowConnectFriendRequestRefreshTimer) clearTimeout(meowConnectFriendRequestRefreshTimer);
      // if (meowConnectFriendHintRefreshTimer) clearTimeout(meowConnectFriendHintRefreshTimer);
      window.removeEventListener('setup-guide-button:changed', handleSetupGuideButtonChanged);
      window.removeEventListener('header-countdown:changed', handleHeaderCountdownChanged);
      window.removeEventListener('haals-hourglass-reminder:changed', handleHaalsHourglassReminderChanged);
      // window.removeEventListener('meow-connect-consent-changed', handleMeowConnectConsentChanged);
      // window.removeEventListener('meow-connect-feature-changed', handleMeowConnectFeatureChanged);
      // window.removeEventListener('meow-connect-realtime-changed', handleMeowConnectRealtimeChanged);
      // window.removeEventListener('meow-connect-friend-clear-hints-changed', handleMeowConnectFriendClearHintsChanged);
      // window.removeEventListener('raid-completed', handleMeowConnectCompletionChanged);
      window.removeEventListener('todo-task-status-changed', handleTodoTaskStatusChanged);
      clearInterval(countdownInterval);
      clearInterval(resetRefreshInterval);
      clearInterval(loaLogsStatusInterval);
      // clearInterval(meowConnectFriendRequestInterval);
    };
  });

  function refreshHeaderCountdownPreference() {
    showHeaderCountdown = getHeaderCountdownPreference();
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
      discordAuthUserId = '';
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
    discordAuthUserId = result.user_id ?? '';

    if (result.approved) {
      discordAuthState = showAuthWelcome ? 'welcome' : 'approved';
    } else if (!result.user_id) {
      discordAuthState = 'login';
    } else {
      discordAuthState = 'denied';
    }

    if (discordAuthState === 'welcome' || discordAuthState === 'denied') {
      reloadTenorEmbeds();
    }
  }

  function retryDiscordLogin() {
    discordAuthState = 'login';
    discordAuthMessage = 'Sign in with Discord to access LOA Tracker.';
    discordAuthUser = '';
    discordAuthUserId = '';
  }

  async function proceedFromWelcome() {
    discordAuthState = 'approved';
    await initializeAuthorizedApp();
  }

  async function initializeAuthorizedApp() {
    if (appInitializationStarted) {
      return;
    }
    appInitializationStarted = true;

    try {
      await initializeApp();
      await loadSystemPreferences();
      // Temporarily disabled due to Supabase realtime message limits
      // refreshMeowConnectHeaderStatus();
      appReady = true;
      await showLoaLogsReminderIfNeeded();
      checkForAppUpdates().catch((error) => console.warn('Update check failed:', error));

      // Update rested values immediately on app start
      try {
        await updateRestedValuesNow();
      } catch (restedError) {
        console.error('Failed to update rested values:', restedError);
      }

      // Check and ensure data completeness on app start
      try {
        await ensureCharacterDataComplete();
        window.dispatchEvent(new CustomEvent('character-data-complete'));

        // Update reset timestamps
        await updateResetTimestamps();

        // Initialize reset countdown from bootstrap snapshot
        nextResetTime = $nextDailyReset;
        await updateCountdownFromKnownReset();

        try {
          await syncEncountersToCompletions();
        } catch (syncError) {
          console.error('Encounters sync failed:', syncError);
        }

        await showHaalsHourglassReminderIfNeeded();
        //await syncMeowConnectOnAppStart();
      } catch (error) {
        console.error('Data completeness check failed:', error);
      }
    } catch (error) {
      appInitializationStarted = false;
      console.error('Failed to initialize authorized app:', error);
    }
  }

  // Temporarily disabled due to Supabase realtime message limits
  // async function syncMeowConnectOnAppStart() {
  //   if (!meowConnectFeatureEnabled || !hasMeowConnectConsent()) {
  //     refreshMeowConnectHeaderStatus();
  //     return;
  //   }
  //
  //   try {
  //     markMeowConnectConnecting('Checking MeowConnect startup sync.');
  //     const result = await uploadMeowConnectSnapshotIfNeeded();
  //     const appliedClearHints = await applyMeowConnectFriendClearHintsIfEnabled(result.snapshot.weeklyResetMs);
  //     if (appliedClearHints > 0) {
  //       await uploadMeowConnectSnapshotIfNeeded({ force: true });
  //       window.dispatchEvent(new CustomEvent('raid-completed'));
  //     }
  //     const statusMessage = appliedClearHints > 0
  //       ? `MeowConnect applied ${appliedClearHints} friend clear hint${appliedClearHints === 1 ? '' : 's'}.`
  //       : result.uploaded
  //         ? 'MeowConnect startup sync succeeded.'
  //         : 'MeowConnect is connected.';
  //     markMeowConnectActive(statusMessage);
  //     await refreshMeowConnectFriendRequests();
  //   } catch (error) {
  //     markMeowConnectFailure(error);
  //     console.warn('MeowConnect startup sync failed:', error);
  //     await refreshMeowConnectFriendRequests();
  //   }
  // }

  // Temporarily disabled due to Supabase realtime message limits
  // async function syncMeowConnectAfterRosterScrape() {
  //   if (!meowConnectFeatureEnabled || !hasMeowConnectConsent()) {
  //     refreshMeowConnectHeaderStatus();
  //     return;
  //   }
  //
  //   try {
  //     markMeowConnectConnecting('Syncing MeowConnect after roster scrape.');
  //     await uploadMeowConnectSnapshotIfNeeded({ force: true });
  //     markMeowConnectActive('MeowConnect roster scrape sync succeeded.');
  //     await refreshMeowConnectFriendRequests();
  //   } catch (error) {
  //     markMeowConnectFailure(error);
  //     console.warn('MeowConnect roster scrape sync failed:', error);
  //     await refreshMeowConnectFriendRequests();
  //   }
  // }

  async function refreshRaidManagementAccess(userId: string) {
    raidManagementAccessUserId = userId;
    raidManagementAccessLoading = true;
    try {
      raidManagementAccessGranted = await hasRaidManagementAccessRemote(userId);
    } catch (error) {
      raidManagementAccessGranted = false;
      console.warn('Failed to check remote raid management access:', error);
    } finally {
      raidManagementAccessLoading = false;
    }
  }

  function switchTab(tab: string) {
    // Temporarily disabled due to Supabase realtime message limits
    // if (tab === 'meow-connect' && !meowConnectFeatureEnabled) {
    //   tab = 'dashboard';
    // }
    if (tab === 'raid-management' && !raidManagementVisible) {
      tab = 'dashboard';
    }
    activeTab = isAppTab(tab) ? tab : 'dashboard';
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

  // Temporarily disabled due to Supabase realtime message limits
  // function openMeowConnectRequests() {
  //   activeMeowConnectTab = 'settings';
  //   switchTab('meow-connect');
  // }

  // function switchMeowConnectTab(tab: MeowConnectSection) {
  //   //activeMeowConnectTab = tab;
  // }

  // function handlePendingRequestsChanged(count: number) {
  //   pendingMeowConnectRequests = count;
  //   scheduleMeowConnectFriendRequestRefresh();
  // }

  // Temporarily disabled due to Supabase realtime message limits
  // function scheduleMeowConnectFriendRequestRefresh() {
  //   if (meowConnectFriendRequestRefreshTimer) clearTimeout(meowConnectFriendRequestRefreshTimer);
  //   meowConnectFriendRequestRefreshTimer = setTimeout(() => {
  //     meowConnectFriendRequestRefreshTimer = null;
  //     void refreshMeowConnectFriendRequests();
  //   }, 500);
  // }
  //
  // async function refreshMeowConnectFriendRequests() {
  //   if (!meowConnectFeatureEnabled || !hasMeowConnectConsent() || discordAuthState !== 'approved') {
  //     pendingMeowConnectFriendRequests = [];
  //     pendingMeowConnectGroupInvites = [];
  //     pendingMeowConnectRequests = 0;
  //     return;
  //   }
  //
  //   const startedAt = performance.now();
  //   logMeowConnectRequest('Header pending request refresh started.');
  //   try {
  //     const { friendRequests, groupInvites } = await loadMeowConnectPendingRequests();
  //     pendingMeowConnectFriendRequests = friendRequests;
  //     pendingMeowConnectGroupInvites = groupInvites;
  //     pendingMeowConnectRequests = pendingMeowConnectFriendRequests.length + pendingMeowConnectGroupInvites.length;
  //     logMeowConnectRequest(
  //       `Header pending request refresh finished in ${Math.round(performance.now() - startedAt)}ms: friendRequests=${friendRequests.length}, groupInvites=${groupInvites.length}, pending=${pendingMeowConnectRequests}.`,
  //       'info'
  //     );
  //   } catch (error) {
  //     logMeowConnectRequest(`Header pending request refresh failed: ${error}`, 'warn');
  //     console.warn('Failed to refresh MeowConnect friend request notifications:', error);
  //   }
  // }

  function switchSettingsTab(tab: string) {
    activeSettingsTab = tab;
  }

  function startSetupGuide() {
    window.dispatchEvent(new CustomEvent('setup-guide:start'));
  }

  async function loadSystemPreferences() {
    try {
      const settings = await loadAppSystemPreferences();
      showSetupGuideButton = settings.showSetupGuideButton;
      showAuthWelcome = settings.showAuthWelcome;
      showHaalsHourglassReminder = settings.showHaalsHourglassReminder;
      startWithLoaLogsEnabled = settings.startWithLoaLogs;
      loaLogsPathConfigured = settings.loaLogsPathConfigured;
    } catch (error) {
      console.warn('Failed to load system preferences:', error);
    }
  }

  async function showLoaLogsReminderIfNeeded() {
    if (!startWithLoaLogsEnabled || loaLogsReminderShown) {
      return;
    }

    try {
      loaLogsReminderMessage = await getLoaLogsReminderMessage(startWithLoaLogsEnabled, loaLogsPathConfigured);
      loaLogsReminderShown = Boolean(loaLogsReminderMessage);
    } catch (error) {
      console.warn('Failed to check LOA Logs reminder state:', error);
    }
  }

  async function showHaalsHourglassReminderIfNeeded() {
    if (!showHaalsHourglassReminder || !isTuesdayBeforeWeeklyReset() || isHaalsHourglassReminderDismissedToday()) {
      return;
    }

    try {
      const characters = await getHaalsHourglassReminderCharacters();
      haalsHourglassReminderCharacters = characters;
    } catch (error) {
      console.warn("Failed to check Haal's Hourglass reminder state:", error);
    }
  }

  function isTuesdayBeforeWeeklyReset() {
    return new Date().getDay() === 2;
  }

  function getTodayKey() {
    return new Date().toISOString().slice(0, 10);
  }

  function isHaalsHourglassReminderDismissedToday() {
    return localStorage.getItem(`${HAALS_HOURGLASS_DISMISS_KEY_PREFIX}:${getTodayKey()}`) === '1';
  }

  async function getHaalsHourglassReminderCharacters() {
    const pendingCharacters = new Map<number, HaalsHourglassReminderCharacter>();

    for (const roster of $rosters) {
      const snapshot = await loadDashboardSnapshot(roster.id);

      for (const character of snapshot.characters || []) {
        const itemLevel = Number((character as any).item_level ?? (character as any).ilvl ?? 0);
        if (itemLevel < 1730) continue;

        const key = String(character.char_id);
        const trackingStatus = snapshot.tracking_by_character?.[key] || [];
        const completionStatus = snapshot.completion_by_character?.[key] || [];
        const cubeTracked = trackingStatus.some((entry) => entry.content_id === 'cube' && Number(entry.is_tracked) === 1);
        const cubeCompleted = completionStatus.some((entry) => entry.content_id === 'cube' && Number(entry.is_completed) === 1);

        if (cubeTracked && !cubeCompleted) {
          const classId = character.class_id || '';
          pendingCharacters.set(character.char_id, {
            charId: character.char_id,
            name: character.char_name,
            className: getGameClassDisplayName(classId),
            iconId: character.icon_id || getGameClassIconId(classId),
            itemLevel,
            combatPower: Number(character.combat_power ?? 0)
          });
        }
      }
    }

    return [...pendingCharacters.values()].sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }));
  }

  function dismissHaalsHourglassReminder() {
    localStorage.setItem(`${HAALS_HOURGLASS_DISMISS_KEY_PREFIX}:${getTodayKey()}`, '1');
    haalsHourglassReminderCharacters = [];
  }

  function openHaalsHourglassTodo() {
    haalsHourglassReminderCharacters = [];
    switchTab('todo');
  }

  async function clearLoaLogsReminderWhenRunning() {
    if (!loaLogsReminderMessage) {
      return;
    }

    try {
      if (await isLoaLogsRunning()) {
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

  // Temporarily disabled due to Supabase realtime message limits
  // function refreshMeowConnectHeaderStatus() {
  //   if (!meowConnectFeatureEnabled || !hasMeowConnectConsent()) {
  //     meowConnectStatus.set({
  //       state: 'inactive',
  //       message: meowConnectFeatureEnabled ? 'MeowConnect is inactive.' : 'MeowConnect is disabled.',
  //       updatedAt: Date.now()
  //     });
  //   }
  // }
  //
  // function refreshMeowConnectFeatureSettings() {
  //   meowConnectFeatureEnabled = isMeowConnectFeatureEnabled();
  //   meowConnectRealtimeEnabled = isMeowConnectRealtimeEnabled();
  //   refreshMeowConnectHeaderStatus();
  //   if (!meowConnectFeatureEnabled || !hasMeowConnectConsent() || !isMeowConnectFriendClearHintsEnabled()) {
  //     stopMeowConnectRealtimeHints();
  //   } else {
  //     startMeowConnectRealtimeHints();
  //   }
  // }
  //
  // function startMeowConnectRealtimeHints() {
  //   if (unsubscribeMeowConnectRealtime) return;
  //   if (!meowConnectFeatureEnabled || !hasMeowConnectConsent() || !isMeowConnectFriendClearHintsEnabled()) return;
  //
  //   unsubscribeMeowConnectRealtime = subscribeMeowConnectChanges(
  //     () => {
  //       if (meowConnectFriendHintRefreshTimer) clearTimeout(meowConnectFriendHintRefreshTimer);
  //       meowConnectFriendHintRefreshTimer = setTimeout(() => {
  //         meowConnectFriendHintRefreshTimer = null;
  //         void applyMeowConnectFriendClearHintsFromRealtime();
  //       }, 1500);
  //     },
  //     {
  //       ignoreRealtimePreference: true,
  //       connectedMessage: meowConnectRealtimeEnabled
  //         ? 'MeowConnect realtime is connected.'
  //         : 'MeowConnect friend clear hints are listening.'
  //     }
  //   );
  // }
  //
  // function stopMeowConnectRealtimeHints() {
  //   unsubscribeMeowConnectRealtime?.();
  //   unsubscribeMeowConnectRealtime = null;
  //   if (meowConnectFriendHintRefreshTimer) clearTimeout(meowConnectFriendHintRefreshTimer);
  //   meowConnectFriendHintRefreshTimer = null;
  // }
  //
  // function scheduleMeowConnectCompletionUpload() {
  //   if (!meowConnectFeatureEnabled || !meowConnectRealtimeEnabled || !hasMeowConnectConsent()) return;
  //   if (meowConnectCompletionUploadTimer) clearTimeout(meowConnectCompletionUploadTimer);
  //   meowConnectCompletionUploadTimer = setTimeout(() => {
  //     void syncMeowConnectAfterCompletionChange();
  //   }, 1200);
  // }
  //
  // async function syncMeowConnectAfterCompletionChange() {
  //   if (!meowConnectFeatureEnabled || !meowConnectRealtimeEnabled || !hasMeowConnectConsent()) return;
  //
  //   try {
  //     markMeowConnectConnecting('Syncing MeowConnect completion update.');
  //     const result = await uploadMeowConnectSnapshotIfNeeded({ force: true });
  //     const appliedClearHints = await applyMeowConnectFriendClearHintsIfEnabled(result.snapshot.weeklyResetMs);
  //     if (appliedClearHints > 0) {
  //       await uploadMeowConnectSnapshotIfNeeded({ force: true });
  //       window.dispatchEvent(new CustomEvent('raid-completed'));
  //     }
  //     const statusMessage = appliedClearHints > 0
  //       ? `MeowConnect applied ${appliedClearHints} friend clear hint${appliedClearHints === 1 ? '' : 's'}.`
  //       : result.uploaded
  //         ? 'MeowConnect completion update synced.'
  //         : 'MeowConnect completion update checked.';
  //     //markMeowConnectActive(statusMessage);
  //   } catch (error) {
  //     //markMeowConnectFailure(error);
  //     console.warn('MeowConnect completion sync failed:', error);
  //   }
  // }
  //
  // async function applyMeowConnectFriendClearHintsIfEnabled(weeklyResetMs?: number): Promise<number> {
  //   if (!isMeowConnectFriendClearHintsEnabled()) return 0;
  //
  //   const localSnapshot = await loadMeowConnectLocalSnapshot();
  //   const resetCycle = String(weeklyResetMs || localSnapshot.weeklyResetMs || 0);
  //   const remoteSnapshots = await fetchMeowConnectRemoteSnapshots(resetCycle);
  //   return applyFriendClearHintsToLocalSnapshot(localSnapshot, remoteSnapshots);
  // }
  //
  // async function applyMeowConnectFriendClearHintsFromRealtime() {
  //   if (!meowConnectFeatureEnabled || !hasMeowConnectConsent() || !isMeowConnectFriendClearHintsEnabled()) return;
  //
  //   try {
  //     const appliedClearHints = await applyMeowConnectFriendClearHintsIfEnabled();
  //     if (appliedClearHints <= 0) return;
  //
  //     await uploadMeowConnectSnapshotIfNeeded({ force: true });
  //     window.dispatchEvent(new CustomEvent('raid-completed'));
  //     markMeowConnectActive(`MeowConnect applied ${appliedClearHints} friend clear hint${appliedClearHints === 1 ? '' : 's'}.`);
  //   } catch (error) {
  //     markMeowConnectFailure(error);
  //     console.warn('MeowConnect friend clear hint refresh failed:', error);
  //   }
  // }

  function toggleSidebar() {
    sidebarOpen = !sidebarOpen;
  }

  // Update reset countdown
  function updateResetCountdown() {
    resetCountdown = formatResetCountdown(nextResetTime);
  }

  async function refreshNextResetTimeFromBackend() {
    try {
      nextResetTime = await getNextDailyResetTime();
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

{#if discordAuthState !== 'approved'}
  <AppAuthScreen
    {discordAuthState}
    {discordAuthUser}
    {discordAuthMessage}
    {loginWithDiscord}
    {proceedFromWelcome}
    {retryDiscordLogin}
  />
{:else}
<div class="app">
  <!-- Sidebar -->
  <Sidebar
    {activeTab}
    {switchTab}
    isOpen={sidebarOpen}
    {discordAuthUser}
    showMeowConnect={false}
    showRaidManagement={raidManagementVisible}
  />

  <!-- Overlay for mobile -->
  {#if sidebarOpen}
    <div class="overlay" role="button" tabindex="0" on:click={toggleSidebar} on:keydown={(e) => e.key === 'Enter' && toggleSidebar()} aria-label="Close sidebar"></div>
  {/if}

  <!-- Main Content -->
  <div class="main-content">
    <AppHeader
      {activeTab}
      bind:activeSettingsTab
      //bind:activeMeowConnectTab
      //{meowConnectFeatureEnabled}
      {showHeaderCountdown}
      {resetCountdown}
      //{meowConnectHeaderState}
      //{meowConnectHeaderMessage}
      //{meowConnectHeaderLabel}
      //{pendingMeowConnectRequests}
      //{pendingMeowConnectFriendRequests}
      {showSetupGuideButton}
      {headerContent}
      {loaLogsReminderMessage}
      updateAvailable={$updateAvailable}
      latestAppVersion={$latestAppVersion}
      currentAppVersion={$currentAppVersion}
      isUpdateChecking={$isUpdateChecking}
      {toggleSidebar}
      {switchTab}
      //{openMeowConnectRequests}
      {startSetupGuide}
      {dismissLoaLogsReminder}
      {checkForAppUpdates}
    />

    <AppContent
      {activeTab}
      bind:activeSettingsTab
      //{activeMeowConnectTab}
      //{meowConnectFeatureEnabled}
      {raidManagementVisible}
      {discordAuthUserId}
      {discordAuthUser}
      highlightCharId={$activeFilterCharId}
      {setHeaderContent}
      //{handlePendingRequestsChanged}
    />
  </div>

  <SetupGuide
    {activeTab}
    {activeSettingsTab}
    //{activeMeowConnectTab}
    {appReady}
    characterCount={$characters.length}
    {switchTab}
    setSettingsTab={switchSettingsTab}
    //setMeowConnectTab={switchMeowConnectTab}
  />

  {#if haalsHourglassReminderCharacters.length > 0}
    <div class="haals-reminder-overlay" role="dialog" aria-modal="true" aria-labelledby="haals-reminder-title">
      <section class="haals-reminder-card">
        <div>
          <p class="haals-reminder-kicker">Weekly reset reminder</p>
          <h2 id="haals-reminder-title">Haal's Hourglass is still open</h2>
          <p>
            The following 1730+ character{haalsHourglassReminderCharacters.length === 1 ? ' has' : 's have'} Cube tracked but not completed.
            Haal's Hourglass should be done before weekly reset.
          </p>
        </div>

        <ul>
          {#each haalsHourglassReminderCharacters as character}
            <li>
              <img src={classAsset(character.iconId)} alt="" />
              <div class="haals-character-main">
                <strong>{character.name}</strong>
                <span>{character.className}</span>
              </div>
              <div class="haals-character-stats">
                <span>iLvl {character.itemLevel.toLocaleString()}</span>
                <span>CP {character.combatPower.toLocaleString()}</span>
              </div>
            </li>
          {/each}
        </ul>

        <div class="haals-reminder-actions">
          <button type="button" class="secondary" on:click={openHaalsHourglassTodo}>Open To Do</button>
          <button type="button" on:click={dismissHaalsHourglassReminder}>Dismiss today</button>
        </div>
      </section>
    </div>
  {/if}
</div>
{/if}

<style>
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
    background: var(--app-color-overlay);
    z-index: 101;
    backdrop-filter: blur(2px);
  }

  .haals-reminder-overlay {
    position: fixed;
    inset: 0;
    z-index: 1100;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: var(--app-color-modal-backdrop);
  }

  .haals-reminder-card {
    width: min(460px, 100%);
    display: grid;
    gap: 0.9rem;
    padding: 1rem;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-warning) 42%, var(--md-sys-color-outline));
    border-radius: 12px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    box-shadow: var(--app-shadow-md);
  }

  .haals-reminder-kicker {
    margin: 0 0 0.25rem;
    color: var(--md-sys-color-warning);
    font-size: 0.72rem;
    font-weight: 800;
    text-transform: uppercase;
  }

  .haals-reminder-card h2,
  .haals-reminder-card p {
    margin: 0;
  }

  .haals-reminder-card h2 {
    margin-bottom: 0.4rem;
    font-size: 1.15rem;
  }

  .haals-reminder-card p {
    color: var(--md-sys-color-on-surface-variant);
    line-height: 1.45;
  }

  .haals-reminder-card ul {
    display: grid;
    gap: 0.35rem;
    max-height: 12rem;
    margin: 0;
    padding: 0;
    list-style: none;
    overflow: auto;
  }

  .haals-reminder-card li {
    display: grid;
    grid-template-columns: 2.25rem minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.55rem;
    padding: 0.48rem 0.55rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 7px;
    background: var(--md-sys-color-surface-container);
  }

  .haals-reminder-card li img {
    width: 2.25rem;
    height: 2.25rem;
    object-fit: contain;
  }

  .haals-character-main {
    min-width: 0;
    display: grid;
    gap: 0.1rem;
  }

  .haals-character-main strong {
    min-width: 0;
    overflow: hidden;
    color: var(--md-sys-color-on-surface);
    font-size: 0.88rem;
    font-weight: 650;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .haals-character-main span,
  .haals-character-stats span {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
    font-weight: 500;
  }

  .haals-character-stats {
    display: grid;
    gap: 0.1rem;
    text-align: right;
    white-space: nowrap;
  }

  .haals-reminder-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .haals-reminder-actions button {
    border: 0;
    border-radius: 8px;
    padding: 0.58rem 0.85rem;
    background: var(--md-sys-color-warning);
    color: var(--app-color-on-warning, #1d1200);
    cursor: pointer;
    font-size: 0.82rem;
    font-weight: 600;
    letter-spacing: 0;
  }

  .haals-reminder-actions button.secondary {
    border: 1px solid var(--md-sys-color-outline);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

</style>
