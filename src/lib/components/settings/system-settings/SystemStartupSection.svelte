<script lang="ts">
  import ToggleSwitch from '$lib/components/common/ToggleSwitch.svelte';

  export let startWithWindows = false;
  export let hideOnLaunch = false;
  export let startWithLostArk = false;
  export let startWithLoaLogs = false;
  export let isRunning = false;
  export let isLoaLogsRunning = false;
  export let onToggleStartWithWindows: () => void;
  export let onToggleHideOnLaunch: () => void;
  export let onToggleStartWithLostArk: () => void;
  export let onToggleStartWithLoaLogs: () => void;
</script>

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
        <ToggleSwitch checked={startWithWindows} ariaLabel="Start with Windows" on:change={onToggleStartWithWindows} />
      </div>
    </div>

    <div class="setting-card toggle-card">
      <div class="setting-header">
        <div class="setting-icon windows">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M6 18L18 6"/>
            <path d="M8 6h10v10"/>
            <path d="M6 8v10h10"/>
          </svg>
        </div>
        <div class="toggle-content">
          <h4>Hide on Launch</h4>
          <p>Start LOA Tracker hidden in the tray area</p>
        </div>
        <ToggleSwitch checked={hideOnLaunch} ariaLabel="Hide on launch" on:change={onToggleHideOnLaunch} />
      </div>
    </div>

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
        <ToggleSwitch checked={startWithLostArk} ariaLabel="Start with Lost Ark" on:change={onToggleStartWithLostArk} />
      </div>
    </div>

    <div class="setting-card toggle-card">
      <div class="setting-header">
        <div class="setting-icon game">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z"/>
          </svg>
        </div>
        <div class="toggle-content">
          <h4>Monitor LOA Logs Startup</h4>
          <p>Watch for LOA Logs while LOA Tracker is running and show reminders without registering Windows autostart</p>
          <div class="lost-ark-status">
            <span class="status-dot" class:running={isLoaLogsRunning}></span>
            <span class="status-text">{isLoaLogsRunning ? 'LOA Logs is running' : 'LOA Logs is not running'}</span>
          </div>
        </div>
        <ToggleSwitch checked={startWithLoaLogs} ariaLabel="Monitor LOA Logs startup" on:change={onToggleStartWithLoaLogs} />
      </div>
    </div>
  </div>
</div>

<style>
  .lost-ark-status {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
  }

  .lost-ark-status .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--md-sys-color-error);
  }

  .lost-ark-status .status-dot.running {
    background: var(--md-sys-color-success);
  }

  .status-text {
    font-size: 12px;
    color: var(--md-sys-color-on-surface-variant);
  }
</style>
