<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { activeRosterId, rosters } from '$lib/store';
  import { GAME_TASKS, GAME_CLASSES } from '$lib/data/index';
  import { RAIDS } from '$lib/data/raids';
  import type { TrackingConfigMatrix } from '$lib/types';

  let selectedCharacterId: number | null = null;

  let matrixData: TrackingConfigMatrix | null = null;
  let isLoading = true;
  let error = '';

  async function loadMatrixData() {
    try {
      isLoading = true;
      
      // Get basic matrix data with characters and tracking status
      const baseMatrix = await invoke('get_tracking_config_matrix', { 
        rosterId: $activeRosterId,
        tasks: [],
        raids: []
      });
      
      // Transform tasks to match backend field names and use backend data
      const tasksArray = Object.values(GAME_TASKS).map(task => {
        // Create character states from backend character_states
        const characterStates = baseMatrix.characters.map((char: any) => {
          // Find state for this character and task from backend character_states
          const backendState = baseMatrix.character_states?.find((s: any) => {
            return s.char_id === char.char_id && s.content_id === task.id;
          });
          
          return {
            char_id: char.char_id,
            tracked: backendState?.tracked || false, // Use backend data
            current_value: backendState?.current_value || null
          };
        });
        
        return {
          content_id: task.id,
          content_name: task.name,
          category: task.category,
          reset_schedule: task.reset_schedule,
          logic_type: task.logic_type,
          max_rest_value: task.max_rest_value,
          character_states: characterStates
        };
      });
      
      // Transform raids and sort by min_ilvl, grouping by base name
      const raidsMap = new Map<string, any>();
      [...RAIDS].forEach(raid => {
        const baseName = raid.name; // Use name directly without difficulty
        if (!raidsMap.has(baseName) || raid.min_ilvl < raidsMap.get(baseName)?.gates[0]?.minIlvl) {
          raidsMap.set(baseName, raid);
        }
      });
      
      const raidsArray = Array.from(raidsMap.values()).sort((a, b) => {
        const aMinIlvl = a.gates[0]?.minIlvl || 0;
        const bMinIlvl = b.gates[0]?.minIlvl || 0;
        return aMinIlvl - bMinIlvl;
      }).map((raid: any) => {
        // Create character states from backend data for raids
        const characterStates = baseMatrix.characters.map((char: any) => {
          // Find the state for this character and raid from backend character_states
          const backendState = baseMatrix.character_states?.find((s: any) => 
            s.char_id === char.char_id && s.content_id === raid.id
          );
          
          return {
            char_id: char.char_id,
            tracked: backendState?.tracked || false, // Use backend data
            current_value: null // Raids don't have rested values
          };
        });
        
        return {
          raid_id: raid.id,
          raid_name: raid.name, // Only name, no difficulty
          min_ilvl: raid.gates[0]?.minIlvl || 0,
          character_states: characterStates
        };
      });
      
      // Combine everything with proper categorization
      matrixData = {
        characters: baseMatrix.characters,
        daily_tasks: tasksArray.filter((task: any) => 
          task.reset_schedule === 'daily' && 
          (task.content_id === 'chaos' || task.content_id === 'guardian')
        ),
        weekly_tasks: tasksArray.filter((task: any) => task.reset_schedule === 'weekly'),
        roster_tasks: tasksArray.filter((task: any) => task.category === 'roster'),
        raids: raidsArray
      };
      
      if (!matrixData || !matrixData.characters || matrixData.characters.length === 0) {
        console.error('No characters found in matrix data!');
        error = 'No characters found for this roster';
        return;
      }
    } catch (err) {
      error = `Error loading matrix: ${err}`;
    } finally {
      isLoading = false;
    }
  }

  async function updateRestedValue(charId: number, contentId: string, currentValue: number) {
    try {
      await invoke('save_rested_value', {
        characterId: charId,
        taskId: contentId,
        restedValue: currentValue
      });
      
      // Update local data instead of reloading entire matrix
      const updateTaskState = (tasks: any[]) => {
        for (const task of tasks) {
          const state = task.character_states.find((s: any) => s.char_id === charId);
          if (state) {
            state.current_value = currentValue;
          }
        }
      };
      
      updateTaskState(matrixData.daily_tasks);
      updateTaskState(matrixData.weekly_tasks);
      updateTaskState(matrixData.roster_tasks);
      
    } catch (err) {
      console.error('Failed to update rested value:', err);
    }
  }

  function getClassIcon(classId: string) {
    return GAME_CLASSES[classId]?.iconId || '0';
  }

  function getCharacterTaskState(task: any, charId: number): any {
    return task.character_states.find((state: any) => state.char_id === charId);
  }

  function getCharacterRaidState(raid: any, charId: number): any {
    return raid.character_states.find((state: any) => state.char_id === charId);
  }

  async function toggleTask(charId: number, taskId: string, newState: boolean) {
    try {
      await invoke('update_tracking_config', {
        characterId: charId,
        taskId: taskId,
        tracked: newState,
        currentValue: null
      });
      
      // Update local data with the confirmed new state
      const updateTaskState = (tasks: any[]) => {
        for (const task of tasks) {
          const state = task.character_states.find((s: any) => s.char_id === charId);
          if (state) {
            state.tracked = newState;
          }
        }
      };
      
      updateTaskState(matrixData.daily_tasks);
      updateTaskState(matrixData.weekly_tasks);
      updateTaskState(matrixData.roster_tasks);
      
    } catch (err) {
      console.error('Failed to toggle task:', err);
    }
  }

  async function toggleRaid(charId: number, raidId: string, newState: boolean) {
    try {
      await invoke('update_tracking_config', {
        characterId: charId,
        taskId: raidId,
        tracked: newState,
        currentValue: null
      });
      
      // Update local data with the confirmed new state
      for (const raid of matrixData.raids) {
        const state = raid.character_states.find((s: any) => s.char_id === charId);
        if (state) {
          state.tracked = newState;
        }
      }
      
    } catch (err) {
      console.error('Failed to toggle raid:', err);
    }
  }

  async function toggleRosterTask(taskId: string, newState: boolean) {
    try {
      // For roster tasks, we need to update all characters
      const characters = matrixData?.characters || [];
      
      for (const char of characters) {
        await invoke('update_tracking_config', {
          characterId: char.char_id,
          taskId: taskId,
          tracked: newState,
          currentValue: null
        });
      }
      
      // Update local data instead of reloading entire matrix
      const task = matrixData.roster_tasks.find((t: any) => t.content_id === taskId);
      if (task) {
        for (const state of task.character_states) {
          state.tracked = newState;
        }
      }
      
    } catch (err) {
      console.error('Failed to toggle roster task:', err);
    }
  }

  async function toggleAllCharactersForTask(taskId: string, newState: boolean) {
    try {
      // For individual tasks, update all characters in the roster
      const characters = matrixData?.characters || [];
      
      for (const char of characters) {
        await invoke('update_tracking_config', {
          characterId: char.char_id,
          taskId: taskId,
          tracked: newState,
          currentValue: null
        });
      }
      
      // Update local data for all task sections
      const updateTaskState = (tasks: any[]) => {
        const task = tasks.find((t: any) => t.content_id === taskId);
        if (task) {
          for (const state of task.character_states) {
            state.tracked = newState;
          }
        }
      };
      
      updateTaskState(matrixData.daily_tasks);
      updateTaskState(matrixData.weekly_tasks);
      
    } catch (err) {
      console.error('Failed to toggle all characters for task:', err);
    }
  }

  function areAllCharactersTrackedForTask(taskId: string): boolean {
    const checkTaskState = (tasks: any[]) => {
      const task = tasks.find((t: any) => t.content_id === taskId);
      if (!task || task.character_states.length === 0) return false;
      return task.character_states.every((state: any) => state.tracked === true);
    };
    
    return checkTaskState(matrixData.daily_tasks) || checkTaskState(matrixData.weekly_tasks);
  }

  // Load data when component mounts or roster changes
  $: if ($activeRosterId) {
    loadMatrixData();
  }
