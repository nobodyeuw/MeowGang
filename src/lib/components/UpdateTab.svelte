<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  let currentVersion = '';
  let latestVersion: string | null = null;
  let updateAvailable = false;
  let isCheckingUpdate = false;
  let isInstallingUpdate = false;
  let updateInfo: string | null = null;
  let showUpdateModal = false;
  let changelogs: any = null;
  let knownBugs: any = null;
  let isChangelogLoading = false;
  let loadError = '';
  let installMessage = '';

  onMount(async () => {
    await loadAppVersion();
    await loadChangelogData();
    await checkForUpdates();

    if (updateAvailable) {
      showUpdateModal = true;
    }
  });

  async function loadAppVersion() {
    try {
      const version = await invoke('get_app_version');
      currentVersion = version as string;
    } catch (err) {
      console.warn('Failed to load app version:', err);
      currentVersion = 'Unknown';
    }
  }

  async function checkForUpdates() {
    try {
      isCheckingUpdate = true;
      const updateData: any = await invoke('check_for_updates');
      currentVersion = updateData.current_version;
      latestVersion = updateData.latest_version;
      updateAvailable = updateData.update_available;
      updateInfo = updateData.body ?? null;
      if (updateAvailable) {
        showUpdateModal = true;
      }
    } catch (err) {
      console.warn('Failed to check for updates:', err);
    } finally {
      isCheckingUpdate = false;
    }
  }

  async function installUpdate() {
    try {
      isInstallingUpdate = true;
      installMessage = '';
      const result = await invoke('install_update');
      installMessage = result as string;
    } catch (err) {
      installMessage = `Failed to install update: ${err}`;
    } finally {
      isInstallingUpdate = false;
    }
  }

  async function loadChangelogData() {
    try {
      isChangelogLoading = true;
      loadError = '';
      const [changelogData, bugsData] = await Promise.all([
        invoke('get_changelogs'),
        invoke('get_known_bugs')
      ]);
      changelogs = changelogData;
      knownBugs = bugsData;
    } catch (err) {
      loadError = `Failed to load changelog data: ${err}`;
      console.error(loadError);
    } finally {
      isChangelogLoading = false;
    }
  }

  function closeUpdateModal() {
    showUpdateModal = false;
  }

  function formatVersionTag(version: string) {
    return version.startsWith('v') ? version : `v${version}`;
  }
</script>

