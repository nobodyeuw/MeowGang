<script lang="ts">
  import { isRosterEventTask } from '$lib/components/settings/tracking-settings/helpers';

  export let task: any;
  export let characterCount = 0;
  export let rosterEventProgress: Record<string, any> = {};
  export let onToggleRosterTask: (taskId: string, tracked: boolean) => void;
  export let onRosterEventCountChange: (event: Event, taskId: string) => void;
  export let onRestedWheel: (event: WheelEvent) => void;
</script>

<tr>
  <td class="task-name-cell sticky-col first-col">{task.content_name}</td>
  <td class="toggle-cell roster-toggle-cell" colspan={characterCount}>
    <div class="cell-content">
      <input
        type="checkbox"
        class="tracking-checkbox"
        checked={task.character_states[0]?.tracked || false}
        aria-label={`Toggle ${task.content_name}`}
        on:change={(event) => onToggleRosterTask(task.content_id, event.currentTarget.checked)}
      />
      {#if isRosterEventTask(task.content_id)}
        <span class="roster-label">All Characters</span>
        <input
          type="number"
          class="event-count-input"
          min="0"
          max={rosterEventProgress[task.content_id]?.weekly_limit ?? 3}
          step="1"
          value={rosterEventProgress[task.content_id]?.completed_this_week ?? 0}
          disabled={!task.character_states[0]?.tracked}
          aria-label={`${task.content_name} completions this week`}
          on:change={(event) => onRosterEventCountChange(event, task.content_id)}
          on:wheel={onRestedWheel}
        />
        <span class="roster-label">
          / {rosterEventProgress[task.content_id]?.weekly_limit ?? 3} this week
        </span>
      {:else}
        <span class="roster-label">All Characters</span>
      {/if}
    </div>
  </td>
</tr>

<style>
  .task-name-cell {
    background: var(--md-sys-color-surface-variant);
    padding: 12px 8px;
    border-bottom: 1px solid var(--md-sys-color-outline);
    font-weight: 500;
    min-width: var(--task-column-width);
  }

  .toggle-cell {
    padding: 8px;
    text-align: center;
    border-bottom: 1px solid var(--md-sys-color-outline);
    border-left: 1px solid var(--md-sys-color-outline);
    min-width: 80px;
  }

  .roster-toggle-cell {
    background: var(--md-sys-color-surface-container);
  }

  .roster-toggle-cell .cell-content {
    position: sticky;
    left: calc(var(--task-column-width) + ((100cqw - var(--task-column-width)) / 2));
    z-index: 12;
    width: max-content;
    box-sizing: border-box;
    padding: 0.5rem;
    transform: translateX(-50%);
  }

  .cell-content {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 8px;
    min-height: 32px;
  }

  .tracking-checkbox {
    width: 16px;
    height: 16px;
    accent-color: var(--md-sys-color-primary);
    cursor: pointer;
  }

  .roster-label {
    font-size: 12px;
    color: var(--md-sys-color-on-surface-variant);
    font-style: italic;
  }

  .event-count-input {
    width: 3rem;
    padding: 4px 6px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 4px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    font-size: 12px;
    font-weight: 600;
    text-align: center;
    outline: none;
  }

  .event-count-input:focus {
    border-color: var(--md-sys-color-primary);
  }

  .event-count-input:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .sticky-col {
    position: sticky;
    left: 0;
    z-index: 10;
    background: var(--md-sys-color-surface);
    box-shadow: 2px 0 0 0 var(--md-sys-color-outline);
  }

  .first-col {
    z-index: 20;
    min-width: var(--task-column-width);
    background: var(--md-sys-color-surface-variant);
    box-shadow: 2px 0 0 0 var(--md-sys-color-outline);
  }

  .task-name-cell.sticky-col {
    background: var(--md-sys-color-surface-variant);
    z-index: 15;
  }

  @media (max-width: 768px) {
    .task-name-cell {
      min-width: 150px;
    }
  }
</style>
