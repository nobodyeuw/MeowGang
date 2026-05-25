<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { activeRosterId } from '$lib/store';
  import { GAME_TASKS, GAME_CLASSES } from '$lib/data/index';
  import { RAIDS } from '$lib/data/raids';
  import RosterButtonGroup from '$lib/components/common/RosterButtonGroup.svelte';

  const COLLAPSE_UNTRACKED_ROWS_STORAGE_KEY = 'trackingSettings.collapseUntrackedRows';

  let selectedCharacterId: number | null = null;

  let matrixData: any | null = null;
  let isLoading = true;
  let error = '';
  let lastLoadedRosterId: string = '';
  let warningMessage = '';
  let rosterEventProgress: Record<string, RosterEventProgress> = {};
  let collapseUntrackedRows = loadCollapseUntrackedRows();
  $: visibleDailyTasks = getVisibleTrackingRows(matrixData?.daily_tasks || [], collapseUntrackedRows);
  $: visibleWeeklyTasks = getVisibleTrackingRows(matrixData?.weekly_tasks || [], collapseUntrackedRows);
  $: visibleRosterTasks = getVisibleTrackingRows(matrixData?.roster_tasks || [], collapseUntrackedRows);
  $: visibleRaids = getVisibleTrackingRows(matrixData?.raids || [], collapseUntrackedRows);
  $: hasHiddenTrackingRows = Boolean(matrixData) && [
    ...(matrixData?.daily_tasks || []),
    ...(matrixData?.weekly_tasks || []),
    ...(matrixData?.roster_tasks || []),
    ...(matrixData?.raids || [])
  ].some((row: any) => !isTrackingRowEnabled(row));

  interface RosterEventProgress {
    task_id: string;
    completed_this_week: number;
    weekly_limit: number;
    completed_today: boolean;
    available: boolean;
  }

  function isRosterEventTask(taskId: string): boolean {
    return taskId === 'event_argeos_winter';
  }

  function loadCollapseUntrackedRows(): boolean {
    try {
      return localStorage.getItem(COLLAPSE_UNTRACKED_ROWS_STORAGE_KEY) === '1';
    } catch {
      return false;
    }
  }

  function setCollapseUntrackedRows(value: boolean) {
    collapseUntrackedRows = value;
    try {
      localStorage.setItem(COLLAPSE_UNTRACKED_ROWS_STORAGE_KEY, value ? '1' : '0');
    } catch {
      // Ignore storage failures; the in-memory view state still updates.
    }
  }

  function getVisibleTrackingRows(rows: any[], collapseRows: boolean) {
    return collapseRows ? rows.filter(isTrackingRowEnabled) : rows;
  }

  function isTrackingRowEnabled(row: any): boolean {
    return (row?.character_states || []).some((state: any) => state.tracked === true);
  }

  async function loadRosterEventProgress() {
    const rosterId = $activeRosterId;
    const eventTasks = matrixData?.roster_tasks?.filter((task: any) => isRosterEventTask(task.content_id)) || [];
    const nextProgress: Record<string, RosterEventProgress> = {};

    for (const task of eventTasks) {
      try {
        const progress = await invoke<RosterEventProgress>('get_roster_event_progress', {
          rosterId,
          taskId: task.content_id
        });
        nextProgress[task.content_id] = progress;
      } catch (err) {
        console.warn('Failed to load roster event progress:', err);
      }
    }

    rosterEventProgress = nextProgress;
  }

  function syncRosterEventTaskState(taskId: string) {
    const progress = rosterEventProgress[taskId];
    if (!progress || !matrixData?.roster_tasks) return;

    matrixData.roster_tasks = matrixData.roster_tasks.map((task: any) => {
      if (task.content_id !== taskId) return task;
      return {
        ...task,
        character_states: task.character_states.map((state: any) => ({
          ...state,
          completed: progress.completed_this_week >= progress.weekly_limit
        }))
      };
    });
    matrixData = { ...matrixData };
  }

  async function loadMatrixData() {
    try {
      error = '';
      matrixData = null;
      isLoading = true;
      
      // Get basic matrix data with characters and tracking status
      const baseMatrix = await invoke<any>('get_tracking_config_matrix', { 
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
            current_value: backendState?.current_value || null,
            lazy_daily: backendState?.lazy_daily || false
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
        const raidMinIlvl = raid.gates[0]?.minIlvl || 0;
        const existingMinIlvl = raidsMap.get(baseName)?.gates[0]?.minIlvl || 0;
        if (!raidsMap.has(baseName) || raidMinIlvl < existingMinIlvl) {
          raidsMap.set(baseName, raid);
        }
      });
      
      const raidUpdatePromises: Promise<unknown>[] = [];

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

          const eligible = raid.gates[0]?.minIlvl === undefined || raid.gates[0].minIlvl <= char.item_level;
          const tracked = eligible ? (backendState?.tracked || false) : false;

          if (!eligible && backendState?.tracked) {
            raidUpdatePromises.push(
              invoke('update_tracking_config', {
                characterId: char.char_id,
                taskId: raid.id,
                tracked: false
              }).catch((err) => {
                console.warn('Failed to clear low ilvl tracking for character', char.char_id, 'raid', raid.id, err);
              })
            );
          }

          return {
            char_id: char.char_id,
            tracked,
            current_value: null, // Raids don't have rested values
            lazy_daily: false
          };
        });

        return {
          raid_id: raid.id,
          raid_name: raid.name, // Only name, no difficulty
          min_ilvl: raid.gates[0]?.minIlvl || 0,
          character_states: characterStates
        };
      });

      if (raidUpdatePromises.length > 0) {
        await Promise.all(raidUpdatePromises);
      }
      
      // Combine everything with proper categorization
      matrixData = {
        characters: baseMatrix.characters,
        daily_tasks: tasksArray.filter((task: any) => 
          task.reset_schedule === 'daily' && 
          (task.content_id === 'chaos' || task.content_id === 'guardian')
        ),
        weekly_tasks: tasksArray.filter((task: any) => task.reset_schedule === 'weekly' && task.category === 'character'),
        roster_tasks: tasksArray.filter((task: any) => task.category === 'roster'),
        raids: raidsArray
      };
      
      if (!matrixData || !matrixData.characters || matrixData.characters.length === 0) {
        console.error('No characters found in matrix data!');
        error = 'No characters found for this roster';
        return;
      }

      await loadRosterEventProgress();
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

  function showWarning(message: string) {
    warningMessage = message;
  }

  function isValidRestedValue(value: number): boolean {
    return Number.isInteger(value) && value >= 0 && value <= 100 && value % 10 === 0;
  }

  function handleRestedWheel(event: WheelEvent) {
    event.preventDefault();
    (event.currentTarget as HTMLInputElement).blur();
  }

  function handleRestedChange(event: Event, charId: number, contentId: string) {
    const input = event.currentTarget as HTMLInputElement;
    const rawValue = input.value.trim();
    const nextValue = Number(rawValue);
    const currentState = matrixData?.daily_tasks
      ?.find((task: any) => task.content_id === contentId)
      ?.character_states
      ?.find((state: any) => state.char_id === charId);
    const previousValue = currentState?.current_value || 0;

    if (rawValue === '' || !Number.isFinite(nextValue) || !isValidRestedValue(nextValue)) {
      input.value = String(previousValue);
      showWarning('Rested values must be 0, 10, 20, ... 100.');
      return;
    }

    warningMessage = '';
    updateRestedValue(charId, contentId, nextValue);
  }

  async function handleRosterEventCountChange(event: Event, taskId: string) {
    const input = event.currentTarget as HTMLInputElement;
    const rawValue = input.value.trim();
    const nextValue = Number(rawValue);
    const progress = rosterEventProgress[taskId];
    const weeklyLimit = progress?.weekly_limit ?? 3;
    const previousValue = progress?.completed_this_week ?? 0;

    if (
      rawValue === '' ||
      !Number.isInteger(nextValue) ||
      nextValue < 0 ||
      nextValue > weeklyLimit
    ) {
      input.value = String(previousValue);
      showWarning(`Event completions must be 0, 1, 2, or 3.`);
      return;
    }

    try {
      warningMessage = '';
      await invoke('update_roster_event_weekly_count', {
        rosterId: $activeRosterId,
        taskId,
        completedCount: nextValue
      });
      await loadRosterEventProgress();
      syncRosterEventTaskState(taskId);
      window.dispatchEvent(new CustomEvent('roster-event-progress-updated', { detail: { taskId } }));
    } catch (err) {
      input.value = String(previousValue);
      showWarning(`Failed to update event completions: ${err}`);
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
      
      // Update local data for the specific task row only
      const updateTaskState = (tasks: any[]) => {
        const task = tasks.find((t: any) => t.content_id === taskId);
        if (task) {
          task.character_states = task.character_states.map((s: any) =>
            s.char_id === charId ? { ...s, tracked: newState } : { ...s }
          );
        }
      };
      
      updateTaskState(matrixData.daily_tasks);
      updateTaskState(matrixData.weekly_tasks);
      updateTaskState(matrixData.roster_tasks);
      matrixData = { ...matrixData };
      
    } catch (err) {
      console.error('Failed to toggle task:', err);
    }
  }

  async function toggleLazyDaily(charId: number, taskId: string, newState: boolean) {
    try {
      await invoke('update_lazy_daily_config', {
        characterId: charId,
        taskId,
        lazyDaily: newState
      });

      const task = matrixData?.daily_tasks?.find((t: any) => t.content_id === taskId);
      if (task) {
        task.character_states = task.character_states.map((s: any) =>
          s.char_id === charId ? { ...s, lazy_daily: newState } : { ...s }
        );
      }
      matrixData = { ...matrixData };
    } catch (err) {
      console.error('Failed to toggle lazy daily:', err);
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
      
      // Update only the targeted raid row for this character
      matrixData.raids = matrixData.raids.map((raid: any) => {
        if (raid.raid_id !== raidId) return raid;
        return {
          ...raid,
          character_states: raid.character_states.map((s: any) =>
            s.char_id === charId ? { ...s, tracked: newState } : { ...s }
          )
        };
      });
      matrixData = { ...matrixData };
      window.dispatchEvent(new CustomEvent('tracking-config-changed', {
        detail: { type: 'raid', characterId: charId, contentId: raidId, tracked: newState }
      }));
       
    } catch (err) {
      console.error('Failed to toggle raid:', err);
    }
  }

  async function toggleRosterTask(taskId: string, newState: boolean) {
    try {
      const characters = matrixData?.characters || [];
      
      for (const char of characters) {
        await invoke('update_tracking_config', {
          characterId: char.char_id,
          taskId: taskId,
          tracked: newState,
          currentValue: null
        });
      }
      
      // Update local data and force reactivity
      matrixData.roster_tasks = matrixData.roster_tasks.map((t: any) => {
        if (t.content_id === taskId) {
          return { ...t, character_states: t.character_states.map((s: any) => ({ ...s, tracked: newState })) };
        }
        return { ...t };
      });
      matrixData = { ...matrixData };
      if (isRosterEventTask(taskId)) {
        await loadRosterEventProgress();
        syncRosterEventTaskState(taskId);
        window.dispatchEvent(new CustomEvent('roster-event-progress-updated', { detail: { taskId } }));
      }
      
    } catch (err) {
      console.error('Failed to toggle roster task:', err);
    }
  }

  async function toggleAllCharactersForTask(taskId: string, newState: boolean) {
    try {
      const characters = matrixData?.characters || [];
      
      for (const char of characters) {
        await invoke('update_tracking_config', {
          characterId: char.char_id,
          taskId: taskId,
          tracked: newState,
          currentValue: null
        });
      }
      
      // Update local data for all task sections and force reactivity
      const updateTaskState = (tasks: any[]) => {
        const task = tasks.find((t: any) => t.content_id === taskId);
        if (task) {
          task.character_states = task.character_states.map((s: any) => ({ ...s, tracked: newState }));
        }
      };
      
      updateTaskState(matrixData.daily_tasks);
      updateTaskState(matrixData.weekly_tasks);
      matrixData = { ...matrixData };
      
    } catch (err) {
      console.error('Failed to toggle all characters for task:', err);
    }
  }

  async function toggleAllLazyDailyForTask(taskId: string, newState: boolean) {
    try {
      const characters = matrixData?.characters || [];

      for (const char of characters) {
        await invoke('update_lazy_daily_config', {
          characterId: char.char_id,
          taskId,
          lazyDaily: newState
        });
      }

      const task = matrixData?.daily_tasks?.find((t: any) => t.content_id === taskId);
      if (task) {
        task.character_states = task.character_states.map((s: any) => ({ ...s, lazy_daily: newState }));
      }
      matrixData = { ...matrixData };
    } catch (err) {
      console.error('Failed to toggle lazy daily for all characters:', err);
    }
  }

  async function toggleAllCharactersForRaid(raidId: string, newState: boolean) {
    try {
      const characters = matrixData?.characters || [];
      
      for (const char of characters) {
        const raid = matrixData.raids.find((r: any) => r.raid_id === raidId);
        if (raid) {
          const charState = raid.character_states.find((s: any) => s.char_id === char.char_id);
          // Only toggle eligible characters (ilvl high enough)
          if (raid.min_ilvl <= char.item_level) {
            await invoke('update_tracking_config', {
              characterId: char.char_id,
              taskId: raidId,
              tracked: newState,
              currentValue: null
            });
          }
        }
      }
      
      // Update local data and force reactivity
      const raid = matrixData.raids.find((r: any) => r.raid_id === raidId);
      if (raid) {
        raid.character_states = raid.character_states.map((s: any, i: number) => {
          const char = matrixData.characters[i];
          if (raid.min_ilvl <= char.item_level) {
            return { ...s, tracked: newState };
          }
          return { ...s };
        });
      }
      matrixData = { ...matrixData };
      window.dispatchEvent(new CustomEvent('tracking-config-changed', {
        detail: { type: 'raid', contentId: raidId, tracked: newState }
      }));
      
    } catch (err) {
      console.error('Failed to toggle all characters for raid:', err);
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

  function areAllCharactersLazyForTask(taskId: string): boolean {
    const task = matrixData?.daily_tasks?.find((t: any) => t.content_id === taskId);
    if (!task || task.character_states.length === 0) return false;
    return task.character_states.every((state: any) => state.lazy_daily === true);
  }

  function supportsLazyDaily(taskId: string): boolean {
    return taskId === 'chaos' || taskId === 'guardian';
  }

  function areAllEligibleCharactersTrackedForRaid(raidId: string): boolean {
    const raid = matrixData?.raids?.find((r: any) => r.raid_id === raidId);
    if (!raid || raid.character_states.length === 0) return false;
    const eligibleStates = raid.character_states.filter((_: any, i: number) => {
      const char = matrixData.characters[i];
      return raid.min_ilvl <= char.item_level;
    });
    if (eligibleStates.length === 0) return false;
    return eligibleStates.every((state: any) => state.tracked === true);
  }

  // Load data when component mounts or roster changes
  $: if ($activeRosterId && $activeRosterId !== lastLoadedRosterId) {
    lastLoadedRosterId = $activeRosterId;
    loadMatrixData();
  }

  onMount(() => {
    const handleRosterEventProgressUpdated = async (event: Event) => {
      const detail = (event as CustomEvent<{ taskId?: string }>).detail;
      await loadRosterEventProgress();
      if (detail?.taskId) {
        syncRosterEventTaskState(detail.taskId);
      }
    };

    window.addEventListener('roster-event-progress-updated', handleRosterEventProgressUpdated);
    return () => {
      window.removeEventListener('roster-event-progress-updated', handleRosterEventProgressUpdated);
    };
  });
</script>

<div class="tracking-settings">
  <RosterButtonGroup />

  {#if warningMessage}
    <div class="status-indicator warning">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <line x1="12" y1="8" x2="12" y2="12"/>
        <line x1="12" y1="16" x2="12.01" y2="16"/>
      </svg>
      {warningMessage}
    </div>
  {/if}

  {#if isLoading}
    <div class="loading">Loading tracking data...</div>
  {:else if error}
    <div class="error">
      <p>{error}</p>
      <button on:click={loadMatrixData}>Retry</button>
    </div>
  {:else if matrixData}
    <div class="matrix-container" data-guide="tracking-matrix">
      <div class="tracking-matrix-wrapper">
        <table class="tracking-matrix">
          <thead>
            <tr class="header-row">
              <th class="sticky-col first-col">
                <div class="matrix-corner-header">
                  <span>Tasks/Character</span>
                  {#if hasHiddenTrackingRows}
                    <button
                      type="button"
                      class:active={collapseUntrackedRows}
                      class="collapse-empty-rows-btn"
                      title={collapseUntrackedRows ? 'Show untracked rows' : 'Hide untracked rows'}
                      on:click={() => setCollapseUntrackedRows(!collapseUntrackedRows)}
                    >
                      {collapseUntrackedRows ? '+' : '-'}
                    </button>
                  {/if}
                </div>
              </th>
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
            <td class="section-title-cell sticky-col first-col">
              <div class="section-title">DAILY</div>
            </td>
            <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
          </tr>
          {#each visibleDailyTasks as task}
            <tr>
              <td class="task-name-cell sticky-col first-col">
                <div class="task-info">
                  <span class="task-name">{task.content_name}</span>
                  <div class="task-actions">
                  <button 
                    class="toggle-all-btn"
                    data-guide="tracking-row-toggle"
                    on:click={() => {
                      const currentState = areAllCharactersTrackedForTask(task.content_id);
                      toggleAllCharactersForTask(task.content_id, !currentState);
                    }}
                    title="Toggle all characters"
                  >
                    {areAllCharactersTrackedForTask(task.content_id) ? '☑' : '☐'}
                  </button>
                  {#if supportsLazyDaily(task.content_id)}
                    <button
                      class="toggle-all-btn lazy-all-btn"
                      on:click={() => {
                        const currentState = areAllCharactersLazyForTask(task.content_id);
                        toggleAllLazyDailyForTask(task.content_id, !currentState);
                      }}
                      title="Toggle lazy behavior for all characters"
                    >
                      Lazy {areAllCharactersLazyForTask(task.content_id) ? 'On' : 'Off'}
                    </button>
                  {/if}
                  </div>
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
                        const newState = (event.currentTarget as HTMLInputElement).checked;
                        toggleTask(char.char_id, task.content_id, newState);
                      }}
                    />
                    {#if task.max_rest_value && (task.content_id === 'chaos' || task.content_id === 'guardian')}
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
                          on:wheel={handleRestedWheel}
                          on:change={(event) => handleRestedChange(event, char.char_id, task.content_id)}
                        />
                      </div>
                      <label class="lazy-toggle" title="Only count this daily when rested is 20 or higher">
                        <input
                          type="checkbox"
                          checked={getCharacterTaskState(task, char.char_id)?.lazy_daily || false}
                          on:change={(event) => {
                            const newState = (event.currentTarget as HTMLInputElement).checked;
                            toggleLazyDaily(char.char_id, task.content_id, newState);
                          }}
                        />
                        <span>Lazy</span>
                      </label>
                    {/if}
                  </div>
                </td>
              {/each}
            </tr>
          {/each}

          <!-- Section 2: WEEKLY -->
          <tr class="section-separator">
            <td class="section-title-cell sticky-col first-col">
              <div class="section-title">WEEKLY</div>
            </td>
            <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
          </tr>
          {#each visibleWeeklyTasks as task}
            <tr>
              <td class="task-name-cell sticky-col first-col">
                <div class="task-info">
                  <span class="task-name">{task.content_name}</span>
                  <button 
                    class="toggle-all-btn"
                    data-guide="tracking-row-toggle"
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
                        const newState = (event.currentTarget as HTMLInputElement).checked;
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
            <td class="section-title-cell sticky-col first-col">
              <div class="section-title">ROSTER WIDE</div>
            </td>
            <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
          </tr>
          {#each visibleRosterTasks as task}
            <tr>
              <td class="task-name-cell sticky-col first-col">{task.content_name}</td>
              <td class="toggle-cell roster-toggle-cell" colspan={matrixData.characters.length}>
                <div class="cell-content">
                  <input 
                    type="checkbox" 
                    class="task-checkbox"
                    checked={task.character_states[0]?.tracked || false}
                    on:change={(event) => {
                      const newState = (event.currentTarget as HTMLInputElement).checked;
                      toggleRosterTask(task.content_id, newState);
                    }}
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
                      on:change={(event) => handleRosterEventCountChange(event, task.content_id)}
                      on:wheel={handleRestedWheel}
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
          {/each}

          <!-- Section 4: RAIDS -->
          <tr class="section-separator">
            <td class="section-title-cell sticky-col first-col">
              <div class="section-title">RAIDS</div>
            </td>
            <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
          </tr>
          {#each visibleRaids as raid}
            <tr>
              <td class="task-name-cell sticky-col first-col">
                <div class="raid-info">
                  <span class="raid-name">{raid.raid_name}</span>
                  <span class="raid-ilvl">iLvl: {raid.min_ilvl}</span>
                  <button 
                    class="toggle-all-btn"
                    on:click={() => {
                      const currentState = areAllEligibleCharactersTrackedForRaid(raid.raid_id);
                      toggleAllCharactersForRaid(raid.raid_id, !currentState);
                    }}
                    title="Toggle all characters"
                  >
                    {areAllEligibleCharactersTrackedForRaid(raid.raid_id) ? '☑' : '☐'}
                  </button>
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
                          const newState = (event.currentTarget as HTMLInputElement).checked;
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
  {:else}
    <div class="no-data">
      <p>No tracking data loaded. Please select an active roster above.</p>
    </div>
  {/if}
</div>

<style>
  .tracking-settings {
    display: flex;
    flex-direction: column;
    padding: 0;
    flex: 1 1 0;
    min-height: 0;
    height: 100%;
    overflow: hidden;
  }

  .roster-selector {
    padding: 16px;
    background: var(--md-sys-color-surface);
    border-bottom: 1px solid var(--md-sys-color-outline);
    display: flex;
    align-items: center;
    gap: 12px;
    position: sticky;
    left: 0;
    z-index: 40;
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

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 500;
    border-radius: 6px;
    margin: 8px 16px 0;
    color: var(--md-sys-color-on-surface);
    background: color-mix(in srgb, var(--md-sys-color-surface-variant) 50%, transparent);
  }

  .status-indicator.warning svg {
    color: #ef4444;
    flex: 0 0 auto;
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

  .no-data {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: var(--md-sys-color-on-surface-variant);
    padding: 1rem;
    text-align: center;
  }

  .matrix-container {
    display: flex;
    flex: 1 1 0;
    min-height: 0;
    height: 100%;
    background: var(--md-sys-color-surface);
    border-radius: 12px;
    overflow: hidden;
    border: 1px solid var(--md-sys-color-outline);
  }

  .tracking-matrix-wrapper {
    flex: 1 1 0;
    min-height: 0;
    height: 100%;
    overflow-x: auto;
    overflow-y: auto;
    position: relative;
    container-type: inline-size;
  }

  .tracking-matrix {
    --task-column-width: 200px;
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    font-size: 14px;
    min-width: 800px;
  }

  .header-row th {
    background: var(--md-sys-color-surface-variant);
    padding: 12px 8px;
    text-align: center;
    border-bottom: 2px solid var(--md-sys-color-outline);
    font-weight: 600;
    color: var(--md-sys-color-on-surface-variant);
    position: sticky;
    top: 0;
    z-index: 20;
  }

  .header-row th.first-col {
    z-index: 30;
  }

  .matrix-corner-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.35rem;
  }

  .collapse-empty-rows-btn {
    width: 1.35rem;
    height: 1.35rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 6px;
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.85rem;
    line-height: 1;
    cursor: pointer;
  }

  .collapse-empty-rows-btn.active {
    border-color: var(--md-sys-color-primary);
    color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 10%, var(--md-sys-color-surface-container-high));
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
    background: rgba(255, 107, 53, 0.02);
    border-bottom: 1px solid rgba(255, 107, 53, 0.08);
    padding: 8px 12px;
    font-weight: 600;
    color: rgba(255, 107, 53, 0.7);
    text-align: left;
  }

  .section-separator .section-title-cell {
    background: color-mix(in srgb, var(--md-sys-color-surface-variant) 92%, rgba(255, 107, 53, 0.08));
    min-width: var(--task-column-width);
    position: sticky;
    left: 0;
    z-index: 18;
  }

  .section-fill-cell {
    min-width: 0;
  }

  .section-title {
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    padding: 0;
    font-size: 16px;
    font-weight: 700;
  }

  .task-name-cell {
    background: var(--md-sys-color-surface-variant);
    padding: 12px 8px;
    border-bottom: 1px solid var(--md-sys-color-outline);
    font-weight: 500;
    min-width: var(--task-column-width);
  }

  .task-info {
    display: flex;
    flex-direction: column;
    gap: 8px;
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

  .lazy-all-btn {
    font-size: 11px;
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

  .lazy-toggle {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
    user-select: none;
  }

  .lazy-toggle input {
    width: 14px;
    height: 14px;
    accent-color: var(--md-sys-color-primary);
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
    min-width: var(--task-column-width);
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
