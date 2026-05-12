<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';

  // State
  let systemSettings: any = null;
  let isLoading = true;
  let error = '';
  let successMessage = '';
  let showSuccessMessage = false;
  
  // System settings
  let encountersDbPath = '';
  let lostArkExePath = '';
  let startWithWindows = false;
  let startWithLostArk = false;
  let isRunning = false;

  // Logging
  let logContent: string | null = null;
  let isLogLoading = false;
  let showLogDialog = false;

  // Load system settings on mount
  onMount(async () => {
    await loadSystemSettings();
    await checkLostArkStatus();
    
    // Check Lost Ark status periodically
    const statusInterval = setInterval(async () => {
      await checkLostArkStatus();
    }, 5000); // Check every 5 seconds

    return () => {
      clearInterval(statusInterval);
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
      startWithWindows = settings.startWithWindows || settings.start_with_windows || false;
      startWithLostArk = settings.startWithLostArk || settings.start_with_lost_ark || false;
      
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

  // Check if Lost Ark is running
  async function checkLostArkStatus() {
    try {
      isRunning = await invoke('is_lost_ark_running');
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
        </div>
      </div>

      <!-- Startup Settings Section -->
      <div class="settings-section">
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

  .dialog-button.secondary {
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    border: 1px solid var(--md-sys-color-outline);
  }

  .dialog-button.secondary:hover {
    background: var(--md-sys-color-surface-variant);
  }

  /* Responsive Design */
</style>
