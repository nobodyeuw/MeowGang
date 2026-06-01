<script lang="ts">
  import { dndzone } from 'svelte-dnd-action';
  import type { Character } from '$lib/store';

  export let rosters: any[] = [];
  export let characters: Character[] = [];
  export let activeRosterId = '';
  export let rosterDndItems: any[] = [];
  export let rosterDndOptions: any;
  export let isLoading = false;
  export let onAddRoster: () => void;
  export let onRosterClick: (rosterId: string) => void;
  export let onRosterDndConsider: (event: CustomEvent<any>) => void;
  export let onRosterDndFinalize: (event: CustomEvent<any>) => void;
  export let onRenameRoster: (rosterId: string, currentName: string) => void;
  export let onRemoveRoster: (rosterId: string) => void;

  function getCharacterCount(rosterId: string): number {
    return characters.filter((character) => character.roster_id === rosterId).length;
  }
</script>

<div class="roster-section">
  <div class="settings-container">
    <button
      class="ui-button primary add-button"
      data-guide="add-roster"
      on:click={onAddRoster}
      disabled={isLoading}
    >
      <span>+</span> Add Roster
    </button>
  </div>

  <div
    class="roster-list"
    use:dndzone={rosterDndOptions}
    on:consider={onRosterDndConsider}
    on:finalize={onRosterDndFinalize}
  >
    {#each rosterDndItems as roster (roster.id)}
      <div
        class="settings-list-card interactive roster-item"
        class:active={roster.id === activeRosterId}
        on:click={() => onRosterClick(roster.id)}
        on:keydown={(event) => event.key === 'Enter' && onRosterClick(roster.id)}
        tabindex="0"
        role="button"
      >
        <div class="roster-drag-handle" title="Drag roster" aria-label="Drag roster">
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="9" cy="5" r="1"/><circle cx="9" cy="12" r="1"/><circle cx="9" cy="19" r="1"/>
            <circle cx="15" cy="5" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="15" cy="19" r="1"/>
          </svg>
        </div>
        <div class="roster-info">
          <h4>{roster.roster_name}</h4>
          <p class="roster-id">ID: {roster.id}</p>
          <p class="character-count">{getCharacterCount(roster.id)} characters</p>
        </div>
        <div class="roster-actions">
          <button
            class="ui-button action-button secondary"
            on:click|stopPropagation={() => onRenameRoster(roster.id, roster.roster_name || '')}
            on:keydown|stopPropagation={(event) => event.key === 'Enter' && onRenameRoster(roster.id, roster.roster_name || '')}
          >
            Rename
          </button>
          <button
            class="ui-button danger action-button"
            on:click|stopPropagation={() => onRemoveRoster(roster.id)}
            on:keydown|stopPropagation={(event) => event.key === 'Enter' && onRemoveRoster(roster.id)}
          >
            Remove
          </button>
        </div>
      </div>
    {/each}

    {#if rosters.length === 0}
      <div class="empty-state">
        <p>No rosters configured yet.</p>
        <p>Click "Add Roster" to get started.</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .roster-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .settings-container {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .add-button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
    transition: all 0.3s ease;
    box-shadow: var(--app-shadow-sm);
  }

  .add-button:hover:not(:disabled) {
    background: var(--md-sys-color-primary-container);
    transform: translateY(-2px);
    box-shadow: var(--app-shadow-md);
  }

  .add-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .roster-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .roster-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--md-sys-color-surface);
    padding: 0.8rem;
  }

  .roster-item:hover {
    transform: translateY(-2px);
  }

  .roster-item.active {
    border-color: var(--md-sys-color-primary);
    background: var(--md-sys-color-primary-container);
    box-shadow: var(--app-shadow-sm);
  }

  .roster-drag-handle {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--md-sys-color-on-surface-variant);
    cursor: grab;
    padding: 0.5rem;
    margin-right: 0.5rem;
    flex-shrink: 0;
  }

  .roster-drag-handle:active {
    cursor: grabbing;
  }

  .roster-info {
    flex: 1;
    min-width: 0;
  }

  .roster-info h4 {
    margin: 0 0 0.35rem 0;
    color: var(--md-sys-color-on-surface);
    font-size: 0.98rem;
    font-weight: 600;
  }

  .roster-info p {
    margin: 0.18rem 0;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.78rem;
  }

  .character-count {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.78rem;
  }

  .roster-actions {
    display: flex;
    gap: 0.5rem;
  }

  .action-button {
    font-size: 0.76rem;
  }

  .action-button:hover {
    transform: translateY(-1px);
  }

  .empty-state {
    text-align: center;
    padding: 2rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .empty-state p {
    margin: 0.5rem 0;
  }

  @media (max-width: 768px) {
    .roster-item {
      flex-direction: column;
      align-items: flex-start;
      gap: 1rem;
    }

    .roster-actions {
      width: 100%;
      justify-content: flex-end;
    }
  }
</style>
