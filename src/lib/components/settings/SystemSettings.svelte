<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { activeRosterId, characters, loadCharacters, loadRosters, rosters } from '$lib/store';
  import {
    isMeowConnectFeatureEnabled,
    isMeowConnectRealtimeEnabled,
    setMeowConnectFeatureEnabled,
    setMeowConnectRealtimeEnabled
  } from '$lib/services/meow-connect';
  import { setSplitRatTodoView, splitRatTodoView } from '$lib/services/todo-preferences';

  // State
  let systemSettings: any = null;
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
  let showSetupGuideButton = true;
  let showAuthWelcome = true;
  let meowConnectEnabled = true;
  let meowConnectRealtimeEnabled = true;
  let dashboardView: 'cards' | 'compact' = 'compact';
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

      const settings: any = await invoke('get_system_settings');
      systemSettings = settings;

      // Update local state (use camelCase from backend serialization)
      encountersDbPath = settings.encountersDbPath || settings.encounters_db_path || '';
      lostArkExePath = settings.lostArkExePath || settings.lost_ark_exe_path || '';
      loaLogsExePath = settings.loaLogsExePath || settings.loa_logs_exe_path || '';
      startWithWindows = settings.startWithWindows || settings.start_with_windows || false;
      startWithLostArk = settings.startWithLostArk || settings.start_with_lost_ark || false;
      startWithLoaLogs = settings.startWithLoaLogs || settings.start_with_loa_logs || false;
      showSetupGuideButton = settings.showSetupGuideButton ?? settings.show_setup_guide_button ?? true;
      showAuthWelcome = settings.showAuthWelcome ?? settings.show_auth_welcome ?? true;
      meowConnectEnabled = isMeowConnectFeatureEnabled();
      meowConnectRealtimeEnabled = isMeowConnectRealtimeEnabled();
      dashboardView = localStorage.getItem('dashboardView') === 'cards' ? 'cards' : 'compact';

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
      await invoke('set_encounters_db_path', { path });
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
      await invoke('set_loa_logs_exe_path', { path });
      loaLogsExePath = path;
      showSuccess('LOA Logs executable path updated successfully!');
    } catch (err) {
      showError(`Failed to set LOA Logs executable path: ${err}`);
    }
  }

  async function setLostArkExePath(path: string) {
    try {
      await invoke('set_lost_ark_exe_path', { path });
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
      await invoke('set_start_with_windows', { enabled: newValue });
      startWithWindows = newValue;
      showSuccess(`Start with Windows ${newValue ? 'enabled' : 'disabled'}!`);
    } catch (err) {
      showError(`Failed to toggle start with Windows: ${err}`);
    }
  }

  async function toggleStartWithLostArk() {
    try {
      const newValue = !startWithLostArk;
      await invoke('set_start_with_lost_ark', { enabled: newValue });
      startWithLostArk = newValue;
      showSuccess(`Start with Lost Ark ${newValue ? 'enabled' : 'disabled'}!`);
    } catch (err) {
      showError(`Failed to toggle start with Lost Ark: ${err}`);
    }
  }

  async function toggleStartWithLoaLogs() {
    try {
      const newValue = !startWithLoaLogs;
      await invoke('set_start_with_loa_logs', { enabled: newValue });
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
      await invoke('set_show_setup_guide_button', { enabled: newValue });
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
      await invoke('set_show_auth_welcome', { enabled: newValue });
      showAuthWelcome = newValue;
      showSuccess(`Welcome screen ${newValue ? 'enabled' : 'disabled'}!`);
    } catch (err) {
      showError(`Failed to update welcome screen: ${err}`);
    }
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

  function toggleSplitRatTodoView() {
    const newValue = !$splitRatTodoView;
    setSplitRatTodoView(newValue);
    showSuccess(`RAT To Do view ${newValue ? 'enabled' : 'disabled'}!`);
  }

  function setDashboardView(view: 'cards' | 'compact') {
    dashboardView = view;
    localStorage.setItem('dashboardView', view);
    window.dispatchEvent(new CustomEvent('dashboard-view:changed', { detail: view }));
    showSuccess(`Dashboard view set to ${view === 'compact' ? 'List' : 'Cards'}!`);
  }

  function requestClearUserData() {
    showClearUserDataDialog = true;
  }

  async function confirmClearUserData() {
    try {
      isClearingUserData = true;
      const result: string = await invoke('clear_user_data');
      showClearUserDataDialog = false;
      localStorage.removeItem('activeRosterId');
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
      isRunning = await invoke('is_lost_ark_running');
      try {
        isLoaLogsRunning = await invoke('is_loa_logs_running');
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
      logContent = await invoke('get_log_content');
    } catch (err) {
      showError(`Failed to load log content: ${err}`);
    } finally {
      isLogLoading = false;
    }
  }

  async function clearLog() {
    try {
      await invoke('clear_log');
      logContent = null;
      showSuccess('Log cleared successfully!');
    } catch (err) {
      showError(`Failed to clear log: ${err}`);
    }
  }

  async function sendLogReport() {
    try {
      const result: string = await invoke('send_log_report');
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

  function formatPath(path: string): string {
    if (!path) return 'No path selected';
    if (path.length > 60) {
      return '...' + path.substring(path.length - 57);
    }
    return path;
  }

  function isPathValid(path: string): boolean {
    return path !== null && path !== undefined && path.trim().length > 0;
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

      <!-- General Section -->
      <div class="settings-section">
        <div class="section-header">
          <div class="section-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="3"/>
              <path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-2 2 2 2 0 01-2-2v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83 0 2 2 0 010-2.83l.06-.06A1.65 1.65 0 004.6 15a1.65 1.65 0 00-1.51-1H3a2 2 0 01-2-2 2 2 0 012-2h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 010-2.83 2 2 0 012.83 0l.06.06A1.65 1.65 0 009 4.6a1.65 1.65 0 001-1.51V3a2 2 0 012-2 2 2 0 012 2v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 0 2 2 0 010 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 012 2 2 2 0 01-2 2h-.09a1.65 1.65 0 00-1.51 1z"/>
            </svg>
          </div>
          <div>
            <h3>General</h3>
            <p>Manage global app behavior and local user data</p>
          </div>
        </div>

        <div class="settings-grid">
          <div class="setting-card toggle-card">
            <div class="setting-header">
              <div class="setting-icon windows">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <circle cx="12" cy="12" r="10"/>
                  <path d="M9.09 9a3 3 0 015.83 1c0 2-3 3-3 3"/>
                  <line x1="12" y1="17" x2="12.01" y2="17"/>
                </svg>
              </div>
              <div class="toggle-content">
                <h4>Show Set-Up Guide</h4>
                <p>Show the Set-Up Guide button in the main header</p>
              </div>
              <label class="toggle-switch">
                <input
                  type="checkbox"
                  checked={showSetupGuideButton}
                  on:change={toggleSetupGuideButton}
                />
                <span class="toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="setting-card toggle-card">
            <div class="setting-header">
              <div class="setting-icon windows">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M20 21v-2a4 4 0 00-4-4H8a4 4 0 00-4 4v2"/>
                  <circle cx="12" cy="7" r="4"/>
                </svg>
              </div>
              <div class="toggle-content">
                <h4>Show Welcome Screen</h4>
                <p>Show the Discord welcome screen before opening the app</p>
              </div>
              <label class="toggle-switch">
                <input
                  type="checkbox"
                  checked={showAuthWelcome}
                  on:change={toggleAuthWelcome}
                />
                <span class="toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="setting-card toggle-card">
            <div class="setting-header">
              <div class="setting-icon windows">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M8 6h13"/>
                  <path d="M8 12h13"/>
                  <path d="M8 18h13"/>
                  <path d="M3 6h.01"/>
                  <path d="M3 12h.01"/>
                  <path d="M3 18h.01"/>
                </svg>
              </div>
              <div class="toggle-content">
                <h4>Separate RAT To Do View</h4>
                <p>Add a frontend-only RAT roster button in To Do for non-gold characters across all rosters</p>
              </div>
              <label class="toggle-switch">
                <input
                  type="checkbox"
                  checked={$splitRatTodoView}
                  on:change={toggleSplitRatTodoView}
                />
                <span class="toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="setting-card option-card">
            <div class="setting-header">
              <div class="setting-icon windows">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <rect x="3" y="4" width="18" height="16" rx="2"/>
                  <path d="M7 8h10"/>
                  <path d="M7 12h10"/>
                  <path d="M7 16h6"/>
                </svg>
              </div>
              <div class="toggle-content">
                <h4>Dashboard View</h4>
                <p>Choose how roster characters are shown on the Dashboard</p>
              </div>
              <div class="setting-segmented" aria-label="Dashboard view mode">
                <button
                  type="button"
                  class:active={dashboardView === 'cards'}
                  on:click={() => setDashboardView('cards')}
                >
                  Cards
                </button>
                <button
                  type="button"
                  class:active={dashboardView === 'compact'}
                  on:click={() => setDashboardView('compact')}
                >
                  List
                </button>
              </div>
            </div>
          </div>

          <div class="setting-card danger-card">
            <div class="setting-header">
              <div class="setting-icon danger">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="3 6 5 6 21 6"/>
                  <path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
                </svg>
              </div>
              <div class="toggle-content">
                <h4>Clear User Data</h4>
                <p>Delete all characters and connected tracking data from userlogs.db</p>
              </div>
              <button class="danger-button" on:click={requestClearUserData} disabled={isClearingUserData}>
                {isClearingUserData ? 'Clearing...' : 'Clear Data'}
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- MeowConnect Section -->
      <div class="settings-section" data-guide="system-meowconnect">
        <div class="section-header">
          <div class="section-icon">
            <img src="/images/meowconnect_tab.png" alt="" class="section-image-icon" />
          </div>
          <div>
            <h3>MeowConnect</h3>
            <p>Control shared availability and live cloud updates</p>
          </div>
        </div>

        <div class="settings-grid">
          <div class="setting-card toggle-card">
            <div class="setting-header">
              <div class="setting-icon meowconnect-icon">
                <img src="/images/meowconnect_tab.png" alt="" />
              </div>
              <div class="toggle-content">
                <h4>MeowConnect</h4>
                <p>Enable or disable the MeowConnect feature</p>
              </div>
              <label class="toggle-switch">
                <input
                  type="checkbox"
                  checked={meowConnectEnabled}
                  on:change={toggleMeowConnectEnabled}
                />
                <span class="toggle-slider"></span>
              </label>
            </div>
          </div>

          <div class="setting-card toggle-card" class:disabled-card={!meowConnectEnabled}>
            <div class="setting-header">
              <div class="setting-icon meowconnect-icon">
                <img src="/images/meowconnect_tab.png" alt="" />
              </div>
              <div class="toggle-content">
                <h4>Real-time MeowConnect</h4>
                <p>Enable real-time raid completion syncing with the cloud database</p>
              </div>
              <label class="toggle-switch">
                <input
                  type="checkbox"
                  checked={meowConnectRealtimeEnabled}
                  disabled={!meowConnectEnabled}
                  on:change={toggleMeowConnectRealtime}
                />
                <span class="toggle-slider"></span>
              </label>
            </div>
          </div>
        </div>
      </div>

      <!-- File Paths Section -->
      <div class="settings-section">
        <div class="section-header">
          <div class="section-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
            </svg>
          </div>
          <div>
            <h3>File Paths</h3>
            <p>Configure paths to external files and databases</p>
          </div>
        </div>

        <div class="settings-grid">
          <!-- encounters.db Path -->
          <div class="setting-card">
            <div class="setting-header">
              <div class="setting-icon database">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <ellipse cx="12" cy="5" rx="9" ry="3"/>
                  <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/>
                  <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>
                </svg>
              </div>
              <div>
                <h4>encounters.db</h4>
                <p>LOA Logs combat database for auto-completion</p>
              </div>
            </div>

            <div class="path-input-group">
              <div class="path-display" class:valid={isPathValid(encountersDbPath)} class:invalid={!isPathValid(encountersDbPath)}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M13 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V9z"/>
                  <polyline points="13 2 13 9 20 9"/>
                </svg>
                <span class="path-text">{formatPath(encountersDbPath)}</span>
              </div>
              <button class="browse-button" on:click={browseEncountersDb}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"/>
                  <polyline points="9 22 9 12 15 12 15 22"/>
                </svg>
                Browse
              </button>
            </div>

            {#if isPathValid(encountersDbPath)}
              <div class="status-indicator success">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                  <polyline points="20 6 9 17 4 12"/>
                </svg>
                Database connected
              </div>
            {:else}
              <div class="status-indicator warning">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <circle cx="12" cy="12" r="10"/>
                  <line x1="12" y1="8" x2="12" y2="12"/>
                  <line x1="12" y1="16" x2="12.01" y2="16"/>
                </svg>
                No database selected
              </div>
            {/if}
          </div>

          <!-- LostArk.exe Path -->
          <div class="setting-card">
            <div class="setting-header">
              <div class="setting-icon executable">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
                  <polyline points="14 2 14 8 20 8"/>
                  <line x1="12" y1="18" x2="12" y2="12"/>
                  <line x1="9" y1="15" x2="15" y2="15"/>
                </svg>
              </div>
              <div>
                <h4>LostArk.exe</h4>
                <p>Lost Ark game executable for auto-launch</p>
              </div>
            </div>

            <div class="path-input-group">
              <div class="path-display" class:valid={isPathValid(lostArkExePath)} class:invalid={!isPathValid(lostArkExePath)}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
                  <polyline points="14 2 14 8 20 8"/>
                </svg>
                <span class="path-text">{formatPath(lostArkExePath)}</span>
              </div>
              <button class="browse-button" on:click={browseLostArkExe}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"/>
                  <polyline points="9 22 9 12 15 12 15 22"/>
                </svg>
                Browse
              </button>
            </div>

            {#if isPathValid(lostArkExePath)}
              <div class="status-indicator success">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                  <polyline points="20 6 9 17 4 12"/>
                </svg>
                Executable found
              </div>
            {:else}
              <div class="status-indicator warning">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <circle cx="12" cy="12" r="10"/>
                  <line x1="12" y1="8" x2="12" y2="12"/>
                  <line x1="12" y1="16" x2="12.01" y2="16"/>
                </svg>
                No executable selected
              </div>
            {/if}
          </div>

          <!-- LOA Logs.exe Path -->
          <div class="setting-card">
            <div class="setting-header">
              <div class="setting-icon executable">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
                  <polyline points="14 2 14 8 20 8"/>
                </svg>
              </div>
              <div>
                <h4>LOA Logs.exe</h4>
                <p>LOA Logs executable to auto-launch when LOA Tracker starts</p>
              </div>
            </div>

            <div class="path-input-group">
              <div class="path-display" class:valid={isPathValid(loaLogsExePath)} class:invalid={!isPathValid(loaLogsExePath)}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
                  <polyline points="14 2 14 8 20 8"/>
                </svg>
                <span class="path-text">{formatPath(loaLogsExePath)}</span>
              </div>
              <button class="browse-button" on:click={browseLoaLogsExe}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"/>
                  <polyline points="9 22 9 12 15 12 15 22"/>
                </svg>
                Browse
              </button>
            </div>

            {#if isPathValid(loaLogsExePath)}
              <div class="status-indicator success">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                  <polyline points="20 6 9 17 4 12"/>
                </svg>
                Executable found
              </div>
            {:else}
              <div class="status-indicator warning">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <circle cx="12" cy="12" r="10"/>
                  <line x1="12" y1="8" x2="12" y2="12"/>
                  <line x1="12" y1="16" x2="12.01" y2="16"/>
                </svg>
                No executable selected
              </div>
            {/if}
          </div>
        </div>
      </div>

      <!-- Startup Settings Section -->
      <div class="settings-section" data-guide="system-startup">
        <div class="section-header">
          <div class="section-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 12h-4l-3 9L9 3l-3 9H2"/>
            </svg>
          </div>
          <div>
            <h3>Startup Settings</h3>
            <p>Configure how and when the application starts</p>
          </div>
        </div>

        <div class="settings-grid">
          <!-- Start with Windows -->
          <div class="setting-card toggle-card">
            <div class="setting-header">
              <div class="setting-icon windows">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <rect x="2" y="3" width="20" height="14" rx="2" ry="2"/>
                  <line x1="8" y1="21" x2="16" y2="21"/>
                  <line x1="12" y1="17" x2="12" y2="21"/>
                </svg>
              </div>
              <div class="toggle-content">
                <h4>Start with Windows</h4>
                <p>Launch LOA Tracker automatically when Windows starts</p>
              </div>
              <label class="toggle-switch">
                <input
                  type="checkbox"
                  checked={startWithWindows}
                  on:change={toggleStartWithWindows}
                />
                <span class="toggle-slider"></span>
              </label>
            </div>
          </div>

          <!-- Start with Lost Ark -->
          <div class="setting-card toggle-card">
            <div class="setting-header">
              <div class="setting-icon game">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z"/>
                </svg>
              </div>
              <div class="toggle-content">
                <h4>Start with Lost Ark</h4>
                <p>Automatically launch when Lost Ark starts</p>
                <div class="lost-ark-status">
                  <span class="status-dot" class:running={isRunning}></span>
                  <span class="status-text">{isRunning ? 'Lost Ark is running' : 'Lost Ark is not running'}</span>
                </div>
              </div>
              <label class="toggle-switch">
                <input
                  type="checkbox"
                  checked={startWithLostArk}
                  on:change={toggleStartWithLostArk}
                />
                <span class="toggle-slider"></span>
              </label>
            </div>
          </div>

          <!-- Start with LOA Logs -->
          <div class="setting-card toggle-card">
            <div class="setting-header">
              <div class="setting-icon game">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z"/>
                </svg>
              </div>
              <div class="toggle-content">
                <h4>Monitor LOA Logs Startup</h4>
                <p>Temporarily only reminds you to start LOA Logs manually and reveals LOA Tracker when it starts</p>
                <div class="lost-ark-status">
                  <span class="status-dot" class:running={isLoaLogsRunning}></span>
                  <span class="status-text">{isLoaLogsRunning ? 'LOA Logs is running' : 'LOA Logs is not running'}</span>
                </div>
              </div>
              <label class="toggle-switch">
                <input
                  type="checkbox"
                  checked={startWithLoaLogs}
                  on:change={toggleStartWithLoaLogs}
                />
                <span class="toggle-slider"></span>
              </label>
            </div>
          </div>
        </div>
      </div>

      <!-- Logging Section -->
      <div class="settings-section">
        <div class="section-header">
          <div class="section-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
              <polyline points="14 2 14 8 20 8"/>
              <line x1="16" y1="13" x2="8" y2="13"/>
              <line x1="16" y1="17" x2="8" y2="17"/>
              <polyline points="10 9 9 9 8 9"/>
            </svg>
          </div>
          <div>
            <h3>Logging & Diagnostics</h3>
            <p>View application logs and create diagnostic reports</p>
          </div>
        </div>

        <div class="logging-section">
          <div class="logging-actions">
            <button class="log-button" on:click={openLogDialog}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
                <polyline points="14 2 14 8 20 8"/>
                <line x1="16" y1="13" x2="8" y2="13"/>
                <line x1="16" y1="17" x2="8" y2="17"/>
                <polyline points="10 9 9 9 8 9"/>
              </svg>
              View Logs
            </button>

            <button class="log-button report" on:click={sendLogReport}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
              Send Report
            </button>

            <button class="log-button clear" on:click={clearLog}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="3 6 5 6 21 6"/>
                <path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
              </svg>
              Clear Logs
            </button>
          </div>

          <div class="logging-info">
            <p>Logs are stored in: %LOCALAPPDATA%\LOAtracker\logs\loatracker.log</p>
            <p>Use "Send Report" to create a diagnostic file for support requests.</p>
          </div>
        </div>
      </div>
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
    <div class="dialog-overlay" on:click={() => showLogDialog = false}>
      <div class="log-dialog" on:click|stopPropagation>
        <div class="dialog-header">
          <h3>Application Logs</h3>
          <button class="dialog-close" on:click={() => showLogDialog = false}>
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
    border-radius: 16px;
    padding: 24px;
    color: var(--md-sys-color-on-surface);
    max-width: 1200px;
    margin: 0 auto;
  }

  /* Header Styles */
  .settings-header {
    margin-bottom: 32px;
  }

  .header-content {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .header-icon {
    width: 48px;
    height: 48px;
    background: var(--md-sys-color-primary-container);
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--md-sys-color-on-primary-container);
  }

  .header-content h2 {
    margin: 0 0 4px 0;
    font-size: 24px;
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
    gap: 32px;
  }

  .loa-logs-reminder {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 12px;
    align-items: center;
    padding: 12px 14px;
    border-radius: 10px;
    background: rgba(255, 140, 0, 0.12);
    border: 1px solid rgba(255, 140, 0, 0.24);
    color: var(--md-sys-color-on-surface);
    font-size: 14px;
  }

  .loa-logs-reminder button {
    border: 0;
    border-radius: 8px;
    padding: 8px 12px;
    background: rgba(0, 0, 0, 0.1);
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font-weight: 600;
  }

  /* Section Styles */
  .settings-section {
    background: var(--md-sys-color-surface-container);
    border-radius: 12px;
    padding: 20px;
    border: 1px solid var(--md-sys-color-outline-variant);
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 20px;
  }

  .section-icon {
    width: 40px;
    height: 40px;
    background: var(--md-sys-color-secondary-container);
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--md-sys-color-on-secondary-container);
  }

  .section-header h3 {
    margin: 0 0 2px 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
  }

  .section-header p {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 13px;
  }

  .section-image-icon {
    width: 22px;
    height: 22px;
    object-fit: contain;
    display: block;
  }

  /* Settings Grid */
  .settings-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
    gap: 16px;
  }

  /* Setting Cards */
  .setting-card {
    background: var(--md-sys-color-surface);
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 12px;
    padding: 16px;
    transition: all 0.2s ease;
  }

  .setting-card:hover {
    border-color: var(--md-sys-color-primary);
    box-shadow: 0 2px 8px color-mix(in srgb, var(--md-sys-color-primary) 20%, transparent);
  }

  .setting-header {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 12px;
  }

  .setting-icon {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .setting-icon.database {
    background: color-mix(in srgb, var(--md-sys-color-tertiary) 20%, transparent);
    color: var(--md-sys-color-tertiary);
  }

  .setting-icon.executable {
    background: color-mix(in srgb, var(--md-sys-color-primary) 20%, transparent);
    color: var(--md-sys-color-primary);
  }

  .setting-icon.windows {
    background: color-mix(in srgb, var(--md-sys-color-secondary) 20%, transparent);
    color: var(--md-sys-color-secondary);
  }

  .setting-icon.game {
    background: color-mix(in srgb, var(--md-sys-color-tertiary) 20%, transparent);
    color: var(--md-sys-color-tertiary);
  }

  .setting-icon.danger {
    background: color-mix(in srgb, var(--md-sys-color-error) 16%, transparent);
    color: var(--md-sys-color-error);
  }

  .setting-icon.meowconnect-icon {
    background: color-mix(in srgb, var(--md-sys-color-primary) 20%, transparent);
  }

  .setting-icon.meowconnect-icon img {
    width: 22px;
    height: 22px;
    object-fit: contain;
    display: block;
  }

  .disabled-card {
    opacity: 0.62;
  }

  .danger-card {
    border-color: color-mix(in srgb, var(--md-sys-color-error) 35%, var(--md-sys-color-outline-variant));
  }

  .option-card .setting-header {
    align-items: center;
    margin-bottom: 0;
  }

  .setting-segmented {
    flex-shrink: 0;
    display: inline-flex;
    padding: 3px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface-variant);
  }

  .setting-segmented button {
    border: 0;
    border-radius: 6px;
    padding: 7px 11px;
    background: transparent;
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
    font: inherit;
    font-size: 12px;
    font-weight: 700;
    white-space: nowrap;
  }

  .setting-segmented button.active {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
  }

  .danger-button {
    flex-shrink: 0;
    padding: 8px 14px;
    background: var(--md-sys-color-error);
    color: var(--md-sys-color-on-error);
    border: 1px solid var(--md-sys-color-error);
    border-radius: 8px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    transition: all 0.2s ease;
  }

  .danger-button:hover:not(:disabled) {
    background: color-mix(in srgb, var(--md-sys-color-error) 90%, black);
  }

  .danger-button:disabled {
    cursor: wait;
    opacity: 0.65;
  }

  .setting-header h4 {
    margin: 0 0 2px 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
  }

  .setting-header p {
    margin: 0;
    font-size: 12px;
    color: var(--md-sys-color-on-surface-variant);
    line-height: 1.4;
  }

  /* Path Input Group */
  .path-input-group {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
    align-items: stretch;
    min-height: 44px; /* Ensure consistent height */
  }

  .path-display {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface-variant);
    font-size: 13px;
    color: var(--md-sys-color-on-surface-variant);
    transition: all 0.2s ease;
    min-width: 0; /* Allow flex item to shrink */
  }

  .path-display.valid {
    border-color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 10%, transparent);
    color: var(--md-sys-color-on-surface);
  }

  .path-display.invalid {
    border-color: var(--md-sys-color-error);
    background: color-mix(in srgb, var(--md-sys-color-error) 10%, transparent);
  }

  .path-text {
    font-family: 'Consolas', 'Monaco', monospace;
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0; /* Allow text to shrink */
  }

  .browse-button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 16px;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.2s ease;
    white-space: nowrap;
    flex-shrink: 0; /* Prevent button from shrinking */
    min-width: fit-content; /* Ensure button fits its content */
  }

  .browse-button:hover {
    background: color-mix(in srgb, var(--md-sys-color-primary) 90%, black);
    transform: translateY(-1px);
  }

  /* Status Indicators */
  .status-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 500;
    border-radius: 6px;
    margin-top: 8px;
  }

  .status-indicator.success svg {
    color: #22c55e; /* Green dot for success */
  }

  .status-indicator.warning svg {
    color: #ef4444; /* Red dot for warning/error */
  }

  .status-indicator.success {
    color: var(--md-sys-color-on-surface);
    background: color-mix(in srgb, var(--md-sys-color-surface-variant) 50%, transparent);
  }

  .status-indicator.warning {
    color: var(--md-sys-color-on-surface);
    background: color-mix(in srgb, var(--md-sys-color-surface-variant) 50%, transparent);
  }

  /* Toggle Cards */
  .toggle-card {
    cursor: pointer;
  }

  .toggle-card .setting-header {
    justify-content: space-between;
  }

  .toggle-content {
    flex: 1;
  }

  /* Lost Ark Status Styles */
  .lost-ark-status {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }

  .lost-ark-status .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #ef4444;
  }

  .lost-ark-status .status-dot.running {
    background: #22c55e;
  }

  .status-text {
    font-size: 12px;
    color: var(--md-sys-color-on-surface-variant);
  }

  /* Toggle Switch */
  .toggle-switch {
    position: relative;
    width: 48px;
    height: 24px;
  }

  .toggle-switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--md-sys-color-surface-variant);
    border: 1px solid var(--md-sys-color-outline);
    transition: all 0.3s ease;
    border-radius: 24px;
  }

  .toggle-slider:before {
    position: absolute;
    content: "";
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 2px;
    background: var(--md-sys-color-on-surface-variant);
    transition: all 0.3s ease;
    border-radius: 50%;
  }

  input:checked + .toggle-slider {
    background: var(--md-sys-color-primary);
    border-color: var(--md-sys-color-primary);
  }

  input:checked + .toggle-slider:before {
    transform: translateX(24px);
    background: var(--md-sys-color-on-primary);
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

    .settings-grid {
      grid-template-columns: 1fr;
    }

    .option-card .setting-header {
      align-items: flex-start;
      flex-wrap: wrap;
    }

    .setting-segmented {
      width: 100%;
    }

    .setting-segmented button {
      flex: 1;
    }

    .path-input-group {
      flex-direction: column;
      min-height: auto;
    }

    .browse-button {
      width: 100%;
      justify-content: center;
    }

    .success-toast {
      left: 16px;
      right: 16px;
      bottom: 16px;
    }
  }

  /* Logging Section */
  .logging-section {
    max-width: 600px;
  }

  .logging-actions {
    display: flex;
    gap: 12px;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }

  .log-button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.2s ease;
  }

  .log-button:hover {
    background: var(--md-sys-color-surface-variant);
    border-color: var(--md-sys-color-outline);
  }

  .log-button.report {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border-color: var(--md-sys-color-primary);
  }

  .log-button.report:hover {
    background: color-mix(in srgb, var(--md-sys-color-primary) 90%, black);
  }

  .log-button.clear {
    background: var(--md-sys-color-error);
    color: var(--md-sys-color-on-error);
    border-color: var(--md-sys-color-error);
  }

  .log-button.clear:hover {
    background: color-mix(in srgb, var(--md-sys-color-error) 90%, black);
  }

  .logging-info {
    padding: 12px 16px;
    background: var(--md-sys-color-surface-variant);
    border-radius: 8px;
    border: 1px solid var(--md-sys-color-outline-variant);
  }

  .logging-info p {
    margin: 0 0 4px 0;
    font-size: 12px;
    color: var(--md-sys-color-on-surface-variant);
  }

  .logging-info p:last-child {
    margin-bottom: 0;
  }

  /* Log Dialog */
  .dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .log-dialog {
    background: var(--md-sys-color-surface);
    border-radius: 12px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
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
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
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
