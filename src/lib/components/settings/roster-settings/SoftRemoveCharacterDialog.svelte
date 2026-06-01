<script lang="ts">
  import type { Character } from '$lib/store';

  export let character: Character;
  export let remaining = 0;
  export let onCancel: () => void;
  export let onConfirm: () => void;
</script>

<div
  class="dialog-overlay"
  on:click={onCancel}
  role="presentation"
  on:keydown={(event) => event.key === 'Escape' && onCancel()}
>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="dialog delete-character-dialog" on:click|stopPropagation role="dialog" tabindex="-1" aria-modal="true">
    <div class="dialog-header delete-dialog-header">
      <h3>Confirm Character Deletion</h3>
      <button class="close-button" on:click={onCancel}>x</button>
    </div>
    <div class="dialog-content delete-dialog-content">
      <p>
        There is no restoring available. This character will be gone forever and wiped from existence from all active app tabs.
      </p>
      <p class="delete-character-name">{character.char_name}</p>
    </div>
    <div class="dialog-actions">
      <button class="button secondary" on:click={onCancel}>Keep Them</button>
      <button class="button evaporate" on:click={onConfirm} disabled={remaining > 0}>
        {remaining > 0 ? `Evaporate (${remaining})` : 'Evaporate'}
      </button>
    </div>
  </div>
</div>

<style>
  .dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
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

  .delete-character-dialog {
    max-width: 460px;
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
    font-size: 1.3rem;
    font-weight: 600;
  }

  .delete-dialog-header h3 {
    color: color-mix(in srgb, var(--md-sys-color-error) 78%, var(--md-sys-color-primary));
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

  .delete-dialog-content {
    color: var(--md-sys-color-on-surface);
    line-height: 1.45;
  }

  .delete-dialog-content p {
    margin: 0;
  }

  .delete-character-name {
    margin-top: 1rem !important;
    color: var(--md-sys-color-on-surface-variant);
    font-weight: 700;
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

  .button.secondary {
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface-variant);
    border: 1px solid var(--md-sys-color-outline);
  }

  .button.secondary:hover:not(:disabled) {
    background: var(--md-sys-color-surface-container);
    border-color: var(--md-sys-color-primary);
  }

  .button.evaporate {
    background: color-mix(in srgb, var(--md-sys-color-error) 82%, black);
    color: var(--md-sys-color-on-error);
  }

  .button.evaporate:hover:not(:disabled) {
    background: var(--md-sys-color-error);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px color-mix(in srgb, var(--md-sys-color-error) 34%, transparent);
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
