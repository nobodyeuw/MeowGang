<script lang="ts">
  export let loaLogsReminderMessage = '';
  export let updateAvailable = false;
  export let latestAppVersion: string | null = '';
  export let currentAppVersion: string | null = '';
  export let isUpdateChecking = false;
  export let dismissLoaLogsReminder: () => void;
  export let switchTab: (tab: string) => void;
  export let checkForAppUpdates: () => void | Promise<void>;
</script>

{#if loaLogsReminderMessage}
  <div class="app-alert loa-logs-reminder">
    <div class="alert-copy">
      <strong>LOA Logs:</strong> {loaLogsReminderMessage}
    </div>
    <button class="banner-button secondary" type="button" on:click={dismissLoaLogsReminder}>Dismiss</button>
  </div>
{/if}

{#if updateAvailable}
  <div class="app-alert update-banner">
    <div class="alert-copy">
      <strong>Update available:</strong> version {latestAppVersion} is ready. Current version: {currentAppVersion}.
    </div>
    <div class="banner-actions">
      <button class="banner-button" on:click={() => switchTab('updates')}>View updates</button>
      <button class="banner-button secondary" on:click={checkForAppUpdates} disabled={isUpdateChecking}>
        {isUpdateChecking ? 'Refreshing...' : 'Refresh'}
      </button>
    </div>
  </div>
{/if}

<style>
  .app-alert {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.75rem;
    align-items: center;
    padding: 0.9rem 1rem;
    margin-top: 0.75rem;
    border-radius: 16px;
    color: var(--md-sys-color-on-surface);
  }

  .update-banner {
    background: color-mix(in srgb, var(--md-sys-color-warning) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-warning) 25%, transparent);
  }

  .loa-logs-reminder {
    background: color-mix(in srgb, var(--md-sys-color-primary) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 24%, transparent);
  }

  .alert-copy {
    font-size: 0.95rem;
  }

  .banner-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .banner-button {
    border: none;
    border-radius: 12px;
    padding: 0.65rem 1rem;
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font-weight: 600;
  }

  .banner-button.secondary {
    background: var(--app-color-subtle-scrim);
  }

  .banner-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