<div class="update-tab">
  {#if showUpdateModal}
    <div class="modal-overlay" role="dialog" aria-modal="true">
      <div class="modal-card">
        <h3>Update Available</h3>
        <p class="modal-message">A new version is available: <strong>{latestVersion}</strong></p>
        <p>You are currently running <strong>{currentVersion}</strong>.</p>
        {#if updateInfo}
          <div class="modal-details">
            <h4>Release Notes</h4>
            <p>{updateInfo}</p>
          </div>
        {/if}
        <div class="modal-actions">
          <button class="button primary" on:click={installUpdate} disabled={isInstallingUpdate}>
            {#if isInstallingUpdate}Installing...{:else}Install Update{/if}
          </button>
          <button class="button secondary" on:click={closeUpdateModal} disabled={isInstallingUpdate}>
            Dismiss
          </button>
        </div>
        {#if installMessage}
          <div class="install-message">{installMessage}</div>
        {/if}
      </div>
    </div>
  {/if}

  <div class="header-panel">
    <div>
      <h2>Updates & Changelog</h2>
      <p>View the current app version, available updates, and known issues.</p>
    </div>
  </div>

  <div class="status-grid">
    <div class="status-card">
      <span class="status-label">Current Version</span>
      <span class="status-value">{currentVersion}</span>
    </div>
    <div class="status-card">
      <span class="status-label">Latest Version</span>
      <span class="status-value">{latestVersion ?? currentVersion ?? 'Unknown'}</span>
    </div>
    <div class="status-card">
      <span class="status-label">Update Status</span>
      <span class="status-value {updateAvailable ? 'available' : 'current'}">
        {#if isCheckingUpdate}Checking...{:else if updateAvailable}Update available{:else}Up to date{/if}
      </span>
    </div>
  </div>

  <div class="actions-row">
    <button class="button primary" on:click={checkForUpdates} disabled={isCheckingUpdate}>
      {#if isCheckingUpdate}Checking...{:else}Check for Updates{/if}
    </button>
    {#if updateAvailable && latestVersion}
      <button class="button secondary" on:click={installUpdate} disabled={isInstallingUpdate}>
        {#if isInstallingUpdate}Installing...{:else}Install {formatVersionTag(latestVersion)}{/if}
      </button>
    {/if}
  </div>

  {#if loadError}
    <div class="alert error">{loadError}</div>
  {/if}

  <div class="changelog-card">
    <div class="section-header">
      <h3>Changelog</h3>
      <p>Latest releases and fixes from the tracker.</p>
    </div>

    {#if isChangelogLoading}
      <div class="loading-state">Loading changelog...</div>
    {:else if changelogs?.versions?.length}
      <div class="changelog-list">
        {#each changelogs.versions as version}
          <div class="changelog-entry">
            <div class="entry-header">
              <span class="entry-version">v{version.version}</span>
              <span class="entry-date">{version.date}</span>
            </div>
            <ul>
              {#each version.changes as change}
                <li>{change.type}: {change.description}</li>
              {/each}
            </ul>
          </div>
        {/each}
      </div>
    {:else}
      <div class="empty-state">No changelog data available.</div>
    {/if}
  </div>

  <div class="bugs-card">
    <div class="section-header">
      <h3>Known Issues</h3>
      <p>Open issues affecting the current tracker version.</p>
    </div>

    {#if isChangelogLoading}
      <div class="loading-state">Loading known issues...</div>
    {:else if knownBugs?.bugs?.length}
      <ul class="bugs-list">
        {#each knownBugs.bugs as bug}
          <li>
            <span class={`bug-severity severity-${bug.severity.toLowerCase().replace(/\s+/g, '-')}`}>{bug.severity}</span>
            <span class="bug-description">{bug.description}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <div class="empty-state">No known bugs recorded.</div>
    {/if}
  </div>
</div>

<style>
  .update-tab {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    max-width: 1280px;
    margin: 0 auto;
    width: 100%;
    padding: 1rem;
  }

  .header-panel {
    padding: 1.25rem;
    background: var(--md-sys-color-surface-container-highest);
    border-radius: 16px;
    border: 1px solid var(--md-sys-color-outline);
  }

  .header-panel h2 {
    margin: 0 0 0.25rem;
    font-size: 1.5rem;
  }

  .header-panel p {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
  }

  .status-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1rem;
  }

  .status-card {
    padding: 1rem;
    border-radius: 16px;
    background: var(--md-sys-color-surface-container-highest);
    border: 1px solid var(--md-sys-color-outline);
  }

  .status-label {
    display: block;
    font-size: 0.9rem;
    color: var(--md-sys-color-on-surface-variant);
    margin-bottom: 0.5rem;
  }

  .status-value {
    font-size: 1.15rem;
    font-weight: 600;
  }

  .status-value.available {
    color: var(--md-sys-color-warning);
  }

  .status-value.current {
    color: var(--md-sys-color-secondary);
  }

  .actions-row {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .button {
    padding: 0.9rem 1.25rem;
    border: none;
    border-radius: 12px;
    cursor: pointer;
    font-weight: 600;
    min-width: 180px;
  }

  .button.primary {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
  }

  .button.secondary {
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface);
  }

  .button:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .alert.error {
    color: var(--md-sys-color-error);
    background: rgba(255, 0, 0, 0.08);
    border: 1px solid rgba(255, 0, 0, 0.2);
    border-radius: 12px;
    padding: 0.9rem 1rem;
  }

  .changelog-card,
  .bugs-card {
    padding: 1rem;
    border-radius: 16px;
    background: var(--md-sys-color-surface-container-highest);
    border: 1px solid var(--md-sys-color-outline);
  }

  .section-header {
    margin-bottom: 1rem;
  }

  .section-header h3 {
    margin: 0 0 0.25rem;
  }

  .section-header p {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
  }

  .changelog-list,
  .bugs-list {
    display: grid;
    gap: 0.75rem;
  }

  .changelog-entry {
    padding: 0.9rem;
    border-radius: 14px;
    background: var(--md-sys-color-surface);
    border: 1px solid var(--md-sys-color-outline);
  }

  .entry-header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.75rem;
  }

  .entry-version {
    font-weight: 700;
  }

  .entry-date {
    color: var(--md-sys-color-on-surface-variant);
  }

  .bugs-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .bugs-list li {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    padding: 0.65rem 0.75rem;
    border-radius: 14px;
    border: 1px solid var(--md-sys-color-outline);
    background: var(--md-sys-color-surface);
  }

  .bug-severity {
    font-weight: 700;
    padding: 0.3rem 0.75rem;
    border-radius: 999px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 0.75rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: auto;
    position: relative;
    overflow: hidden;
    transition: transform 0.2s ease, box-shadow 0.2s ease, background 0.2s ease;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  }

  .bug-severity::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: linear-gradient(135deg, rgba(255,255,255,0.24), rgba(255,255,255,0.04) 25%, rgba(255,255,255,0) 45%);
    opacity: 0.55;
    pointer-events: none;
    transform: translateY(-10%) skewY(-3deg);
  }

  .bug-severity.severity-critical {
    background: linear-gradient(135deg, #f87171 0%, #dc2626 45%, #b91c1c 100%);
    color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.45);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.35),
      0 3px 12px rgba(220, 38, 38, 0.28);
  }

  .bug-severity.severity-critical:hover {
    transform: translateY(-1px);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.45),
      0 5px 16px rgba(220, 38, 38, 0.35);
  }

  .bug-severity.severity-low-priority {
    background: linear-gradient(135deg, #fde68a 0%, #f59e0b 45%, #d97706 100%);
    color: #1f2937;
    border: 1px solid rgba(255, 255, 255, 0.45);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.35),
      0 3px 12px rgba(245, 158, 11, 0.24);
  }

  .bug-severity.severity-low-priority:hover {
    transform: translateY(-1px);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.45),
      0 5px 16px rgba(245, 158, 11, 0.32);
  }

  .bug-severity.severity-moderate {
    background: rgba(245, 158, 11, 0.16);
    color: #b45309;
    border-color: rgba(245, 158, 11, 0.28);
  }

  .bug-severity.severity-minor {
    background: rgba(250, 204, 21, 0.16);
    color: #92400e;
    border-color: rgba(250, 204, 21, 0.28);
  }

  .bug-severity.severity-low {
    background: rgba(34, 197, 94, 0.16);
    color: #166534;
    border-color: rgba(34, 197, 94, 0.28);
  }

  .bug-severity.severity-info {
    background: rgba(59, 130, 246, 0.16);
    color: #1d4ed8;
    border-color: rgba(59, 130, 246, 0.28);
  }

  .bug-description {
    color: var(--md-sys-color-on-surface-variant);
    flex: 1;
  }

  .empty-state,
  .loading-state {
    padding: 1rem;
    border-radius: 14px;
    background: var(--md-sys-color-surface);
    border: 1px dashed var(--md-sys-color-outline);
    color: var(--md-sys-color-on-surface-variant);
  }

  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10000;
    padding: 1rem;
  }

  .modal-card {
    width: min(560px, 100%);
    background: var(--md-sys-color-surface);
    border-radius: 20px;
    padding: 1.5rem;
    border: 1px solid var(--md-sys-color-outline);
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.18);
  }

  .modal-card h3 {
    margin: 0 0 0.75rem;
  }

  .modal-message {
    margin: 0 1rem 1rem 0;
    color: var(--md-sys-color-on-surface-variant);
  }

  .modal-details {
    margin: 1rem 0;
    padding: 1rem;
    border-radius: 14px;
    background: var(--md-sys-color-surface-container-highest);
    border: 1px solid var(--md-sys-color-outline);
  }

  .modal-actions {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .install-message {
    margin-top: 1rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  @media (max-width: 768px) {
    .status-grid {
      grid-template-columns: 1fr;
    }

    .actions-row {
      flex-direction: column;
    }
  }
</style>