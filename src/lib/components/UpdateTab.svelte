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
  let showDetailsModal = false;
  let detailsTitle = '';
  let detailsContent = '';

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

  function openVersionDetails(version: any) {
    detailsTitle = `v${version.version} — Details`;
    // Build HTML content combining all changes
    const parts: string[] = [];
    for (const change of version.changes) {
      const type = change.type ?? '';
      const desc = change.description ?? '';
      const det = change.details ?? '';
      parts.push(`<div><strong>${type}:</strong> ${desc}${det ? `<div style="margin-top:6px;color:var(--md-sys-color-on-surface-variant)">${det.replace(/\n/g,'<br/>')}</div>` : ''}</div>`);
    }
    detailsContent = parts.join('<hr style="border:none;border-top:1px solid rgba(0,0,0,0.06);margin:8px 0;"/>');
    showDetailsModal = true;
  }

  function closeDetailsModal() {
    showDetailsModal = false;
    detailsTitle = '';
    detailsContent = '';
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
              <button class="change-details" on:click={() => openVersionDetails(version)}>Click here for details</button>
            </div>
            <ul>
              {#each version.changes as change}
                <li class="change-row">
                  <span class={`bug-severity change-label type-${change.type.toLowerCase().replace(/\s+/g,'-')}`}>{change.type}</span>
                  <span class="change-desc">{change.description}</span>
                </li>
              {/each}
            </ul>
          </div>
        {/each}
      </div>
    {:else}
      <div class="empty-state">No changelog data available.</div>
    {/if}
  </div>

  {#if showDetailsModal}
    <div class="modal-overlay" role="dialog" aria-modal="true">
      <div class="modal-card">
        <h3>{detailsTitle}</h3>
        <div class="modal-details">
          <p>{@html detailsContent.replace(/\n/g, '<br/>')}</p>
        </div>
        <div class="modal-actions">
          <button class="button secondary" on:click={closeDetailsModal}>Close</button>
        </div>
      </div>
    </div>
  {/if}

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

  .change-row {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    padding: 0.35rem 0;
  }

  /* Base styles for the premium metallic/shiny look */
.bug-severity.change-label {
  font-weight: 700;
  padding: 0.35rem 0.75rem;
  border-radius: 999px;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  position: relative;
  overflow: hidden;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  min-width: 54px;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.4);
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);


  border: 1px solid rgba(0, 0, 0, 0.25);
}


.bug-severity.change-label::after {
  content: '';
  position: absolute;
  top: -50%;
  left: -60%;
  width: 200%;
  height: 200%;
  pointer-events: none;


  background: linear-gradient(
    115deg,
    rgba(255, 255, 255, 0) 0%,
    rgba(255, 255, 255, 0) 40%,
    rgba(255, 255, 255, 0.45) 43%, /* Knackige Lichtkante */
    rgba(255, 255, 255, 0.15) 45%,
    rgba(255, 255, 255, 0) 55%
  );
  transform: rotate(-10deg);
  transition: transform 0.5s ease;
}


.bug-severity.change-label:hover {
  transform: translateY(-1.5px);
}

.bug-severity.change-label:hover::after {
  transform: translate(15%, 5%) rotate(-10deg);
}

/* */

.bug-severity.change-label.type-fixed {
  background: linear-gradient(135deg, #10b981 0%, #047857 100%);
  /* Ein innerer Lichtrand, der nur oben hell leuchtet */
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.4),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 14px rgba(4, 120, 87, 0.4);
}
.bug-severity.change-label.type-fixed:hover {
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.55),
    0 6px 18px rgba(4, 120, 87, 0.55);
}

.bug-severity.change-label.type-added {
  background: linear-gradient(135deg, #3b82f6 0%, #1d4ed8 100%);
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.4),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 14px rgba(29, 78, 216, 0.4);
}
.bug-severity.change-label.type-added:hover {
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.55),
    0 6px 18px rgba(29, 78, 216, 0.55);
}

.bug-severity.change-label.type-improved {
  background: linear-gradient(135deg, #a78bfa 0%, #6d28d9 100%);
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.45),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 14px rgba(109, 40, 217, 0.4);
}
.bug-severity.change-label.type-improved:hover {
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.6),
    0 6px 18px rgba(109, 40, 217, 0.55);
}

.bug-severity.change-label.type-security {
  background: linear-gradient(135deg, #f87171 0%, #b91c1c 100%);
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.45),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 14px rgba(185, 28, 28, 0.45);
}
.bug-severity.change-label.type-security:hover {
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.6),
    0 6px 18px rgba(185, 28, 28, 0.6);
}

.bug-severity.change-label.type-removed,
.bug-severity.change-label.type-deprecated {
  background: linear-gradient(135deg, #9ca3af 0%, #4b5563 100%);
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.3),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 12px rgba(0, 0, 0, 0.25);
}
.bug-severity.change-label.type-removed:hover,
.bug-severity.change-label.type-deprecated:hover {
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.45),
    0 6px 16px rgba(0, 0, 0, 0.35);
}

  .bug-severity.change-label.type-security:hover {
    box-shadow:
      inset 0 1px 0 rgba(255,255,255,0.45),
      0 5px 16px rgba(220,38,38,0.35);
  }

  .change-desc {
    color: var(--md-sys-color-on-surface-variant);
    flex: 1;
  }

  .change-details {
    background: transparent;
    border: none;
    color: var(--md-sys-color-primary);
    font-weight: 600;
    cursor: pointer;
    padding: 0.25rem 0.5rem;
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

  .bug-severity.severity-no-priority {
    background: linear-gradient(135deg, #4b5563 0%, #374151 50%, #1f2937 100%);
    color: #d1d5db;
    border: 1px solid rgba(156, 163, 175, 0.28);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.12),
      0 2px 8px rgba(17, 24, 39, 0.22);
    opacity: 0.82;
  }

  .bug-severity.severity-no-priority:hover {
    transform: none;
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.12),
      0 2px 8px rgba(17, 24, 39, 0.22);
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
    max-height: min(720px, calc(100vh - 2rem));
    background: var(--md-sys-color-surface);
    border-radius: 20px;
    padding: 1.5rem;
    border: 1px solid var(--md-sys-color-outline);
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.18);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-card h3 {
    flex-shrink: 0;
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
    flex: 1 1 auto;
    overflow-y: auto;
    min-height: 0;
  }

  .modal-details p {
    margin: 0;
  }

  .modal-actions {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    flex-shrink: 0;
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