</script>

<div class="tracking-settings">
  <!-- Roster Selector -->
  <div class="roster-selector">
    <label for="roster-select" class="roster-label">Active Roster:</label>
    <select 
      id="roster-select"
      bind:value={$activeRosterId}
      class="roster-dropdown"
    >
      {#each $rosters as roster}
        <option value={roster.id}>{roster.roster_name}</option>
      {/each}
    </select>
  </div>

  {#if isLoading}
    <div class="loading">Loading tracking data...</div>
  {:else if error}
    <div class="error">
      <p>{error}</p>
      <button on:click={loadMatrixData}>Retry</button>
    </div>
  {:else if matrixData}
    <div class="matrix-container">
      <div class="tracking-matrix-wrapper">
        <table class="tracking-matrix">
          <thead>
            <tr class="header-row">
              <th class="sticky-col first-col">Tasks/Character</th>
              {#each matrixData.characters as char}
                <th class="char-header sticky-col">
                  <div class="char-info">
                    <img src={`/images/classes/${getClassIcon(char.class_id)}.png`} alt="" class="class-icon" />
                    <span class="char-name">{char.char_name}</span>
                    <div class="char-stats">
                      <span class="char-ilvl">{char.item_level.toFixed(0)}</span>
                      <span class="char-cp">{char.combat_power.toFixed(0)}</span>
                    </div>
                  </div>
                </th>
              {/each}
            </tr>
          </thead>
          <tbody>
          <!-- Section 1: DAILY (Chaos & Guardian) -->
          <tr class="section-separator">
            <td colspan={matrixData.characters.length + 1}>
              <div class="section-title">DAILY</div>
            </td>
          </tr>
          {#each matrixData.daily_tasks as task}
            <tr>
              <td class="task-name-cell sticky-col first-col">
                <div class="task-info">
                  <span class="task-name">{task.content_name}</span>
                  <button 
                    class="toggle-all-btn"
                    on:click={() => {
                      const currentState = areAllCharactersTrackedForTask(task.content_id);
                      toggleAllCharactersForTask(task.content_id, !currentState);
                    }}
                    title="Toggle all characters"
                  >
                    {areAllCharactersTrackedForTask(task.content_id) ? '☑' : '☐'}
                  </button>
                </div>
              </td>
              {#each matrixData.characters as char}
                <td class="toggle-cell">
                  <div class="cell-content">
                    <input 
                      type="checkbox" 
                      class="task-checkbox"
                      checked={getCharacterTaskState(task, char.char_id)?.tracked || false}
                      on:change={(event) => {
                        const newState = event.target.checked;
                        toggleTask(char.char_id, task.content_id, newState);
                      }}
                    />
                    {#if task.max_rest_value && (task.content_id === 'chaos' || task.content_id === 'guardian')}
                      <div class="rested-input">
                        <input 
                          type="number" 
                          placeholder="0" 
                          min="0" 
                          max={task.max_rest_value}
                          value={getCharacterTaskState(task, char.char_id)?.current_value || 0}
                          on:change={(e) => {
                            const currentValue = parseInt(e.target.value) || 0;
                            updateRestedValue(char.char_id, task.content_id, currentValue);
                          }}
                        />
                      </div>
                    {/if}
                  </div>
                </td>
              {/each}
            </tr>
          {/each}

          <!-- Section 2: WEEKLY -->
          <tr class="section-separator">
            <td colspan={matrixData.characters.length + 1}>
              <div class="section-title">WEEKLY</div>
            </td>
          </tr>
          {#each matrixData.weekly_tasks as task}
            <tr>
              <td class="task-name-cell sticky-col first-col">
                <div class="task-info">
                  <span class="task-name">{task.content_name}</span>
                  <button 
                    class="toggle-all-btn"
                    on:click={() => {
                      const currentState = areAllCharactersTrackedForTask(task.content_id);
                      toggleAllCharactersForTask(task.content_id, !currentState);
                    }}
                    title="Toggle all characters"
                  >
                    {areAllCharactersTrackedForTask(task.content_id) ? '☑' : '☐'}
                  </button>
                </div>
              </td>
              {#each matrixData.characters as char}
                <td class="toggle-cell">
                  <div class="cell-content">
                    <input 
                      type="checkbox" 
                      class="task-checkbox"
                      checked={getCharacterTaskState(task, char.char_id)?.tracked || false}
                      on:change={(event) => {
                        const newState = event.target.checked;
                        toggleTask(char.char_id, task.content_id, newState);
                      }}
                    />
                  </div>
                </td>
              {/each}
            </tr>
          {/each}

          <!-- Section 3: ROSTER WIDE -->
          <tr class="section-separator">
            <td colspan={matrixData.characters.length + 1}>
              <div class="section-title">ROSTER WIDE</div>
            </td>
          </tr>
          {#each matrixData.roster_tasks as task}
            <tr>
              <td class="task-name-cell sticky-col first-col">{task.content_name}</td>
              <td class="toggle-cell roster-toggle-cell" colspan={matrixData.characters.length}>
                <div class="cell-content">
                  <input 
                    type="checkbox" 
                    class="task-checkbox"
                    checked={task.character_states[0]?.tracked || false}
                    on:change={(event) => {
                      const newState = event.target.checked;
                      toggleRosterTask(task.content_id, newState);
                    }}
                  />
                  <span class="roster-label">All Characters</span>
                </div>
              </td>
            </tr>
          {/each}

          <!-- Section 4: RAIDS -->
          <tr class="section-separator">
            <td colspan={matrixData.characters.length + 1}>
              <div class="section-title">RAIDS</div>
            </td>
          </tr>
          {#each matrixData.raids as raid}
            <tr>
              <td class="task-name-cell sticky-col first-col">
                <div class="raid-info">
                  <span class="raid-name">{raid.raid_name}</span>
                  <span class="raid-ilvl">iLvl: {raid.min_ilvl}</span>
                </div>
              </td>
              {#each matrixData.characters as char}
                <td class="toggle-cell">
                  <div class="cell-content">
                    {#if raid.min_ilvl <= char.item_level}
                      <input 
                        type="checkbox" 
                        class="task-checkbox"
                        checked={getCharacterRaidState(raid, char.char_id)?.tracked || false}
                        on:change={(event) => {
                          const newState = event.target.checked;
                          toggleRaid(char.char_id, raid.raid_id, newState);
                        }}
                      />
                    {:else}
                      <div class="ineligible">iLvl too low</div>
                    {/if}
                  </div>
                </td>
              {/each}
            </tr>
          {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}
</div>

<style>
  .tracking-settings {
    display: flex;
    flex-direction: column;
    padding: 0;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  .roster-selector {
    padding: 16px;
    background: var(--md-sys-color-surface);
    border-bottom: 1px solid var(--md-sys-color-outline);
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .roster-label {
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
    font-size: 14px;
  }

  .roster-dropdown {
    padding: 8px 12px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    font-size: 14px;
    min-width: 200px;
    cursor: pointer;
  }

  .roster-dropdown:focus {
    outline: 2px solid var(--md-sys-color-primary);
    outline-offset: 2px;
  }

  .loading, .error {
    text-align: center;
    padding: 40px;
    color: var(--md-sys-color-on-surface);
  }

  .error {
    color: var(--md-sys-color-error);
  }

  .error button {
    margin-top: 10px;
    padding: 8px 16px;
    background: var(--md-sys-color-error);
    color: var(--md-sys-color-on-error);
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .matrix-container {
    display: flex;
    flex: 1 1 auto;
    min-height: 0;
    background: var(--md-sys-color-surface);
    border-radius: 12px;
    overflow: hidden;
    border: 1px solid var(--md-sys-color-outline);
    height: 100%;
    max-height: none;
  }

  .tracking-matrix-wrapper {
    min-height: 0;
    overflow-x: auto;
    overflow-y: auto;
    height: 100%;
    position: relative;
  }

  .tracking-matrix {
    width: 100%;
    border-collapse: collapse;
    font-size: 14px;
    min-width: 800px; /* Ensure minimum width for horizontal scrolling */
  }

  .header-row th {
    background: var(--md-sys-color-surface-variant);
    padding: 12px 8px;
    text-align: center;
    border-bottom: 2px solid var(--md-sys-color-outline);
    font-weight: 600;
    color: var(--md-sys-color-on-surface-variant);
  }

  .char-header {
    min-width: 120px;
    border-left: 1px solid var(--md-sys-color-outline);
  }

  .char-info {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .class-icon {
    width: 32px;
    height: 32px;
    border-radius: 50%;
  }

  .char-name {
    font-weight: 600;
    font-size: 12px;
    color: var(--md-sys-color-on-surface);
  }

  .char-stats {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 10px;
    color: var(--md-sys-color-on-surface-variant);
  }

  .char-ilvl {
    color: var(--md-sys-color-tertiary);
  }

  .char-cp {
    color: var(--md-sys-color-secondary);
  }

  .section-separator td {
    background: var(--md-sys-color-primary-container);
    padding: 8px 12px;
    font-weight: 600;
    color: var(--md-sys-color-on-primary-container);
    border: none;
  }

  .section-title {
    font-size: 16px;
    font-weight: 700;
  }

  .task-name-cell {
    background: var(--md-sys-color-surface-variant);
    padding: 12px 8px;
    border-bottom: 1px solid var(--md-sys-color-outline);
    font-weight: 500;
    min-width: 200px;
  }

  .task-info {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .task-name {
    font-weight: 500;
    color: var(--md-sys-color-on-surface);
  }

  .toggle-all-btn {
    background: var(--md-sys-color-surface);
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 12px;
    cursor: pointer;
    color: var(--md-sys-color-on-surface);
    transition: all 0.2s ease;
    align-self: flex-start;
  }

  .toggle-all-btn:hover {
    background: var(--md-sys-color-primary-container);
    border-color: var(--md-sys-color-primary);
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

  .cell-content {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 8px;
    min-height: 32px;
  }

  .task-checkbox {
    width: 18px;
    height: 18px;
    cursor: pointer;
    accent-color: var(--md-sys-color-primary);
  }

  .roster-label {
    font-size: 12px;
    color: var(--md-sys-color-on-surface-variant);
    font-style: italic;
  }

  .raid-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .raid-name {
    font-weight: 500;
    color: var(--md-sys-color-on-surface);
  }

  .raid-ilvl {
    font-size: 12px;
    color: var(--md-sys-color-on-surface-variant);
    background: var(--md-sys-color-surface-container);
    padding: 2px 6px;
    border-radius: 3px;
  }

  .ineligible {
    font-size: 11px;
    color: var(--md-sys-color-on-surface-variant);
    background: var(--md-sys-color-surface-variant);
    padding: 4px 6px;
    border-radius: 3px;
    font-style: italic;
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
    background: var(--md-sys-color-surface-variant);
    box-shadow: 2px 0 0 0 var(--md-sys-color-outline);
  }

  .char-header.sticky-col {
    background: var(--md-sys-color-surface-variant);
    z-index: 15;
    top: 0;
  }

  .task-name-cell.sticky-col {
    background: var(--md-sys-color-surface-variant);
    z-index: 15;
  }

  /* Ensure sticky columns work properly with scrolling */
  .tracking-matrix-wrapper::-webkit-scrollbar {
    width: 8px;
    height: 8px;
  }

  .tracking-matrix-wrapper::-webkit-scrollbar-track {
    background: var(--md-sys-color-surface-variant);
  }

  .tracking-matrix-wrapper::-webkit-scrollbar-thumb {
    background: var(--md-sys-color-on-surface-variant);
    border-radius: 4px;
  }

  .tracking-matrix-wrapper::-webkit-scrollbar-thumb:hover {
    background: var(--md-sys-color-on-surface);
  }

  @media (max-width: 768px) {
    .tracking-settings {
      padding: 10px;
    }
    
    .char-header {
      min-width: 100px;
    }
    
    .task-name-cell {
      min-width: 150px;
    }
  }
</style>
