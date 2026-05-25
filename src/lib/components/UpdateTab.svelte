<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { openUrl } from '@tauri-apps/plugin-opener';

  const RELEASES_URL = 'https://github.com/nobodyeuw/MeowGang/releases';

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
  $: knownIssueItems = knownBugs?.knownIssues || (knownBugs?.bugs || []).filter((item: any) => item.category !== 'coming_feature');
  $: comingFeatureItems = knownBugs?.comingFeatures || (knownBugs?.bugs || []).filter((item: any) => item.category === 'coming_feature');

  function normalizeLabel(value: string) {
    return String(value || '').trim().toLowerCase().replace(/\s+/g, '-');
  }

  function formatIssueSeverity(value: string) {
    const normalized = normalizeLabel(value);
    if (normalized === 'low-priority') return 'Low';
    if (normalized === 'no-priority') return 'Low';
    return value;
  }

  function formatFeaturePriority(value: string) {
    const normalized = normalizeLabel(value);
    if (normalized === 'long-term') return 'Long Term';
    if (normalized === 'no-priority') return 'No Prio';
    if (normalized === 'low-priority') return 'Low Prio';
    if (normalized === 'mid-priority' || normalized === 'medium-priority') return 'Mid Prio';
    if (normalized === 'high-priority') return 'High Prio';
    return value;
  }

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

  function escapeHtml(value: string) {
    return value
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  function renderInlineMarkdown(value: string) {
    return escapeHtml(value).replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  }

  function renderReleaseNotes(notes: string) {
    const lines = notes.replace(/\r\n/g, '\n').split('\n');
    const html: string[] = [];
    let listOpen = false;

    const closeList = () => {
      if (listOpen) {
        html.push('</ul>');
        listOpen = false;
      }
    };

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed) {
        closeList();
        continue;
      }

      if (trimmed.startsWith('### ')) {
        closeList();
        html.push(`<h3>${renderInlineMarkdown(trimmed.slice(4))}</h3>`);
      } else if (trimmed.startsWith('#### ')) {
        closeList();
        html.push(`<h4>${renderInlineMarkdown(trimmed.slice(5))}</h4>`);
      } else if (trimmed.startsWith('## ')) {
        closeList();
        html.push(`<h3>${renderInlineMarkdown(trimmed.slice(3))}</h3>`);
      } else if (trimmed.startsWith('- ')) {
        if (!listOpen) {
          html.push('<ul>');
          listOpen = true;
        }
        html.push(`<li>${renderInlineMarkdown(trimmed.slice(2))}</li>`);
      } else {
        closeList();
        html.push(`<p>${renderInlineMarkdown(trimmed)}</p>`);
      }
    }

    closeList();
    return html.join('');
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

  function openTrackerItemDetails(item: any, title: string) {
    detailsTitle = title;
    detailsContent = escapeHtml(item.details || item.description || 'No details available.');
    showDetailsModal = true;
  }

  async function openPreviousChangelogs() {
    try {
      await openUrl(RELEASES_URL);
    } catch (err) {
      console.warn('Failed to open release history:', err);
      window.open(RELEASES_URL, '_blank', 'noopener,noreferrer');
    }
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
            <div class="release-notes">{@html renderReleaseNotes(updateInfo)}</div>
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
      <p>View the current app version, available updates, coming changes, and known issues.</p>
    </div>
    <button type="button" class="release-history-link" on:click={openPreviousChangelogs}>
      Read previous changelogs here
    </button>
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

  <div class="updates-layout">
    <div class="changelog-card">
      <div class="section-header">
        <h3>Changelog</h3>
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
                <button class="change-details" on:click={() => openVersionDetails(version)}>Details</button>
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

    <div class="updates-side-column">
      <div class="bugs-card">
        <div class="section-header">
          <h3>Known Issues</h3>
        </div>

        {#if isChangelogLoading}
          <div class="loading-state">Loading known issues...</div>
        {:else if knownIssueItems.length}
          <ul class="bugs-list">
            {#each knownIssueItems as bug}
              <li>
                <span class={`bug-severity issue-severity severity-${normalizeLabel(formatIssueSeverity(bug.severity))}`}>{formatIssueSeverity(bug.severity)}</span>
                <span class="bug-description">{bug.description}</span>
                {#if bug.details}
                  <button class="change-details" on:click={() => openTrackerItemDetails(bug, 'Known Issue Details')}>Details</button>
                {/if}
              </li>
            {/each}
          </ul>
        {:else}
          <div class="empty-state">No known issues recorded.</div>
        {/if}
      </div>

      <div class="bugs-card">
        <div class="section-header">
          <h3>Coming Features</h3>
        </div>

        {#if isChangelogLoading}
          <div class="loading-state">Loading coming features...</div>
        {:else if comingFeatureItems.length}
          <ul class="bugs-list">
            {#each comingFeatureItems as feature}
              <li>
                <span class={`bug-severity feature-priority priority-${normalizeLabel(formatFeaturePriority(feature.priority || feature.severity))}`}>{formatFeaturePriority(feature.priority || feature.severity)}</span>
                <span class="bug-description">{feature.description}</span>
                {#if feature.details}
                  <button class="change-details" on:click={() => openTrackerItemDetails(feature, 'Coming Feature Details')}>Details</button>
                {/if}
              </li>
            {/each}
          </ul>
        {:else}
          <div class="empty-state">No coming features recorded.</div>
        {/if}
      </div>
    </div>
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

</div>

<style>
  .update-tab {
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
    max-width: 1280px;
    margin: 0 auto;
    width: 100%;
    padding: 0.75rem;
    font-size: 0.9rem;
  }

  .header-panel {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.75rem 0.85rem;
    background: var(--md-sys-color-surface-container-highest);
    border-radius: 12px;
    border: 1px solid var(--md-sys-color-outline);
  }

  .header-panel h2 {
    margin: 0 0 0.15rem;
    font-size: 1.12rem;
  }

  .header-panel p {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.82rem;
  }

  .status-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.55rem;
  }

  .status-card {
    padding: 0.6rem 0.7rem;
    border-radius: 12px;
    background: var(--md-sys-color-surface-container-highest);
    border: 1px solid var(--md-sys-color-outline);
  }

  .status-label {
    display: block;
    font-size: 0.7rem;
    color: var(--md-sys-color-on-surface-variant);
    margin-bottom: 0.3rem;
  }

  .status-value {
    font-size: 0.9rem;
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
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .button {
    padding: 0.55rem 0.8rem;
    border: none;
    border-radius: 10px;
    cursor: pointer;
    font-weight: 600;
    min-width: 132px;
    font-size: 0.82rem;
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

  .updates-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(300px, 1fr);
    gap: 0.65rem;
    align-items: start;
  }

  .updates-side-column {
    display: grid;
    gap: 0.65rem;
  }

  .changelog-card,
  .bugs-card {
    padding: 0.65rem;
    border-radius: 10px;
    background: var(--md-sys-color-surface-container-highest);
    border: 1px solid var(--md-sys-color-outline);
  }

  .section-header {
    margin-bottom: 0.45rem;
  }

  .section-header h3 {
    margin: 0;
    font-size: 0.92rem;
  }

  .section-header p {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
  }

  .changelog-list,
  .bugs-list {
    display: grid;
    gap: 0.42rem;
  }

  .changelog-entry {
    padding: 0.52rem;
    border-radius: 8px;
    background: var(--md-sys-color-surface);
    border: 1px solid var(--md-sys-color-outline);
  }

  .change-row {
    display: flex;
    gap: 0.45rem;
    align-items: center;
    padding: 0.18rem 0;
  }

  /* Base styles for the premium metallic/shiny look */
.bug-severity.change-label {
  font-weight: 700;
  padding: 0.22rem 0.48rem;
  border-radius: 999px;
  font-size: 0.58rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  position: relative;
  overflow: hidden;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  min-width: 42px;
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
.bug-severity.change-label.type-added {
  background: linear-gradient(135deg, #3b82f6 0%, #1d4ed8 100%);
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.4),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 14px rgba(29, 78, 216, 0.4);
}
.bug-severity.change-label.type-improved {
  background: linear-gradient(135deg, #38bdf8 0%, #0f766e 100%);
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.45),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 14px rgba(15, 118, 110, 0.3);
}
.bug-severity.change-label.type-changed {
  background: linear-gradient(135deg, #94a3b8 0%, #475569 100%);
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.34),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 12px rgba(71, 85, 105, 0.28);
}
.bug-severity.change-label.type-security {
  background: linear-gradient(135deg, #f87171 0%, #b91c1c 100%);
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.45),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 14px rgba(185, 28, 28, 0.45);
}
.bug-severity.change-label.type-removed,
.bug-severity.change-label.type-deprecated {
  background: linear-gradient(135deg, #9ca3af 0%, #4b5563 100%);
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.3),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 12px rgba(0, 0, 0, 0.25);
}
  .change-desc {
    color: var(--md-sys-color-on-surface-variant);
    flex: 1;
    font-size: 0.82rem;
    min-width: 0;
  }

  .change-details {
    background: transparent;
    border: none;
    color: var(--md-sys-color-primary);
    font-weight: 600;
    cursor: pointer;
    padding: 0.15rem 0.3rem;
    font-size: 0.76rem;
  }

  .entry-header {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    margin-bottom: 0.32rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .entry-version {
    font-weight: 700;
    font-size: 0.88rem;
  }

  .entry-date {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.76rem;
  }

  .bugs-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .bugs-list li {
    display: flex;
    gap: 0.45rem;
    align-items: flex-start;
    padding: 0.42rem 0.48rem;
    border-radius: 8px;
    border: 1px solid var(--md-sys-color-outline);
    background: var(--md-sys-color-surface);
  }

  .bug-severity {
    font-weight: 700;
    padding: 0.2rem 0.45rem;
    border-radius: 999px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 0.58rem;
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

  .issue-severity.severity-critical {
    background: linear-gradient(135deg, #f87171 0%, #dc2626 45%, #b91c1c 100%);
    color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.45);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.35),
      0 3px 12px rgba(220, 38, 38, 0.28);
  }

  .issue-severity.severity-high {
    background: linear-gradient(135deg, #fb923c 0%, #ea580c 48%, #c2410c 100%);
    color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.45);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.35),
      0 3px 12px rgba(234, 88, 12, 0.24);
  }

  .issue-severity.severity-moderate {
    background: rgba(245, 158, 11, 0.16);
    color: #b45309;
    border-color: rgba(245, 158, 11, 0.28);
  }

  .issue-severity.severity-minor {
    background: rgba(250, 204, 21, 0.16);
    color: #92400e;
    border-color: rgba(250, 204, 21, 0.28);
  }

  .issue-severity.severity-low {
    background: rgba(34, 197, 94, 0.16);
    color: #166534;
    border-color: rgba(34, 197, 94, 0.28);
  }

  .issue-severity.severity-info {
    background: rgba(59, 130, 246, 0.16);
    color: #1d4ed8;
    border-color: rgba(59, 130, 246, 0.28);
  }

  .feature-priority.priority-high-prio {
    background: rgba(239, 68, 68, 0.16);
    color: #b91c1c;
    border-color: rgba(239, 68, 68, 0.28);
  }

  .feature-priority.priority-mid-prio {
    background: rgba(245, 158, 11, 0.16);
    color: #b45309;
    border-color: rgba(245, 158, 11, 0.28);
  }

  .feature-priority.priority-low-prio {
    background: rgba(59, 130, 246, 0.15);
    color: #1d4ed8;
    border-color: rgba(59, 130, 246, 0.26);
  }

  .feature-priority.priority-long-term {
    background: rgba(20, 184, 166, 0.15);
    color: #0f766e;
    border-color: rgba(20, 184, 166, 0.28);
  }

  .feature-priority.priority-no-prio {
    background: rgba(100, 116, 139, 0.16);
    color: #64748b;
    border-color: rgba(100, 116, 139, 0.26);
  }

  .bug-description {
    color: var(--md-sys-color-on-surface-variant);
    flex: 1;
    min-width: 0;
    font-size: 0.8rem;
  }

  .empty-state,
  .loading-state {
    padding: 0.6rem;
    border-radius: 8px;
    font-size: 0.82rem;
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

  .release-notes {
    display: grid;
    gap: 0.65rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .release-notes :global(h3),
  .release-notes :global(h4),
  .release-notes :global(p),
  .release-notes :global(ul) {
    margin: 0;
  }

  .release-notes :global(h3) {
    color: var(--md-sys-color-on-surface);
    font-size: 1.05rem;
  }

  .release-notes :global(h4) {
    color: var(--md-sys-color-primary);
    font-size: 0.9rem;
    margin-top: 0.25rem;
  }

  .release-notes :global(ul) {
    padding-left: 1.1rem;
  }

  .release-notes :global(li) {
    margin: 0.25rem 0;
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

  .release-history-link {
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 999px;
    background: var(--md-sys-color-surface-container-highest);
    color: var(--md-sys-color-primary);
    cursor: pointer;
    font: inherit;
    font-size: 0.74rem;
    font-weight: 700;
    padding: 0.4rem 0.65rem;
    white-space: nowrap;
  }

  .release-history-link:hover {
    background: var(--md-sys-color-surface-variant);
  }

  @media (max-width: 980px) {
    .updates-layout {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 768px) {
    .update-tab {
      padding: 0.55rem;
    }

    .header-panel {
      align-items: flex-start;
      flex-direction: column;
    }

    .release-history-link {
      white-space: normal;
    }

    .status-grid {
      grid-template-columns: 1fr;
    }

    .actions-row {
      flex-direction: column;
    }

  }
</style>
