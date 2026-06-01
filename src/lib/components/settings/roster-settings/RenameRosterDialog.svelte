<script lang="ts">
  export let rosterName = '';
  export let isLoading = false;
  export let onCancel: () => void;
  export let onRename: () => void;

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      event.stopPropagation();
      onRename();
    }
  }
</script>

<div class="dialog-overlay" role="presentation">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="dialog" on:click|stopPropagation role="dialog" tabindex="-1">
    <div class="dialog-header">
      <h3>Rename Roster</h3>
      <button class="close-button" on:click={onCancel}>x</button>
    </div>
    <div class="dialog-content">
      <div class="form-group">
        <label for="rename-roster-name">Roster Name</label>
        <input
          id="rename-roster-name"
          type="text"
          bind:value={rosterName}
          placeholder="Enter new roster name"
          on:keydown={handleKeydown}
          disabled={isLoading}
        />
      </div>
      {#if isLoading}
        <div class="loading-indicator">
          <p>Renaming roster...</p>
        </div>
      {/if}
    </div>
    <div class="dialog-actions">
      <button class="button secondary" on:click={onCancel} disabled={isLoading}>Cancel</button>
      <button class="button primary" on:click={onRename} disabled={isLoading || !rosterName.trim()}>
        {isLoading ? 'Renaming...' : 'Rename'}
      </button>
    </div>
  </div>
</div>

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: var(--app-color-modal-backdrop);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: var(--md-sys-color-surface);
    border: 2px solid var(--md-sys-color-outline);
    border-radius: 16px;
    padding: 0;
    min-width: 400px;
    max-width: 90vw;
    max-height: 90vh;
    overflow: hidden;
    box-shadow: var(--app-shadow-md);
  }

  .dialog-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.5rem;
    border-bottom: 2px solid var(--md-sys-color-outline);
    background: var(--md-sys-color-surface-variant);
  }

  .dialog-header h3 {
    margin: 0;
    color: var(--md-sys-color-primary);
    font-size: 1.3rem;
    font-weight: 600;
  }

  .close-button {
    background: none;
    border: none;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 1.5rem;
    cursor: pointer;
    padding: 0;
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    transition: all 0.3s ease;
  }

  .close-button:hover {
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface);
  }

  .dialog-content {
    padding: 1.5rem;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  .form-group label {
    display: block;
    margin-bottom: 0.5rem;
    color: var(--md-sys-color-on-surface);
    font-weight: 600;
  }

  .form-group input {
    width: 100%;
    background: var(--md-sys-color-surface-variant);
    border: 2px solid var(--md-sys-color-outline);
    color: var(--md-sys-color-on-surface);
    padding: 0.75rem;
    border-radius: 8px;
    font-size: 1rem;
    transition: all 0.3s ease;
  }

  .form-group input:focus {
    outline: none;
    border-color: var(--md-sys-color-primary);
    box-shadow: var(--app-color-accent-glow-sm);
  }

  .loading-indicator {
    text-align: center;
    color: var(--md-sys-color-primary);
    margin-top: 1rem;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 1rem 1.5rem;
    border-top: 2px solid var(--md-sys-color-outline);
    background: var(--md-sys-color-surface-variant);
  }

  .button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    transition: all 0.3s ease;
  }

  .button.primary {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
  }

  .button.primary:hover:not(:disabled) {
    background: var(--md-sys-color-primary-container);
    transform: translateY(-1px);
    box-shadow: var(--app-shadow-md);
  }

  .button.secondary {
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface-variant);
    border: 1px solid var(--md-sys-color-outline);
  }

  .button.secondary:hover:not(:disabled) {
    background: var(--md-sys-color-surface-container);
    border-color: var(--md-sys-color-primary);
  }

  .button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  @media (max-width: 768px) {
    .dialog {
      min-width: auto;
      margin: 1rem;
    }
  }
</style>
