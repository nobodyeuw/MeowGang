<script lang="ts">
  export let onOpenLogs: () => void;
  export let onSendReport: () => void;
  export let onClearLogs: () => void;
</script>

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
      <button class="log-button" on:click={onOpenLogs}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
          <polyline points="14 2 14 8 20 8"/>
          <line x1="16" y1="13" x2="8" y2="13"/>
          <line x1="16" y1="17" x2="8" y2="17"/>
          <polyline points="10 9 9 9 8 9"/>
        </svg>
        View Logs
      </button>

      <button class="log-button report" on:click={onSendReport}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/>
          <polyline points="7 10 12 15 17 10"/>
          <line x1="12" y1="15" x2="12" y2="3"/>
        </svg>
        Send Report
      </button>

      <button class="log-button clear" on:click={onClearLogs}>
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

<style>
  .logging-section {
    max-width: 600px;
  }

  .logging-actions {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
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
    padding: 10px 12px;
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
</style>
