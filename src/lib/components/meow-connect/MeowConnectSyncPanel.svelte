<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let resetText = 'No local snapshot loaded.';
  export let syncLabel = 'Sync now';
  export let syncTitle = 'Upload your current MeowConnect snapshot';
  export let syncDisabled = false;
  export let connectedCharacterCount = 0;
  export let unsyncedRosterChangeCount = '0';
  export let hasUnsyncedChanges = false;
  export let lastSyncValue = 'Never';
  export let lastSyncCaption = 'last synced';

  const dispatch = createEventDispatcher<{ sync: void }>();
</script>

<article class="settings-panel compact-panel" data-guide="meow-connect-sync">
  <div class="panel-title">
    <div>
      <h3>Sync</h3>
      <p>{resetText}</p>
    </div>
    <button
      class="primary-button"
      type="button"
      on:click={() => dispatch('sync')}
      disabled={syncDisabled}
      title={syncTitle}
    >
      {syncLabel}
    </button>
  </div>

  <div class="sync-status-grid">
    <div class="sync-status-item">
      <strong>{connectedCharacterCount}</strong>
      <span>characters marked connected in roster settings</span>
    </div>
    <div class:dirty={hasUnsyncedChanges} class="sync-status-item">
      <strong>{unsyncedRosterChangeCount}</strong>
      <span>unsynced roster changes</span>
    </div>
    <div class="sync-status-item">
      <strong>{lastSyncValue}</strong>
      <span>{lastSyncCaption}</span>
    </div>
  </div>
</article>

<style>
  .settings-panel {
    display: grid;
    align-content: start;
    gap: 0.75rem;
    padding: 0.85rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 12px;
    background: var(--md-sys-color-surface);
    transition: border-color 0.18s ease, box-shadow 0.18s ease;
  }

  .settings-panel:hover {
    border-color: color-mix(in srgb, var(--md-sys-color-primary) 65%, var(--md-sys-color-outline-variant));
    box-shadow: 0 2px 8px color-mix(in srgb, var(--md-sys-color-primary) 16%, transparent);
  }

  .panel-title {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
  }

  h3,
  p {
    margin: 0;
  }

  h3 {
    margin-bottom: 0.12rem;
    color: var(--md-sys-color-on-surface);
    font-size: 0.94rem;
    font-weight: 600;
  }

  p {
    max-width: 60rem;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.76rem;
    line-height: 1.35;
  }

  .primary-button {
    padding: 0.42rem 0.58rem;
    border: 0;
    border-radius: 8px;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    font: inherit;
    font-size: 0.76rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.18s ease, color 0.18s ease, border-color 0.18s ease;
  }

  .primary-button:disabled {
    cursor: default;
    opacity: 0.6;
  }

  .sync-status-grid {
    display: grid;
    gap: 0.42rem;
  }

  .sync-status-item {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: baseline;
    gap: 0.45rem;
    padding: 0.36rem 0.45rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 8px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container) 70%, transparent);
  }

  .sync-status-item strong {
    color: var(--md-sys-color-primary);
    font-size: 0.78rem;
    font-weight: 700;
    white-space: nowrap;
  }

  .sync-status-item span {
    min-width: 0;
    overflow: hidden;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sync-status-item.dirty {
    border-color: color-mix(in srgb, var(--md-sys-color-error) 38%, var(--md-sys-color-outline-variant));
    background: color-mix(in srgb, var(--md-sys-color-error) 8%, var(--md-sys-color-surface));
  }

  .sync-status-item.dirty strong {
    color: var(--md-sys-color-error);
  }
</style>
