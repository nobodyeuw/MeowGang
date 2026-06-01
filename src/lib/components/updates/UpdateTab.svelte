<script lang="ts">
  import { onMount } from 'svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import {
    buildVersionDetailsHtml,
    escapeHtml,
    formatChangelogDate,
    formatFeaturePriority,
    formatIssueSeverity,
    formatVersionTag,
    normalizeLabel,
    renderReleaseNotes
  } from '$lib/components/updates/helpers';
  import type { ChangelogData, ChangelogVersion, KnownBugsData, TrackerItem } from '$lib/components/updates/types';
  import { checkForAppUpdates, getAppVersion, installAppUpdate, loadUpdateResources } from '$lib/services/updates';

  const RELEASES_URL = 'https://github.com/nobodyeuw/MeowGang/releases';

  let currentVersion = '';
  let latestVersion: string | null = null;
  let updateAvailable = false;
  let isCheckingUpdate = false;
  let isInstallingUpdate = false;
  let updateInfo: string | null = null;
  let showUpdateModal = false;
  let changelogs: ChangelogData | null = null;
  let knownBugs: KnownBugsData | null = null;
  let isChangelogLoading = false;
  let loadError = '';
  let installMessage = '';
  let showDetailsModal = false;
  let detailsTitle = '';
  let detailsContent = '';
  $: knownIssueItems = knownBugs?.knownIssues || (knownBugs?.bugs || []).filter((item) => item.category !== 'coming_feature');
  $: comingFeatureItems = knownBugs?.comingFeatures || (knownBugs?.bugs || []).filter((item) => item.category === 'coming_feature');

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
      currentVersion = await getAppVersion();
    } catch (err) {
      console.warn('Failed to load app version:', err);
      currentVersion = 'Unknown';
    }
  }

  async function checkForUpdates() {
    try {
      isCheckingUpdate = true;
      const updateData = await checkForAppUpdates();
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
      installMessage = await installAppUpdate();
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
      const resources = await loadUpdateResources();
      changelogs = resources.changelogs;
      knownBugs = resources.knownBugs;
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

  function openVersionDetails(version: ChangelogVersion) {
    detailsTitle = `v${version.version} - Details`;
    detailsContent = buildVersionDetailsHtml(version);
    showDetailsModal = true;
  }
  function closeDetailsModal() {
    showDetailsModal = false;
    detailsTitle = '';
    detailsContent = '';
  }

  function openTrackerItemDetails(item: TrackerItem, title: string) {
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

<div class="update-tab" data-guide="updates">
  {#if showUpdateModal}
    <div class="ui-modal-overlay" role="dialog" aria-modal="true">
      <div class="ui-modal-card modal-card">
        <h3 class="ui-modal-title">Update Available</h3>
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
        <div class="ui-loading-state update-state">Loading changelog...</div>
      {:else if changelogs?.versions?.length}
        <div class="changelog-list">
          {#each changelogs.versions as version}
            <div class="changelog-entry">
              <div class="entry-header">
                <span class="entry-version">v{version.version}</span>
                <span class="entry-date">{formatChangelogDate(version.date)}</span>
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
        <div class="ui-empty-state update-state">No changelog data available.</div>
      {/if}
    </div>

    <div class="updates-side-column">
      <div class="bugs-card">
        <div class="section-header">
          <h3>Known Issues</h3>
        </div>

        {#if isChangelogLoading}
          <div class="ui-loading-state update-state">Loading known issues...</div>
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
          <div class="ui-empty-state update-state">No known issues recorded.</div>
        {/if}
      </div>

      <div class="bugs-card">
        <div class="section-header">
          <h3>Coming Features</h3>
        </div>

        {#if isChangelogLoading}
          <div class="ui-loading-state update-state">Loading coming features...</div>
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
          <div class="ui-empty-state update-state">No coming features recorded.</div>
        {/if}
      </div>
    </div>
  </div>

  {#if showDetailsModal}
    <div class="ui-modal-overlay" role="dialog" aria-modal="true">
      <div class="ui-modal-card modal-card">
        <h3 class="ui-modal-title">{detailsTitle}</h3>
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
    background: color-mix(in srgb, var(--md-sys-color-error) 8%, transparent);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-error) 20%, transparent);
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
  background: var(--app-color-success-gradient);
  /* Ein innerer Lichtrand, der nur oben hell leuchtet */
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.4),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 14px color-mix(in srgb, var(--md-sys-color-success) 35%, transparent);
}
.bug-severity.change-label.type-added {
  background: linear-gradient(135deg, var(--app-color-tracked) 0%, color-mix(in srgb, var(--app-color-tracked) 70%, black) 100%);
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.4),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 14px color-mix(in srgb, var(--app-color-tracked) 34%, transparent);
}
.bug-severity.change-label.type-improved {
  background: linear-gradient(135deg, var(--app-color-tracked) 0%, var(--app-color-tracked-alt) 100%);
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.45),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 14px color-mix(in srgb, var(--app-color-tracked-alt) 30%, transparent);
}
.bug-severity.change-label.type-changed {
  background: linear-gradient(135deg, var(--app-color-muted-state) 0%, color-mix(in srgb, var(--app-color-muted-state) 62%, black) 100%);
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.34),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 12px color-mix(in srgb, var(--app-color-muted-state) 28%, transparent);
}
.bug-severity.change-label.type-security {
  background: var(--app-color-error-gradient);
  box-shadow:
    inset 0 1.5px 0 rgba(255, 255, 255, 0.45),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15),
    0 4px 14px color-mix(in srgb, var(--md-sys-color-error) 45%, transparent);
}
.bug-severity.change-label.type-removed,
.bug-severity.change-label.type-deprecated {
  background: linear-gradient(135deg, var(--app-color-muted-state) 0%, color-mix(in srgb, var(--app-color-muted-state) 56%, black) 100%);
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
    background: var(--app-color-error-gradient);
    color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.45);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.35),
      0 3px 12px color-mix(in srgb, var(--md-sys-color-error) 28%, transparent);
  }

  .issue-severity.severity-high {
    background: var(--app-color-warning-gradient);
    color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.45);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.35),
      0 3px 12px color-mix(in srgb, var(--md-sys-color-warning) 24%, transparent);
  }

  .issue-severity.severity-moderate {
    background: color-mix(in srgb, var(--md-sys-color-warning) 16%, transparent);
    color: var(--md-sys-color-warning);
    border-color: color-mix(in srgb, var(--md-sys-color-warning) 28%, transparent);
  }

  .issue-severity.severity-minor {
    background: color-mix(in srgb, var(--app-color-gold) 16%, transparent);
    color: var(--app-color-gold);
    border-color: color-mix(in srgb, var(--app-color-gold) 28%, transparent);
  }

  .issue-severity.severity-low {
    background: color-mix(in srgb, var(--md-sys-color-success) 16%, transparent);
    color: var(--md-sys-color-success);
    border-color: color-mix(in srgb, var(--md-sys-color-success) 28%, transparent);
  }

  .issue-severity.severity-info {
    background: color-mix(in srgb, var(--app-color-tracked) 16%, transparent);
    color: var(--app-color-tracked);
    border-color: color-mix(in srgb, var(--app-color-tracked) 28%, transparent);
  }

  .feature-priority.priority-high-prio {
    background: color-mix(in srgb, var(--md-sys-color-error) 16%, transparent);
    color: var(--md-sys-color-error);
    border-color: color-mix(in srgb, var(--md-sys-color-error) 28%, transparent);
  }

  .feature-priority.priority-mid-prio {
    background: color-mix(in srgb, var(--md-sys-color-warning) 16%, transparent);
    color: var(--md-sys-color-warning);
    border-color: color-mix(in srgb, var(--md-sys-color-warning) 28%, transparent);
  }

  .feature-priority.priority-low-prio {
    background: color-mix(in srgb, var(--app-color-tracked) 15%, transparent);
    color: var(--app-color-tracked);
    border-color: color-mix(in srgb, var(--app-color-tracked) 26%, transparent);
  }

  .feature-priority.priority-long-term {
    background: color-mix(in srgb, var(--app-color-tracked-alt) 15%, transparent);
    color: var(--app-color-tracked-alt);
    border-color: color-mix(in srgb, var(--app-color-tracked-alt) 28%, transparent);
  }

  .feature-priority.priority-no-prio {
    background: color-mix(in srgb, var(--app-color-muted-state) 16%, transparent);
    color: var(--app-color-muted-state);
    border-color: color-mix(in srgb, var(--app-color-muted-state) 26%, transparent);
  }

  .bug-description {
    color: var(--md-sys-color-on-surface-variant);
    flex: 1;
    min-width: 0;
    font-size: 0.8rem;
  }

  .update-state {
    padding: 0.6rem;
  }

  .modal-card {
    width: min(560px, 100%);
    border-radius: 20px;
    padding: 1.5rem;
  }

  .modal-card .ui-modal-title {
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
