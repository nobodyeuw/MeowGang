<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { activeRosterId, characters, loadCharacters, loadRosters, rosters } from '$lib/store';
  import {
    getHeaderCountdownPreference,
    setHeaderCountdownPreference
  } from '$lib/services/app-preferences';
  import { clearActiveRosterPreference } from '$lib/services/roster-preferences';
  import {
    isMeowConnectFeatureEnabled,
    isMeowConnectFriendClearHintsEnabled,
    isMeowConnectRealtimeEnabled,
    setMeowConnectFeatureEnabled,
    setMeowConnectFriendClearHintsEnabled,
    setMeowConnectRealtimeEnabled
  } from '$lib/services/meow-connect';
  import {
    getDashboardStaticBadgesPreference,
    getDashboardViewPreference,
    setDashboardStaticBadgesPreference,
    setDashboardViewPreference,
    type DashboardViewMode
  } from '$lib/services/dashboard-preferences';
  import { setSplitRatTodoView, splitRatTodoView } from '$lib/services/todo-preferences';
  import {
    getThemeLabel,
    getThemePreference,
    setThemePreference
  } from '$lib/services/theme-preferences';
  import type { AppThemeId } from '$lib/data/themes';
  import {
    clearLogCommand,
    clearUserDataCommand,
    getLogContentCommand,
    isLoaLogsRunningCommand,
    isLostArkRunningCommand,
    loadSystemSettingsCommand,
    sendLogReportCommand,
    setEncountersDbPathCommand,
    setHideOnLaunchCommand,
    setShowHaalsHourglassReminderCommand,
    setLoaLogsExePathCommand,
    setLostArkExePathCommand,
    setShowAuthWelcomeCommand,
    setShowSetupGuideButtonCommand,
    setStartWithLoaLogsCommand,
    setStartWithLostArkCommand,
    setStartWithWindowsCommand,
    type SystemSettingsPayload
  } from '$lib/services/system-settings';
  import SystemFilePathsSection from '$lib/components/settings/system-settings/SystemFilePathsSection.svelte';
  import SystemGeneralSection from '$lib/components/settings/system-settings/SystemGeneralSection.svelte';
  import SystemLoggingSection from '$lib/components/settings/system-settings/SystemLoggingSection.svelte';
  import SystemMeowConnectSection from '$lib/components/settings/system-settings/SystemMeowConnectSection.svelte';
  import SystemStartupSection from '$lib/components/settings/system-settings/SystemStartupSection.svelte';

  // State
  let systemSettings: SystemSettingsPayload | null = null;
  let isLoading = true;
  let error = '';
  let successMessage = '';
  let showSuccessMessage = false;

  // System settings
  let encountersDbPath = '';
  let lostArkExePath = '';
  let loaLogsExePath = '';
  let startWithWindows = false;
  let startWithLostArk = false;
  let startWithLoaLogs = false;
  let hideOnLaunch = false;
  let showSetupGuideButton = true;
  let showAuthWelcome = true;
  let showHaalsHourglassReminder = true;
  let showHeaderCountdown = true;
  let showDashboardStaticBadges = true;
  let meowConnectEnabled = true;
  let meowConnectRealtimeEnabled = true;
  let meowConnectFriendClearHintsEnabled = false;
  let dashboardView: DashboardViewMode = 'compact';
  let appTheme: AppThemeId = 'outlaw';
  let isClearingUserData = false;
  let showClearUserDataDialog = false;
  let isRunning = false;
  let isLoaLogsRunning = false;
  let loaLogsReminderMessage = '';

  // Logging
  let logContent: string | null = null;
  let isLogLoading = false;
  let showLogDialog = false;

  // Load system settings on mount
  onMount(() => {
    let statusInterval: ReturnType<typeof setInterval> | undefined;

    (async () => {
      await loadSystemSettings();
      await checkLostArkStatus();

      // Check Lost Ark status periodically
      statusInterval = setInterval(async () => {
        await checkLostArkStatus();
      }, 5000); // Check every 5 seconds
    })();

    return () => {
      if (statusInterval) {
        clearInterval(statusInterval);
      }
    };
  });

  async function loadSystemSettings() {
    try {
      isLoading = true;
      error = '';

      const settings = await loadSystemSettingsCommand();
      systemSettings = settings;

      // Update local state (use camelCase from backend serialization)
      encountersDbPath = settings.encountersDbPath || settings.encounters_db_path || '';
      lostArkExePath = settings.lostArkExePath || settings.lost_ark_exe_path || '';
      loaLogsExePath = settings.loaLogsExePath || settings.loa_logs_exe_path || '';
      startWithWindows = settings.startWithWindows || settings.start_with_windows || false;
      startWithLostArk = settings.startWithLostArk || settings.start_with_lost_ark || false;
      startWithLoaLogs = settings.startWithLoaLogs || settings.start_with_loa_logs || false;
      hideOnLaunch = settings.hideOnLaunch || settings.hide_on_launch || false;
      showSetupGuideButton = settings.showSetupGuideButton ?? settings.show_setup_guide_button ?? true;
      showAuthWelcome = settings.showAuthWelcome ?? settings.show_auth_welcome ?? true;
      showHaalsHourglassReminder = settings.showHaalsHourglassReminder ?? settings.show_haals_hourglass_reminder ?? true;
      showHeaderCountdown = getHeaderCountdownPreference();
      showDashboardStaticBadges = getDashboardStaticBadgesPreference();
      meowConnectEnabled = isMeowConnectFeatureEnabled();
      meowConnectRealtimeEnabled = isMeowConnectRealtimeEnabled();
      meowConnectFriendClearHintsEnabled = isMeowConnectFriendClearHintsEnabled();
      dashboardView = getDashboardViewPreference();
      appTheme = getThemePreference();

    } catch (err) {
      error = `Failed to load system settings: ${err}`;
      console.error(error);
    } finally {
      isLoading = false;
    }
  }

  async function browseEncountersDb() {
    try {
      const selected = await open({
        title: 'Select encounters.db file',
        filters: [{
          name: 'Database Files',
          extensions: ['db']
        }],
        multiple: false,
      });

      if (selected && typeof selected === 'string') {
        await setEncountersDbPath(selected);
      }
    } catch (err) {
      showError(`Failed to browse for encounters.db: ${err}`);
    }
  }

  async function setEncountersDbPath(path: string) {
    try {
      await setEncountersDbPathCommand(path);
      encountersDbPath = path;
      showSuccess('encounters.db path updated successfully!');
    } catch (err) {
      showError(`Failed to set encounters.db path: ${err}`);
    }
  }

  // Browse for LostArk.exe
  async function browseLostArkExe() {
    try {
      const selected = await open({
        title: 'Select LostArk.exe',
        filters: [{
          name: 'Executable Files',
          extensions: ['exe']
        }],
        multiple: false,
      });

      if (selected && typeof selected === 'string') {
        await setLostArkExePath(selected);
      }
    } catch (err) {
      showError(`Failed to browse for LostArk.exe: ${err}`);
    }
  }

  // Browse for LOA Logs exe
  async function browseLoaLogsExe() {
    try {
      const selected = await open({
        title: 'Select LOA Logs executable',
        filters: [{ name: 'Executable Files', extensions: ['exe'] }],
        multiple: false,
      });

      if (selected && typeof selected === 'string') {
        await setLoaLogsExePath(selected);
      }
    } catch (err) {
      showError(`Failed to browse for LOA Logs exe: ${err}`);
    }
  }

  async function setLoaLogsExePath(path: string) {
    try {
      await setLoaLogsExePathCommand(path);
      loaLogsExePath = path;
      showSuccess('LOA Logs executable path updated successfully!');
    } catch (err) {
      showError(`Failed to set LOA Logs executable path: ${err}`);
    }
  }

  async function setLostArkExePath(path: string) {
    try {
      await setLostArkExePathCommand(path);
      lostArkExePath = path;
      showSuccess('Lost Ark executable path updated successfully!');
    } catch (err) {
      showError(`Failed to set Lost Ark executable path: ${err}`);
    }
  }

  // Toggle system settings
  async function toggleStartWithWindows() {
    try {
      const newValue = !startWithWindows;
      await setStartWithWindowsCommand(newValue);
      startWithWindows = newValue;
      showSuccess(`Start with Windows ${newValue ? 'enabled' : 'disabled'}!`);
    } catch (err) {
      showError(`Failed to toggle start with Windows: ${err}`);
    }
  }

  async function toggleStartWithLostArk() {
    try {
      const newValue = !startWithLostArk;
      await setStartWithLostArkCommand(newValue);
      startWithLostArk = newValue;
      showSuccess(`Start with Lost Ark ${newValue ? 'enabled' : 'disabled'}!`);
    } catch (err) {
      showError(`Failed to toggle start with Lost Ark: ${err}`);
    }
  }

  async function toggleStartWithLoaLogs() {
    try {
      const newValue = !startWithLoaLogs;
      await setStartWithLoaLogsCommand(newValue);
      startWithLoaLogs = newValue;
      if (newValue) {
        showSuccess('LOA Logs monitoring enabled. Please start LOA Logs manually for now.');
        if (!isLoaLogsRunning) {
          loaLogsReminderMessage = loaLogsExePath
            ? 'Do not forget to start LOA Logs.exe for maximum efficiency.'
            : 'For better QoL you should install LOA Logs.exe or set the path manually in Settings.';
        }
      } else {
        loaLogsReminderMessage = '';
        showSuccess('LOA Logs monitoring disabled!');
      }
    } catch (err) {
      showError(`Failed to toggle start with LOA Logs: ${err}`);
    }
  }

  async function toggleSetupGuideButton() {
    try {
      const newValue = !showSetupGuideButton;
      await setShowSetupGuideButtonCommand(newValue);
      showSetupGuideButton = newValue;
      window.dispatchEvent(new CustomEvent('setup-guide-button:changed', { detail: newValue }));
      showSuccess(`Set-Up Guide button ${newValue ? 'shown' : 'hidden'}!`);
    } catch (err) {
      showError(`Failed to update Set-Up Guide button: ${err}`);
    }
  }

  async function toggleAuthWelcome() {
    try {
      const newValue = !showAuthWelcome;
      await setShowAuthWelcomeCommand(newValue);
      showAuthWelcome = newValue;
      showSuccess(`Welcome screen ${newValue ? 'enabled' : 'disabled'}!`);
    } catch (err) {
      showError(`Failed to update welcome screen: ${err}`);
    }
  }

  async function toggleHaalsHourglassReminder() {
    try {
      const newValue = !showHaalsHourglassReminder;
      await setShowHaalsHourglassReminderCommand(newValue);
      showHaalsHourglassReminder = newValue;
      window.dispatchEvent(new CustomEvent('haals-hourglass-reminder:changed', { detail: newValue }));
      showSuccess(`Haal's Hourglass reminder ${newValue ? 'enabled' : 'disabled'}!`);
    } catch (err) {
      showError(`Failed to update Haal's Hourglass reminder: ${err}`);
    }
  }

  async function toggleHideOnLaunch() {
    try {
      const newValue = !hideOnLaunch;
      await setHideOnLaunchCommand(newValue);
      hideOnLaunch = newValue;
      showSuccess(`Hide on launch ${newValue ? 'enabled' : 'disabled'}!`);
    } catch (err) {
      showError(`Failed to update hide on launch: ${err}`);
    }
  }

  function toggleHeaderCountdown() {
    const newValue = !showHeaderCountdown;
    showHeaderCountdown = newValue;
    setHeaderCountdownPreference(newValue);
    showSuccess(`Header countdown ${newValue ? 'shown' : 'hidden'}!`);
  }

  function toggleDashboardStaticBadges() {
    const newValue = !showDashboardStaticBadges;
    showDashboardStaticBadges = newValue;
    setDashboardStaticBadgesPreference(newValue);
    showSuccess(`Dashboard static badges ${newValue ? 'shown' : 'hidden'}!`);
  }

  function toggleMeowConnectEnabled() {
    const newValue = !meowConnectEnabled;
    meowConnectEnabled = newValue;
    setMeowConnectFeatureEnabled(newValue);
    showSuccess(`MeowConnect ${newValue ? 'enabled' : 'disabled'}!`);
  }

  function toggleMeowConnectRealtime() {
    const newValue = !meowConnectRealtimeEnabled;
    meowConnectRealtimeEnabled = newValue;
    setMeowConnectRealtimeEnabled(newValue);
    showSuccess(`MeowConnect real-time sync ${newValue ? 'enabled' : 'disabled'}!`);
  }

  function toggleMeowConnectFriendClearHints() {
    const newValue = !meowConnectFriendClearHintsEnabled;
    meowConnectFriendClearHintsEnabled = newValue;
    setMeowConnectFriendClearHintsEnabled(newValue);
    showSuccess(`MeowConnect clear hints ${newValue ? 'enabled' : 'disabled'}!`);
  }

  function toggleSplitRatTodoView() {
    const newValue = !$splitRatTodoView;
    setSplitRatTodoView(newValue);
    showSuccess(`RAT To Do view ${newValue ? 'enabled' : 'disabled'}!`);
  }

  function setDashboardView(view: DashboardViewMode) {
    dashboardView = view;
    setDashboardViewPreference(view);
    showSuccess(`Dashboard view set to ${view === 'compact' ? 'List' : 'Cards'}!`);
  }

  function setAppTheme(theme: AppThemeId) {
    appTheme = theme;
    setThemePreference(theme);
    showSuccess(`Theme set to ${getThemeLabel(theme)}!`);
  }

  function requestClearUserData() {
    showClearUserDataDialog = true;
  }

  async function confirmClearUserData() {
    try {
      isClearingUserData = true;
      const result = await clearUserDataCommand();
      showClearUserDataDialog = false;
      clearActiveRosterPreference();
      activeRosterId.set('');
      rosters.set([]);
      characters.set([]);
      await loadRosters();
      await loadCharacters('');
      showSuccess(result);
    } catch (err) {
      showError(`Failed to clear user data: ${err}`);
    } finally {
      isClearingUserData = false;
    }
  }

  // Check if Lost Ark is running
  async function checkLostArkStatus() {
    try {
      isRunning = await isLostArkRunningCommand();
      try {
        isLoaLogsRunning = await isLoaLogsRunningCommand();
        if (isLoaLogsRunning) {
          loaLogsReminderMessage = '';
        }
      } catch (_) {
        isLoaLogsRunning = false;
      }
    } catch (err) {
      console.warn('Failed to check Lost Ark status:', err);
    }
  }

  // Load app version from backend
  // Logging functions
  async function loadLogContent() {
    try {
      isLogLoading = true;
      logContent = await getLogContentCommand();
    } catch (err) {
      showError(`Failed to load log content: ${err}`);
    } finally {
      isLogLoading = false;
    }
  }

  async function clearLog() {
    try {
      await clearLogCommand();
      logContent = null;
      showSuccess('Log cleared successfully!');
    } catch (err) {
      showError(`Failed to clear log: ${err}`);
    }
  }

  async function sendLogReport() {
    try {
      const result = await sendLogReportCommand();
      showSuccess(result);
    } catch (err) {
      showError(`Failed to send log report: ${err}`);
    }
  }

  function openLogDialog() {
    loadLogContent();
    showLogDialog = true;
  }

  // UI helpers
  function showSuccess(message: string) {
    successMessage = message;
    showSuccessMessage = true;
    setTimeout(() => {
      showSuccessMessage = false;
      successMessage = '';
    }, 3000);
  }

  function showError(message: string) {
    error = message;
    setTimeout(() => {
      error = '';
    }, 5000);
  }

  function dismissLoaLogsReminder() {
    loaLogsReminderMessage = '';
  }

