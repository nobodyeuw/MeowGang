<script lang="ts">
  import { supportsLazyDaily } from '$lib/components/settings/tracking-settings/helpers';

  export let task: any;
  export let characters: any[] = [];
  export let showRestedControls = false;
  export let areAllCharactersTrackedForTask: (taskId: string) => boolean;
  export let onToggleAllCharactersForTask: (taskId: string, tracked: boolean) => void;
  export let areAllCharactersLazyForTask: (taskId: string) => boolean;
  export let onToggleAllLazyDailyForTask: (taskId: string, lazy: boolean) => void;
  export let getCharacterTaskState: (task: any, charId: number) => any;
  export let onToggleTask: (charId: number, taskId: string, tracked: boolean) => void;
  export let onToggleLazyDaily: (charId: number, taskId: string, lazy: boolean) => void;
  export let onRestedWheel: (event: WheelEvent) => void;
  export let onRestedChange: (event: Event, charId: number, contentId: string) => void;
</script>

<tr>
  <td class="task-name-cell settings-matrix-label-cell settings-matrix-sticky-col settings-matrix-first-col">
    <div class="task-info" class:has-rested-controls={showRestedControls}>
      <span class="task-name">{task.content_name}</span>
      <div class="task-actions">
        <label
          data-guide="tracking-row-toggle"
          class="ui-inline-checkbox"
          title="Toggle all characters"
        >
          <input
            type="checkbox"
            checked={areAllCharactersTrackedForTask(task.content_id)}
            aria-label={`Toggle all characters for ${task.content_name}`}
            on:change={(event) => onToggleAllCharactersForTask(task.content_id, event.currentTarget.checked)}
          />
          <span>All</span>
        </label>
        {#if showRestedControls && supportsLazyDaily(task.content_id)}
          <label
            class="ui-inline-checkbox accent-warning"
            title="Toggle lazy behavior for all characters"
          >
            <input
              type="checkbox"
              checked={areAllCharactersLazyForTask(task.content_id)}
              aria-label={`Toggle lazy behavior for ${task.content_name}`}
              on:change={(event) => onToggleAllLazyDailyForTask(task.content_id, event.currentTarget.checked)}
            />
            <span>Lazy</span>
          </label>
        {/if}
      </div>
    </div>
  </td>
  {#each characters as char}
    <td class="toggle-cell">
      <div class="cell-content">
        <input
          type="checkbox"
          class="settings-matrix-checkbox"
          checked={getCharacterTaskState(task, char.char_id)?.tracked || false}
          aria-label={`Toggle ${task.content_name} for ${char.name ?? char.character_name ?? 'character'}`}
          on:change={(event) => onToggleTask(char.char_id, task.content_id, event.currentTarget.checked)}
        />
        {#if showRestedControls && task.max_rest_value && (task.content_id === 'chaos' || task.content_id === 'guardian')}
          <div class="rested-input">
            <input
              type="number"
              data-guide="rested-input"
              placeholder="0"
              min="0"
              step="10"
              inputmode="numeric"
              max={task.max_rest_value}
              value={getCharacterTaskState(task, char.char_id)?.current_value || 0}
              on:wheel={onRestedWheel}
              on:change={(event) => onRestedChange(event, char.char_id, task.content_id)}
            />
          </div>
          <input
            type="checkbox"
            class="settings-matrix-checkbox accent-secondary"
            checked={getCharacterTaskState(task, char.char_id)?.lazy_daily || false}
            title="Only count this daily when rested is 20 or higher"
            aria-label={`Toggle lazy daily for ${task.content_name}`}
            on:change={(event) => onToggleLazyDaily(char.char_id, task.content_id, event.currentTarget.checked)}
          />
        {/if}
      </div>
    </td>
  {/each}
</tr>

<style>
  .task-info {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .task-info.has-rested-controls {
    flex-direction: column;
    align-items: flex-start;
  }

  .task-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .task-name {
    font-weight: 500;
    color: var(--md-sys-color-on-surface);
  }

  .toggle-cell {
    padding: 7px;
    text-align: center;
    border-bottom: 1px solid var(--md-sys-color-outline);
    border-left: 1px solid var(--md-sys-color-outline);
    min-width: 80px;
  }

  .cell-content {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 8px;
    min-height: 32px;
  }

  .rested-input input {
    width: 60px;
    padding: 4px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 4px;
    font-size: 12px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
  }

  .task-name-cell.settings-matrix-sticky-col {
    z-index: 15;
  }

  @media (max-width: 768px) {
    .task-name-cell {
      min-width: 150px;
    }
  }
</style>
