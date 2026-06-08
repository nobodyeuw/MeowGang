<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { MeowConnectFriendConnection } from '$lib/services/meow-connect';
  import type { MeowConnectHeaderState, MeowConnectSection } from '$lib/types/app-shell';
  import AppHeaderAlerts from './AppHeaderAlerts.svelte';
  import AppHeaderSeaCoinReminder from './AppHeaderSeaCoinReminder.svelte';
  import AppHeaderStatus from './AppHeaderStatus.svelte';
  import AppHeaderSubTabs from './AppHeaderSubTabs.svelte';
  import AppWindowControls from './AppWindowControls.svelte';

  export let activeTab = 'dashboard';
  export let activeSettingsTab = 'roster';
  export let activeMeowConnectTab: MeowConnectSection = 'together';
  export let meowConnectFeatureEnabled = true;
  export let showHeaderCountdown = true;
  export let resetCountdown = '';
  export let meowConnectHeaderState: MeowConnectHeaderState = 'inactive';
  export let meowConnectHeaderMessage = '';
  export let meowConnectHeaderLabel = 'Inactive';
  export let pendingMeowConnectRequests = 0;
  export let pendingMeowConnectFriendRequests: MeowConnectFriendConnection[] = [];
  export let showSetupGuideButton = true;
  export let headerContent = '';
  export let loaLogsReminderMessage = '';
  export let updateAvailable = false;
  export let latestAppVersion: string | null = '';
  export let currentAppVersion: string | null = '';
  export let isUpdateChecking = false;
  export let toggleSidebar: () => void;
  export let switchTab: (tab: string) => void;
  export let openMeowConnectRequests: () => void;
  export let startSetupGuide: () => void;
  export let dismissLoaLogsReminder: () => void;
  export let checkForAppUpdates: () => void | Promise<void>;

  const appWindow = getCurrentWindow();

  function startWindowDrag(event: MouseEvent) {
    if (event.button !== 0) return;

    const target = event.target as HTMLElement | null;
    if (target?.closest('button, a, input, select, textarea, [role="button"], .no-window-drag')) {
      return;
    }

    appWindow.startDragging().catch((error) => {
      console.warn('Failed to start window drag:', error);
    });
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions - the custom titlebar uses mousedown to start native window dragging. -->
<header class="header" data-tauri-drag-region on:mousedown={startWindowDrag}>
  <button class="menu-toggle" on:click={toggleSidebar} aria-label="Toggle menu">
    <span class="hamburger"></span>
  </button>

  <div class="header-title" data-tauri-drag-region>
    <AppHeaderStatus
      {activeTab}
      {meowConnectFeatureEnabled}
      {showHeaderCountdown}
      {resetCountdown}
      {meowConnectHeaderState}
      {meowConnectHeaderMessage}
      {meowConnectHeaderLabel}
      {pendingMeowConnectRequests}
      {pendingMeowConnectFriendRequests}
      {showSetupGuideButton}
      {switchTab}
      {openMeowConnectRequests}
      {startSetupGuide}
    />

    {#if headerContent}
      <div class="header-info">{headerContent}</div>
    {/if}

    <AppHeaderAlerts
      {loaLogsReminderMessage}
      {updateAvailable}
      {latestAppVersion}
      {currentAppVersion}
      {isUpdateChecking}
      {dismissLoaLogsReminder}
      {switchTab}
      {checkForAppUpdates}
    />
  </div>

  <div class="header-center-tools">
    <AppHeaderSeaCoinReminder />
  </div>

  <AppHeaderSubTabs
    {activeTab}
    bind:activeSettingsTab
    bind:activeMeowConnectTab
    {meowConnectFeatureEnabled}
    {pendingMeowConnectRequests}
  />

  <AppWindowControls />
</header>

<style>
  .header {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.42rem 1.25rem;
    background: var(--md-sys-color-surface);
    border-bottom: 1px solid var(--md-sys-color-outline);
    box-shadow: var(--app-shadow-sm);
    position: sticky;
    top: 0;
    z-index: 100;
    user-select: none;
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

  .header-center-tools {
    position: absolute;
    left: 50%;
    top: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    transform: translate(-50%, -50%);
    pointer-events: none;
  }

  .header-center-tools :global(.sea-coin-reminder-wrap) {
    pointer-events: auto;
  }

  .header-info {
    font-size: 0.875rem;
    color: var(--md-sys-color-on-surface-variant);
    font-weight: 500;
  }

  @media (max-width: 768px) {
    .header {
      padding: 0.38rem 0.9rem;
    }
  }
</style>
