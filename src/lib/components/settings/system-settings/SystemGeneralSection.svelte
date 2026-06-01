<script lang="ts">
  import SegmentedControl from '$lib/components/common/SegmentedControl.svelte';
  import ToggleSwitch from '$lib/components/common/ToggleSwitch.svelte';
  import { APP_THEME_OPTIONS, type AppThemeId } from '$lib/data/themes';

  export let showSetupGuideButton = true;
  export let showDashboardStaticBadges = true;
  export let showAuthWelcome = true;
  export let showHaalsHourglassReminder = true;
  export let showHeaderCountdown = true;
  export let splitRatTodoView = false;
  export let dashboardView: 'cards' | 'compact' = 'compact';
  export let appTheme: AppThemeId = 'outlaw';
  export let isClearingUserData = false;
  export let onToggleSetupGuideButton: () => void;
  export let onToggleDashboardStaticBadges: () => void;
  export let onToggleAuthWelcome: () => void;
  export let onToggleHaalsHourglassReminder: () => void;
  export let onToggleHeaderCountdown: () => void;
  export let onToggleSplitRatTodoView: () => void;
  export let onSetDashboardView: (view: 'cards' | 'compact') => void;
  export let onSetAppTheme: (theme: AppThemeId) => void;
  export let onRequestClearUserData: () => void;

  const dashboardViewOptions = [
    { value: 'cards', label: 'Cards' },
    { value: 'compact', label: 'List' }
  ];

  const themeOptions = APP_THEME_OPTIONS.map((theme) => ({
    value: theme.id,
    label: theme.label
  }));
</script>

<div class="settings-section" data-guide="system-general">
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
        <ToggleSwitch checked={showSetupGuideButton} ariaLabel="Show Set-Up Guide" on:change={onToggleSetupGuideButton} />
      </div>
    </div>

    <div class="setting-card toggle-card">
      <div class="setting-header">
        <div class="setting-icon windows">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M20 7h-9"/>
            <path d="M14 17H5"/>
            <circle cx="17" cy="17" r="3"/>
            <circle cx="7" cy="7" r="3"/>
          </svg>
        </div>
        <div class="toggle-content">
          <h4>Dashboard Static Badges</h4>
          <p>Show Static and group tag badges on dashboard raid labels</p>
        </div>
        <ToggleSwitch checked={showDashboardStaticBadges} ariaLabel="Show dashboard static badges" on:change={onToggleDashboardStaticBadges} />
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
        <ToggleSwitch checked={showAuthWelcome} ariaLabel="Show welcome screen" on:change={onToggleAuthWelcome} />
      </div>
    </div>

    <div class="setting-card toggle-card">
      <div class="setting-header">
        <div class="setting-icon windows">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="9"/>
            <path d="M8 3h8"/>
            <path d="M8 21h8"/>
            <path d="M9 8h6"/>
            <path d="M9 16h6"/>
            <path d="M12 8v2"/>
            <path d="M10 14c1.4-1.2 2.6-1.2 4 0"/>
          </svg>
        </div>
        <div class="toggle-content">
          <h4>Haal's Hourglass Reminder</h4>
          <p>Warn on Tuesday when a 1730+ character still has tracked Cube unfinished before weekly reset</p>
        </div>
        <ToggleSwitch checked={showHaalsHourglassReminder} ariaLabel="Show Haal's Hourglass reminder" on:change={onToggleHaalsHourglassReminder} />
      </div>
    </div>

    <div class="setting-card toggle-card">
      <div class="setting-header">
        <div class="setting-icon windows">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <polyline points="12 6 12 12 16 14"/>
          </svg>
        </div>
        <div class="toggle-content">
          <h4>Show Header Countdown</h4>
          <p>Show daily and weekly reset timing in the app header</p>
        </div>
        <ToggleSwitch checked={showHeaderCountdown} ariaLabel="Show header countdown" on:change={onToggleHeaderCountdown} />
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
        <ToggleSwitch checked={splitRatTodoView} ariaLabel="Separate RAT To Do view" on:change={onToggleSplitRatTodoView} />
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
        <div class="dashboard-view-control">
          <SegmentedControl
            options={dashboardViewOptions}
            value={dashboardView}
            ariaLabel="Dashboard view mode"
            density="compact"
            on:change={(event) => onSetDashboardView(event.detail as 'cards' | 'compact')}
          />
        </div>
      </div>
    </div>

    <div class="setting-card option-card theme-card">
      <div class="theme-header">
        <div class="setting-header">
          <div class="setting-icon windows">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="13.5" cy="6.5" r=".5"/>
              <circle cx="17.5" cy="10.5" r=".5"/>
              <circle cx="8.5" cy="7.5" r=".5"/>
              <circle cx="6.5" cy="12.5" r=".5"/>
              <path d="M12 2C6.5 2 2 6.1 2 11.2c0 3.1 2 5.8 5 7.4.7.4 1.5-.1 1.5-.9 0-.8.7-1.4 1.5-1.4h2c5.5 0 10-4.1 10-9.2C22 4.3 17.5 2 12 2Z"/>
            </svg>
          </div>
          <div class="toggle-content">
            <h4>Theme</h4>
            <p>Choose the app color theme. Outlaw is the current default.</p>
          </div>
        </div>
      </div>
      <div class="theme-options">
        <SegmentedControl
          options={themeOptions}
          value={appTheme}
          ariaLabel="App theme"
          density="compact"
          on:change={(event) => onSetAppTheme(event.detail as AppThemeId)}
        />
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
        <button class="danger-button" on:click={onRequestClearUserData} disabled={isClearingUserData}>
          {isClearingUserData ? 'Clearing...' : 'Clear Data'}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .danger-card {
    border-color: color-mix(in srgb, var(--md-sys-color-error) 35%, var(--md-sys-color-outline-variant));
  }

  .theme-card {
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
  }

  .theme-card .setting-header {
    margin-bottom: 0;
  }

  .theme-options {
    display: flex;
    justify-content: flex-start;
    padding-left: 38px;
  }

  .theme-options :global(.ui-segmented-button) {
    font-size: 0.6rem;
    letter-spacing: 0;
  }

  .dashboard-view-control :global(.ui-segmented-button) {
    font-size: 0.72rem;
    font-weight: 560;
    letter-spacing: 0;
  }

  .danger-button {
    flex-shrink: 0;
    padding: 7px 12px;
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

  @media (max-width: 768px) {
    :global(.ui-segmented) {
      width: 100%;
    }

    :global(.ui-segmented-button) {
      flex: 1;
    }

    .theme-options {
      padding-left: 0;
    }
  }
</style>