</script>

<div class="system-settings">
  <!-- Header -->
  <div class="settings-header">
    <div class="header-content">
      <div class="header-icon">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 2L2 7v10c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V7l-10-5z"/>
        </svg>
      </div>
      <div>
        <h2>System Settings</h2>
        <p>Configure application paths, startup behavior, and system preferences</p>
      </div>
    </div>
  </div>

  {#if isLoading}
    <div class="loading-state">
      <div class="loading-spinner"></div>
      <p>Loading system settings...</p>
    </div>
  {:else if error}
    <div class="error-state">
      <div class="error-icon">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/>
          <line x1="12" y1="8" x2="12" y2="12"/>
          <line x1="12" y1="16" x2="12.01" y2="16"/>
        </svg>
      </div>
      <p>{error}</p>
      <button class="retry-button" on:click={loadSystemSettings}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M23 4v6h-6"/>
          <path d="M1 20v-6h6"/>
          <path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15"/>
        </svg>
        Retry
      </button>
    </div>
  {:else}
    <div class="settings-content">
      {#if loaLogsReminderMessage}
        <div class="loa-logs-reminder">
          <div>
            <strong>LOA Logs:</strong> {loaLogsReminderMessage}
          </div>
          <button type="button" on:click={dismissLoaLogsReminder}>Dismiss</button>
        </div>
      {/if}

      <SystemGeneralSection
        {showSetupGuideButton}
        {showDashboardStaticBadges}
        {showAuthWelcome}
        {showHaalsHourglassReminder}
        {showHeaderCountdown}
        splitRatTodoView={$splitRatTodoView}
        {dashboardView}
        {appTheme}
        {isClearingUserData}
        onToggleSetupGuideButton={toggleSetupGuideButton}
        onToggleDashboardStaticBadges={toggleDashboardStaticBadges}
        onToggleAuthWelcome={toggleAuthWelcome}
        onToggleHaalsHourglassReminder={toggleHaalsHourglassReminder}
        onToggleHeaderCountdown={toggleHeaderCountdown}
        onToggleSplitRatTodoView={toggleSplitRatTodoView}
        onSetDashboardView={setDashboardView}
        onSetAppTheme={setAppTheme}
        onRequestClearUserData={requestClearUserData}
      />

      <SystemMeowConnectSection
        {meowConnectEnabled}
        {meowConnectRealtimeEnabled}
        {meowConnectFriendClearHintsEnabled}
        onToggleMeowConnectEnabled={toggleMeowConnectEnabled}
        onToggleMeowConnectRealtime={toggleMeowConnectRealtime}
        onToggleMeowConnectFriendClearHints={toggleMeowConnectFriendClearHints}
      />

      <SystemFilePathsSection
        {encountersDbPath}
        {lostArkExePath}
        {loaLogsExePath}
        onBrowseEncountersDb={browseEncountersDb}
        onBrowseLostArkExe={browseLostArkExe}
        onBrowseLoaLogsExe={browseLoaLogsExe}
      />

      <SystemStartupSection
        {startWithWindows}
        {hideOnLaunch}
        {startWithLostArk}
        {startWithLoaLogs}
        {isRunning}
        {isLoaLogsRunning}
        onToggleStartWithWindows={toggleStartWithWindows}
        onToggleHideOnLaunch={toggleHideOnLaunch}
        onToggleStartWithLostArk={toggleStartWithLostArk}
        onToggleStartWithLoaLogs={toggleStartWithLoaLogs}
      />

      <SystemLoggingSection
        onOpenLogs={openLogDialog}
        onSendReport={sendLogReport}
        onClearLogs={clearLog}
      />
    </div>
  {/if}

  <!-- Clear User Data Dialog -->
  {#if showClearUserDataDialog}
    <div
      class="dialog-overlay"
      role="presentation"
      on:click={() => !isClearingUserData && (showClearUserDataDialog = false)}
      on:keydown={(event) => event.key === 'Escape' && !isClearingUserData && (showClearUserDataDialog = false)}
    >
      <div
        class="confirm-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="clear-user-data-title"
        tabindex="-1"
        on:click|stopPropagation
        on:keydown={(event) => event.key === 'Escape' && !isClearingUserData && (showClearUserDataDialog = false)}
      >
        <div class="dialog-header">
          <h3 id="clear-user-data-title">Clear User Data</h3>
          <button
            class="dialog-close"
            aria-label="Close clear user data dialog"
            disabled={isClearingUserData}
            on:click={() => showClearUserDataDialog = false}
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>

        <div class="dialog-content confirm-content">
          <div class="warning-icon">
            <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
              <line x1="12" y1="9" x2="12" y2="13"/>
              <line x1="12" y1="17" x2="12.01" y2="17"/>
            </svg>
          </div>
          <div>
            <p class="confirm-title">You are about to delete all character information.</p>
            <p>
              This removes every character from userlogs.db together with tracking, raid, rested,
              gold, and progression data connected to those characters. This action cannot be undone.
            </p>
          </div>
        </div>

        <div class="dialog-actions">
          <button
            class="dialog-button secondary"
            disabled={isClearingUserData}
            on:click={() => showClearUserDataDialog = false}
          >
            Cancel
          </button>
          <button class="dialog-button danger" disabled={isClearingUserData} on:click={confirmClearUserData}>
            {isClearingUserData ? 'Deleting...' : 'Delete All Character Data'}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Log Dialog -->
  {#if showLogDialog}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="dialog-overlay" role="presentation" on:click={() => showLogDialog = false}>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div class="log-dialog" role="dialog" aria-modal="true" aria-label="Application logs" tabindex="-1" on:click|stopPropagation>
        <div class="dialog-header">
          <h3>Application Logs</h3>
          <button class="dialog-close" aria-label="Close application logs" on:click={() => showLogDialog = false}>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>

        <div class="dialog-content">
          {#if isLogLoading}
            <div class="loading-logs">
              <div class="spinner"></div>
              <span>Loading logs...</span>
            </div>
          {:else if logContent}
            <div class="log-content">
              <pre>{logContent}</pre>
            </div>
          {:else}
            <div class="no-logs">
              <p>No log content available.</p>
            </div>
          {/if}
        </div>

        <div class="dialog-actions">
          <button class="dialog-button secondary" on:click={() => showLogDialog = false}>
            Close
          </button>
          <button class="dialog-button primary" on:click={sendLogReport}>
            Send Report
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Success Message -->
  {#if showSuccessMessage}
    <div class="success-toast">
      <div class="toast-icon">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M22 11.08V12a10 10 0 11-5.93-9.14"/>
          <polyline points="22 4 12 14.01 9 11.01"/>
        </svg>
      </div>
      <span>{successMessage}</span>
    </div>
  {/if}
</div>

<style>
  .system-settings {
    background: var(--md-sys-color-surface);
    border-radius: 12px;
    padding: 18px;
    color: var(--md-sys-color-on-surface);
    max-width: 1280px;
    margin: 0 auto;
  }

  /* Header Styles */
  .settings-header {
    margin-bottom: 18px;
  }

  .header-content {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .header-icon {
    width: 38px;
    height: 38px;
    background: var(--md-sys-color-primary-container);
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--md-sys-color-on-primary-container);
  }

  .header-content h2 {
    margin: 0 0 4px 0;
    font-size: 20px;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
  }

  .header-content p {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 14px;
  }

  /* Loading and Error States */
  .loading-state, .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 48px;
    text-align: center;
    color: var(--md-sys-color-on-surface-variant);
  }

  .loading-spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--md-sys-color-surface-variant);
    border-top: 3px solid var(--md-sys-color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 16px;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .error-state {
    color: var(--md-sys-color-error);
  }

  .error-icon {
    margin-bottom: 12px;
    color: var(--md-sys-color-error);
  }

  .retry-button {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 16px;
    padding: 8px 16px;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
  }

  .retry-button:hover {
    background: color-mix(in srgb, var(--md-sys-color-primary) 90%, black);
  }

  /* Settings Content */
  .settings-content {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .loa-logs-reminder {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 12px;
    align-items: center;
    padding: 12px 14px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--md-sys-color-tertiary) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-tertiary) 24%, transparent);
    color: var(--md-sys-color-on-surface);
    font-size: 14px;
  }

  .loa-logs-reminder button {
    border: 0;
    border-radius: 8px;
    padding: 8px 12px;
    background: var(--app-color-subtle-scrim);
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font-weight: 600;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--md-sys-color-surface-variant);
    border-top: 2px solid var(--md-sys-color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  /* Success Toast */
  .success-toast {
    position: fixed;
    bottom: 24px;
    right: 24px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: var(--md-sys-color-tertiary);
    color: var(--md-sys-color-on-tertiary);
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    box-shadow: 0 4px 12px color-mix(in srgb, black 20%, transparent);
    z-index: 1000;
    animation: slideIn 0.3s ease;
  }

  @keyframes slideIn {
    from {
      transform: translateY(100%);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  /* Responsive Design */
  @media (max-width: 768px) {
    .system-settings {
      padding: 16px;
      margin: 0;
      border-radius: 0;
    }

    .success-toast {
      left: 16px;
      right: 16px;
      bottom: 16px;
    }
  }

  /* Log Dialog */
  .dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--app-color-overlay);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .log-dialog {
    background: var(--md-sys-color-surface);
    border-radius: 12px;
    box-shadow: var(--app-shadow-md);
    max-width: 800px;
    width: 90%;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--md-sys-color-outline);
  }

  .confirm-dialog {
    background: var(--md-sys-color-surface);
    border-radius: 12px;
    box-shadow: var(--app-shadow-md);
    max-width: 560px;
    width: 90%;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-error) 40%, var(--md-sys-color-outline));
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px 16px;
    border-bottom: 1px solid var(--md-sys-color-outline-variant);
  }

  .dialog-header h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
  }

  .dialog-close {
    background: none;
    border: none;
    padding: 4px;
    border-radius: 4px;
    cursor: pointer;
    color: var(--md-sys-color-on-surface-variant);
    transition: all 0.2s ease;
  }

  .dialog-close:hover {
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface);
  }

  .dialog-content {
    flex: 1;
    padding: 16px 24px;
    overflow-y: auto;
  }

  .loading-logs {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 40px;
    justify-content: center;
    color: var(--md-sys-color-on-surface-variant);
  }

  .log-content {
    background: var(--md-sys-color-surface-variant);
    border-radius: 8px;
    padding: 16px;
    max-height: 400px;
    overflow-y: auto;
  }

  .log-content pre {
    margin: 0;
    font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
    font-size: 11px;
    line-height: 1.4;
    color: var(--md-sys-color-on-surface);
    white-space: pre-wrap;
    word-wrap: break-word;
  }

  .no-logs {
    text-align: center;
    padding: 40px;
    color: var(--md-sys-color-on-surface-variant);
  }

  .dialog-actions {
    display: flex;
    gap: 12px;
    padding: 16px 24px 20px;
    border-top: 1px solid var(--md-sys-color-outline-variant);
    justify-content: flex-end;
  }

  .dialog-button {
    padding: 8px 16px;
    border-radius: 8px;
    border: none;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.2s ease;
  }

  .dialog-button.primary {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
  }

  .dialog-button.primary:hover {
    background: color-mix(in srgb, var(--md-sys-color-primary) 90%, black);
  }

  .dialog-button.danger {
    background: var(--md-sys-color-error);
    color: var(--md-sys-color-on-error);
  }

  .dialog-button.danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--md-sys-color-error) 90%, black);
  }

  .dialog-button.secondary {
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    border: 1px solid var(--md-sys-color-outline);
  }

  .dialog-button.secondary:hover {
    background: var(--md-sys-color-surface-variant);
  }

  .dialog-button:disabled,
  .dialog-close:disabled {
    cursor: wait;
    opacity: 0.65;
  }

  .confirm-content {
    display: flex;
    gap: 14px;
    color: var(--md-sys-color-on-surface);
  }

  .warning-icon {
    flex-shrink: 0;
    color: var(--md-sys-color-error);
  }

  .confirm-content p {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 13px;
    line-height: 1.5;
  }

  .confirm-content .confirm-title {
    margin-bottom: 8px;
    color: var(--md-sys-color-on-surface);
    font-size: 15px;
    font-weight: 700;
  }

  /* Responsive Design */
</style>
