<script lang="ts">
  import { getRaidGateDifficulty } from '$lib/components/todo/helpers';
  import type { RaidGateDifficultyMap } from '$lib/components/todo/types';

  export let raid: any;
  export let characters: any[] = [];
  export let raidConfigMap: RaidGateDifficultyMap = new Map();
  export let getTaskIcon: (taskId: string) => string;
  export let onRaidGateToggle: (characterId: number, raidId: string, gateId: string) => void;

  function formatGateTitle(plannedDifficulty: string, actualDifficulty?: string | null): string {
    if (!actualDifficulty || actualDifficulty === plannedDifficulty) {
      return `Planned: ${plannedDifficulty}`;
    }

    return `Planned: ${plannedDifficulty} | Cleared: ${actualDifficulty}`;
  }
</script>

<tr>
  <td class="task-name-cell sticky-col first-col">
    <div class="task-info">
      <img src={getTaskIcon(raid.id)} alt={raid.raid_name} class="task-icon" />
      <span class="task-name">{raid.raid_name}</span>
    </div>
  </td>
  {#each characters as character, charIndex}
    {@const state = raid.character_states[charIndex]}
    {@const raidIlvlTooLow = raid.gates.some((gate: any) => gate.min_ilvl && character.ilvl && character.ilvl < gate.min_ilvl)}
    <td class="toggle-cell">
      <div class="cell-content">
        {#if state?.tracked && !raidIlvlTooLow}
          <div class="raid-gates">
            {#each raid.gates as gate}
              {@const difficulty = getRaidGateDifficulty(raidConfigMap, raid.id, character.id, gate.gate)}
              {@const gateIlvlTooLow = gate.min_ilvl && character.ilvl && character.ilvl < gate.min_ilvl}
              {@const gateNumber = parseInt(gate.gate.split(' ')[1]) || 0}
              {@const gateIndex = gateNumber - 1}
              {@const gateState = state.gate_states[gateIndex]}
              {@const actualDifficulty = gateState ? state.gate_actual_difficulties?.[gateIndex] : null}
              <button
                class="gate-toggle"
                class:completed={gateState}
                on:click={() => onRaidGateToggle(character.id, raid.id, gate.gate)}
                title={formatGateTitle(difficulty, actualDifficulty)}
                disabled={gateIlvlTooLow ? true : undefined}
              >
                <div class="gate-button">
                  <span class="gate-number">{gate.gate}</span>
                </div>
              </button>
            {/each}
          </div>
        {:else if raidIlvlTooLow}
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

  .ilvl-warning {
    color: var(--md-sys-color-warning);
    font-size: 1.2rem;
    opacity: 0.8;
  }

  .gate-toggle:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .gate-toggle:disabled .gate-button {
    background: var(--app-todo-gate-disabled-inner, var(--md-sys-color-surface-variant));
    border-color: var(--app-todo-gate-disabled-border, var(--md-sys-color-outline));
  }

  .untracked-task {
    display: flex;
    justify-content: center;
    align-items: center;
    opacity: 0.5;
  }

  .raid-gates {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    justify-content: center;
  }

  .gate-toggle {
    background: none;
    border: none;
    cursor: pointer;
    padding: 0.25rem;
    border-radius: 3px;
    transition: background-color 0.2s ease;
  }

  .gate-toggle:hover {
    background: var(--app-todo-gate-hover-shell, var(--md-sys-color-surface-container-highest));
  }

  .gate-toggle.completed {
    background: var(--app-todo-gate-completed-shell, var(--md-sys-color-surface-container));
  }

  .gate-button {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--app-todo-gate-inner, var(--md-sys-color-surface));
    border: 2px solid var(--app-todo-gate-border, var(--md-sys-color-outline));
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
    box-shadow: var(--app-todo-gate-shadow, none);
  }

  .gate-toggle:hover .gate-button {
    background: var(--app-todo-gate-hover-inner, var(--app-todo-gate-inner, var(--md-sys-color-surface)));
    border-color: var(--app-todo-gate-hover-border, var(--md-sys-color-primary));
    box-shadow: var(--app-todo-gate-hover-shadow, var(--app-todo-gate-shadow, none));
    transform: scale(1.1);
  }

  .gate-toggle.completed .gate-button {
    background: var(--app-todo-gate-completed-inner, var(--md-sys-color-primary));
    border-color: var(--app-todo-gate-completed-border, var(--md-sys-color-primary));
    box-shadow: var(
      --app-todo-gate-completed-shadow,
      0 1px 4px color-mix(in srgb, var(--md-sys-color-primary) 20%, transparent)
    );
  }

  .gate-number {
    font-size: 0.5625rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
  }

  .gate-toggle.completed .gate-number {
    color: var(--app-todo-gate-completed-text, var(--md-sys-color-on-primary));
  }
</style>
