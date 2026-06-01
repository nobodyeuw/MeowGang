<script lang="ts">
  export let rosterName = '';
  export let isLoading = false;
  export let onCancel: () => void;
  export let onConfirm: () => void;
</script>

<div
  class="dialog-overlay"
  role="presentation"
  on:click={onCancel}
  on:keydown={(event) => event.key === 'Escape' && onCancel()}
>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="dialog" role="dialog" aria-modal="true" tabindex="-1" on:click|stopPropagation>
    <div class="dialog-header">
      <h3>Delete Roster</h3>
      <button class="close-button" on:click={onCancel} disabled={isLoading}>x</button>
    </div>
    <div class="dialog-content">
      <p>
        Are you sure you want to delete {rosterName || 'this roster'} and all its characters?
        This action cannot be undone and will permanently delete:
      </p>
      <ul>
        <li>All characters from this roster</li>
        <li>All tracking configurations</li>
        <li>All rested values</li>
        <li>All gold logs</li>
        <li>All completion status</li>
      </ul>
    </div>
    <div class="dialog-actions">
      <button class="button secondary" on:click={onCancel} disabled={isLoading}>Cancel</button>
      <button class="button danger" on:click={onConfirm} disabled={isLoading}>
        {isLoading ? 'Deleting...' : 'Delete'}
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
    border: 1px solid var(--md-sys-color-error);
    border-radius: 12px;
    min-width: 340px;
    max-width: min(460px, 90vw);
    overflow: hidden;
    box-shadow: var(--app-shadow-md);
  }

  .dialog-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--md-sys-color-outline);
    background: var(--md-sys-color-surface-variant);
  }

  .dialog-header h3 {
    margin: 0;
    color: var(--md-sys-color-error);
    font-size: 1.1rem;
    font-weight: 700;
  }

  .close-button {
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
    font-size: 1.25rem;
  }

  .close-button:hover:not(:disabled) {
    background: var(--md-sys-color-surface-container);
    color: var(--md-sys-color-on-surface);
  }

  .dialog-content {
    padding: 1rem 1.25rem;
    color: var(--md-sys-color-on-surface);
    line-height: 1.45;
  }

  .dialog-content p {
    margin: 0 0 0.85rem;
  }

  .dialog-content ul {
    margin: 0;
    padding-left: 1.25rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
    padding: 1rem 1.25rem;
    border-top: 1px solid var(--md-sys-color-outline);
    background: var(--md-sys-color-surface-variant);
  }

  .button {
    padding: 0.5rem 0.9rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 700;
    border: 1px solid transparent;
  }

  .button.secondary {
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    border-color: var(--md-sys-color-outline);
  }

  .button.danger {
    background: var(--md-sys-color-error);
    color: var(--md-sys-color-on-error);
  }

  .button:disabled {
    cursor: not-allowed;
    opacity: 0.62;
  }
</style>
