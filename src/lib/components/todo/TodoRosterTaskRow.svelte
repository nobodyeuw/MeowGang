<script lang="ts">
  import Countdown from '$lib/components/common/Countdown.svelte';
  import ToggleButton from '$lib/components/common/ToggleButton.svelte';
  import { isTaskAvailable } from '$lib/utils/availability';

  export let task: any;
  export let characterCount = 0;
  export let rosterTaskStates: Record<string, boolean> = {};
  export let rosterEventProgress: Record<string, any> = {};
  export let getTaskIcon: (taskId: string) => string;
  export let isRosterEventTask: (taskId: string) => boolean;
  export let onRosterTaskToggle: (taskId: string) => void;
  export let onRosterEventToggle: (taskId: string) => void;
</script>

<tr>
  <td class="task-name-cell sticky-col first-col">
    <div class="task-info">
      <img src={getTaskIcon(task.id)} alt={task.name} class="task-icon" />
      <span class="task-name">{task.name}</span>
    </div>
  </td>
  <td colspan={characterCount} class="roster-task-cell">
    <div class="roster-toggle-container">
      {#if isRosterEventTask(task.id)}
        {@const progress = rosterEventProgress[task.id]}
        {#if (task.id === 'gate' || task.id === 'boss') && progress && !progress.available && !progress.completed_today}
          <Countdown taskId={task.id} taskName={task.name} />
        {:else}
          <ToggleButton
            pressed={progress?.completed_today}
            disabled={!progress?.available && !progress?.completed_today}
            ariaLabel={`Toggle ${task.name}`}
            on:change={() => onRosterEventToggle(task.id)}
          >
            {#if task.id !== 'gate' && task.id !== 'boss'}
              <span class="roster-label event-label">
                {progress ? `${progress.completed_this_week}/${progress.weekly_limit}` : '0/3'}
              </span>
            {/if}
            <span class="roster-label">
              {progress?.completed_today ? 'Completed' : progress?.available ? 'Available' : 'Weekly done'}
            </span>
          </ToggleButton>
        {/if}
      {:else if isTaskAvailable(task.id)}
        <ToggleButton
          pressed={rosterTaskStates[task.id]}
          ariaLabel={`Toggle ${task.name}`}
          on:change={() => onRosterTaskToggle(task.id)}
        >
          <span class="roster-label">
            {rosterTaskStates[task.id] ? 'Completed' : 'Available'}
          </span>
        </ToggleButton>
      {:else}
        <Countdown taskId={task.id} taskName={task.name} />
      {/if}
    </div>
  </td>
</tr>

<style>
  .sticky-col {
    position: sticky;
    left: 0;
    background: var(--md-sys-color-surface);
  }

  .first-col {
    z-index: 11;
    min-width: var(--task-column-width);
    background: var(--md-sys-color-surface-variant);
  }

  .task-name-cell.sticky-col.first-col {
    background: var(--md-sys-color-surface-variant);
    z-index: 11;
  }

  .task-name-cell {
    padding: 0.75rem;
    border-bottom: 2px solid var(--app-todo-accent-divider);
  }

  .task-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .task-icon {
    width: 20px;
    height: 20px;
  }

  .task-name {
    font-weight: 500;
  }

  .roster-task-cell {
    text-align: center;
    border-bottom: 2px solid var(--app-todo-accent-divider);
  }

  .roster-toggle-container {
    position: sticky;
    left: calc(var(--task-column-width) + ((100cqw - var(--task-column-width)) / 2));
    z-index: 12;
    display: flex;
    justify-content: center;
    align-items: center;
    width: max-content;
    box-sizing: border-box;
    padding: 0.5rem;
    transform: translateX(-50%);
  }

  .event-label {
    font-weight: 700;
    color: inherit;
  }

  .roster-label {
    font-size: 0.875rem;
    font-weight: 500;
  }
</style>
