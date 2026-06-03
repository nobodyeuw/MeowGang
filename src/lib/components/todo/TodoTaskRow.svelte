<script lang="ts">
  import ToggleButton from '$lib/components/common/ToggleButton.svelte';

  export let task: any;
  export let characters: any[] = [];
  export let showRested = false;
  export let getTaskIcon: (taskId: string) => string;
  export let getCharacterTaskState: (task: any, character: any) => any;
  export let onTaskToggle: (characterId: number, taskId: string, state: any) => void;
</script>

<tr>
  <td class="task-name-cell sticky-col first-col">
    <div class="task-info">
      <img src={getTaskIcon(task.id)} alt={task.name} class="task-icon" />
      <span class="task-name">{task.name}</span>
    </div>
  </td>
  {#each characters as character}
    {@const state = getCharacterTaskState(task, character)}
    <td class="toggle-cell">
      <div class="cell-content">
        {#if state?.tracked && !state?.ilvl_too_low}
          <div class="task-toggle-container">
            <ToggleButton
              pressed={state.completed}
              ariaLabel={`${state.completed ? 'Mark available' : 'Mark completed'} ${task.name} for ${character.name ?? 'character'}`}
              on:change={() => onTaskToggle(character.id, task.id, state)}
            >
              {#if state.completed}
                <span class="checkbox checked">&#10003;</span>
              {:else}
                <span class="checkbox">&#9675;</span>
              {/if}
            </ToggleButton>
            {#if showRested && task.logic_type === 'rested' && state.rested_value !== undefined}
              <div class="rested-bar-container">
                {#each Array(5) as _, i}
                  {@const threshold = (i + 1) * 20}
                  <div class="rested-segment" class:filled={(state.rested_value || 0) >= threshold}></div>
                {/each}
              </div>
              <div class="rested-value-display">{state.rested_value}%</div>
            {/if}
          </div>
        {:else if state?.ilvl_too_low}
          <div class="untracked-task">
            <span class="ilvl-warning">!</span>
          </div>
        {/if}
      </div>
    </td>
  {/each}
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

  .toggle-cell {
    padding: 0.5rem;
    text-align: center;
    border-bottom: 2px solid var(--app-todo-accent-divider);
  }

  .task-toggle-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
  }

  .checkbox {
    font-size: 1rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
    transition: all 0.2s ease;
  }

  .checkbox.checked {
    color: var(--md-sys-color-on-primary);
  }

  .ilvl-warning {
    color: var(--md-sys-color-warning);
    font-size: 1.2rem;
    opacity: 0.8;
  }

  .rested-bar-container {
    display: flex;
    gap: 2px;
    margin-top: 2px;
  }

  .rested-segment {
    width: 8px;
    height: 4px;
    border-radius: 1px;
    background: var(--md-sys-color-surface-container-highest);
    transition: background-color 0.2s ease;
  }

  .rested-segment.filled {
    background: var(--md-sys-color-success);
  }

  .rested-value-display {
    font-size: 0.75rem;
    color: var(--md-sys-color-on-surface-variant);
    margin-top: 2px;
    opacity: 0.8;
  }

  .untracked-task {
    display: flex;
    justify-content: center;
    align-items: center;
    opacity: 0.5;
  }
</style>
