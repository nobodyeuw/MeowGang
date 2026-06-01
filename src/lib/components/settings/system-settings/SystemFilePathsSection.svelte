<script lang="ts">
  import { formatPath, isPathValid } from './helpers';

  export let encountersDbPath = '';
  export let lostArkExePath = '';
  export let loaLogsExePath = '';
  export let onBrowseEncountersDb: () => void;
  export let onBrowseLostArkExe: () => void;
  export let onBrowseLoaLogsExe: () => void;
</script>

<div class="settings-section" data-guide="system-file-paths">
  <div class="section-header">
    <div class="section-icon">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
      </svg>
    </div>
    <div>
      <h3>File Paths</h3>
      <p>Configure paths to external files and databases</p>
    </div>
  </div>

  <div class="settings-grid">
    <div class="setting-card">
      <div class="setting-header">
        <div class="setting-icon database">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <ellipse cx="12" cy="5" rx="9" ry="3"/>
            <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/>
            <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>
          </svg>
        </div>
        <div>
          <h4>encounters.db</h4>
          <p>LOA Logs combat database for auto-completion</p>
        </div>
      </div>

      <div class="path-input-group">
        <div class="path-display" class:valid={isPathValid(encountersDbPath)} class:invalid={!isPathValid(encountersDbPath)}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M13 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V9z"/>
            <polyline points="13 2 13 9 20 9"/>
          </svg>
          <span class="path-text">{formatPath(encountersDbPath)}</span>
        </div>
        <button class="browse-button" on:click={onBrowseEncountersDb}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"/>
            <polyline points="9 22 9 12 15 12 15 22"/>
          </svg>
          Browse
        </button>
      </div>

      {#if isPathValid(encountersDbPath)}
        <div class="status-indicator success">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
          Database connected
        </div>
      {:else}
        <div class="status-indicator warning">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          No database selected
        </div>
      {/if}
    </div>

    <div class="setting-card">
      <div class="setting-header">
        <div class="setting-icon executable">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
            <line x1="12" y1="18" x2="12" y2="12"/>
            <line x1="9" y1="15" x2="15" y2="15"/>
          </svg>
        </div>
        <div>
          <h4>LostArk.exe</h4>
          <p>Lost Ark game executable for auto-launch</p>
        </div>
      </div>

      <div class="path-input-group">
        <div class="path-display" class:valid={isPathValid(lostArkExePath)} class:invalid={!isPathValid(lostArkExePath)}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
          </svg>
          <span class="path-text">{formatPath(lostArkExePath)}</span>
        </div>
        <button class="browse-button" on:click={onBrowseLostArkExe}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"/>
            <polyline points="9 22 9 12 15 12 15 22"/>
          </svg>
          Browse
        </button>
      </div>

      {#if isPathValid(lostArkExePath)}
        <div class="status-indicator success">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
          Executable found
        </div>
      {:else}
        <div class="status-indicator warning">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          No executable selected
        </div>
      {/if}
    </div>

    <div class="setting-card">
      <div class="setting-header">
        <div class="setting-icon executable">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
          </svg>
        </div>
        <div>
          <h4>LOA Logs.exe</h4>
          <p>LOA Logs executable to auto-launch when LOA Tracker starts</p>
        </div>
      </div>

      <div class="path-input-group">
        <div class="path-display" class:valid={isPathValid(loaLogsExePath)} class:invalid={!isPathValid(loaLogsExePath)}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
          </svg>
          <span class="path-text">{formatPath(loaLogsExePath)}</span>
        </div>
        <button class="browse-button" on:click={onBrowseLoaLogsExe}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"/>
            <polyline points="9 22 9 12 15 12 15 22"/>
          </svg>
          Browse
        </button>
      </div>

      {#if isPathValid(loaLogsExePath)}
        <div class="status-indicator success">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
          Executable found
        </div>
      {:else}
        <div class="status-indicator warning">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          No executable selected
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .path-input-group {
    display: flex;
    gap: 8px;
    margin-bottom: 6px;
    align-items: stretch;
    min-height: 38px;
  }

  .path-display {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface-variant);
    font-size: 13px;
    color: var(--md-sys-color-on-surface-variant);
    transition: all 0.2s ease;
    min-width: 0;
  }

  .path-display.valid {
    border-color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 10%, transparent);
    color: var(--md-sys-color-on-surface);
  }

  .path-display.invalid {
    border-color: var(--md-sys-color-error);
    background: color-mix(in srgb, var(--md-sys-color-error) 10%, transparent);
  }

  .path-text {
    font-family: 'Consolas', 'Monaco', monospace;
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .browse-button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.2s ease;
    white-space: nowrap;
    flex-shrink: 0;
    min-width: fit-content;
  }

  .browse-button:hover {
    background: color-mix(in srgb, var(--md-sys-color-primary) 90%, black);
    transform: translateY(-1px);
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 10px;
    font-size: 12px;
    font-weight: 500;
    border-radius: 6px;
    margin-top: 6px;
  }

  .status-indicator.success svg {
    color: var(--md-sys-color-success);
  }

  .status-indicator.warning svg {
    color: var(--md-sys-color-error);
  }

  .status-indicator.success,
  .status-indicator.warning {
    color: var(--md-sys-color-on-surface);
    background: color-mix(in srgb, var(--md-sys-color-surface-variant) 50%, transparent);
  }

  @media (max-width: 768px) {
    .path-input-group {
      flex-direction: column;
      min-height: auto;
    }

    .browse-button {
      width: 100%;
      justify-content: center;
    }
  }
</style>
