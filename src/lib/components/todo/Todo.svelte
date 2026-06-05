<script lang="ts">
  // To Do owns matrix loading, RAT roster shaping, and optimistic write-back.
  // The visible table lives in `todo/TodoMatrixTable.svelte`.
  import { onMount, tick } from 'svelte';
  import { activeRosterId, activeFilterCharId, rosters } from '$lib/store';
  import { GAME_TASKS } from '$lib/data/tasks';
  import { encounterMap } from '$lib/data/encounters';
  import TodoMatrixTable from '$lib/components/todo/TodoMatrixTable.svelte';
  import {
    VIRTUAL_RAT_ROSTER_ID,
    buildRaidGateCompletionRequests,
    buildRaidConfigMap,
    buildRosterTaskStates,
    buildTodoTasks,
    buildTrackedTodoRaids,
    filterTodoMatrixCharacters,
    getTrackedTodoRaidCandidates,
    isRosterEventTask
  } from '$lib/components/todo/helpers';
  import type {
    CharacterTaskState,
    RaidConfigEntry,
    RaidGateDifficultyMap,
    RosterEventProgress,
    TodoCharacter,
    TodoMatrixResponse,
    TodoTask
  } from '$lib/components/todo/types';
  import { splitRatTodoView } from '$lib/services/todo-preferences';
  import {
    loadRaidConfigsForRoster,
    loadRaidGateCompletionsBulk,
    loadTodoMatrixForRoster,
    loadTodoRosterEventProgress,
    updateTodoRaidGateStatus,
    updateTodoRosterEventStatus,
    updateTodoRosterTaskStatus,
    updateTodoTaskStatus
  } from '$lib/services/todo';
  
  export let highlightCharId: number | null = null;

  let matrixData: TodoMatrixResponse | null = null;
  let rosterTaskStates: Record<string, boolean> = {};
  let rosterEventProgress: Record<string, RosterEventProgress> = {};
  let raidConfigMap: RaidGateDifficultyMap = new Map();
  let loading = true;
  let error = '';
  let selectedTodoRosterId = '';
  let lastActiveRosterId = '';
  let lastHighlightRequestKey = '';
  let highlightClearTimer: ReturnType<typeof setTimeout> | null = null;

  $: todoRosterOptions = $splitRatTodoView
    ? [{ id: VIRTUAL_RAT_ROSTER_ID, roster_name: 'RAT' }]
    : [];
  $: effectiveTodoRosterId = $splitRatTodoView
    ? (selectedTodoRosterId || $activeRosterId)
    : $activeRosterId;
  $: rosterCount = $rosters.length;

  $: if ($activeRosterId && (!selectedTodoRosterId || (selectedTodoRosterId === lastActiveRosterId && $activeRosterId !== lastActiveRosterId))) {
    selectedTodoRosterId = $activeRosterId;
    lastActiveRosterId = $activeRosterId;
  }

  $: if (!$splitRatTodoView && selectedTodoRosterId === VIRTUAL_RAT_ROSTER_ID) {
    selectedTodoRosterId = $activeRosterId;
  }
  
  async function loadMatrix() {
    try {
      loading = true;
      error = '';
      const rosterId = effectiveTodoRosterId;
      const isRatView = $splitRatTodoView && rosterId === VIRTUAL_RAT_ROSTER_ID;
      // RAT mode is a virtual roster assembled from non-gold earners across all rosters.
      let { baseMatrix, raidConfigs } = isRatView
        ? await loadRatTodoMatrixSources()
        : {
            baseMatrix: await loadTodoMatrixForRoster(rosterId),
            raidConfigs: await loadRaidConfigsForRoster(rosterId)
          };
      if ($splitRatTodoView && !isRatView) {
        const goldCharacterIds = new Set(baseMatrix.characters.filter((character) => character.earns_gold).map((character) => character.id));
        baseMatrix = filterTodoMatrixCharacters(baseMatrix, goldCharacterIds);
        raidConfigs = raidConfigs.filter((config) => goldCharacterIds.has(config.char_id));
      }
      
      raidConfigMap = buildRaidConfigMap(raidConfigs);
      rosterEventProgress = {};
      
      // Roster tasks are stored against the first character in the roster matrix.
      rosterTaskStates = buildRosterTaskStates(baseMatrix);

      if (!isRatView) {
        const rosterEvents = Object.values(GAME_TASKS).filter((task: any) => isRosterEventTask(task.id));
        await Promise.all(rosterEvents.map(async (task: any) => {
          const progress = await loadTodoRosterEventProgress(rosterId, task.id);
          rosterEventProgress[task.id] = progress;
          rosterTaskStates[task.id] = progress.completed_this_week >= progress.weekly_limit;
        }));
        rosterEventProgress = { ...rosterEventProgress };
      }
      
      const { dailyTasks, rosterTasks, weeklyTasks } = buildTodoTasks(baseMatrix, isRatView);
      const candidateRaids = getTrackedTodoRaidCandidates(baseMatrix);

      // Build one bulk request for raid gate completion.
      // Always use the canonical "Gate N" string as gate_id so the session_id
      // written by encounter-sync ("<raidId>_Gate N") and manual toggle match.
      const gateCompletionRequests = buildRaidGateCompletionRequests(candidateRaids, baseMatrix, raidConfigMap);

      const gateCompletionResponses = await loadRaidGateCompletionsBulk(gateCompletionRequests);

      const gateCompletionMap = new Map<string, { completed: boolean; actualDifficulty?: string | null }>();
      gateCompletionResponses.forEach((response) => {
        const completionKey = `${response.character_id}_${response.raid_id}_${response.gate_id}`;
        gateCompletionMap.set(completionKey, {
          completed: response.completed,
          actualDifficulty: response.actual_difficulty
        });
      });

      const trackedRaids = buildTrackedTodoRaids(candidateRaids, baseMatrix, raidConfigMap, gateCompletionMap);
      
      matrixData = {
        characters: baseMatrix.characters,
        daily_tasks: dailyTasks,
        roster_tasks: rosterTasks,
        weekly_tasks: weeklyTasks,
        raids: trackedRaids,
        character_states: baseMatrix.character_states,
        rested_entries: baseMatrix.rested_entries,
        todo_entries: baseMatrix.todo_entries
      };
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load todo matrix';
      console.error('Failed to load todo matrix:', err);
    } finally {
      loading = false;
    }
  }
  
  $: if (highlightCharId && matrixData) {
    const requestKey = `${highlightCharId}:${effectiveTodoRosterId}:${matrixData.characters.map((character) => character.id).join(',')}`;
    if (requestKey !== lastHighlightRequestKey) {
      lastHighlightRequestKey = requestKey;
      focusHighlightedCharacter(highlightCharId);
    }
  }

  async function focusHighlightedCharacter(characterId: number) {
    if (!matrixData) return;

    const headerIndex = matrixData.characters.findIndex(char => char.id === characterId);
    if (headerIndex === -1) {
      if ($splitRatTodoView && effectiveTodoRosterId !== VIRTUAL_RAT_ROSTER_ID) {
        selectedTodoRosterId = VIRTUAL_RAT_ROSTER_ID;
      }
      return;
    }

    await tick();

    const scrollContainer = document.querySelector<HTMLElement>('.todo-matrix-wrapper');
    const headerElements = document.querySelectorAll<HTMLElement>('.char-header');
    const targetHeader = headerElements[headerIndex];

    if (targetHeader && scrollContainer) {
      const headerRect = targetHeader.getBoundingClientRect();
      const containerRect = scrollContainer.getBoundingClientRect();
      const targetCenter = headerRect.left - containerRect.left + scrollContainer.scrollLeft + headerRect.width / 2;
      const nextScrollLeft = Math.max(targetCenter - scrollContainer.clientWidth / 2, 0);

      scrollContainer.scrollTo({ left: nextScrollLeft, behavior: 'auto' });
    }

    if (highlightClearTimer) {
      clearTimeout(highlightClearTimer);
    }

    highlightClearTimer = setTimeout(() => {
      if (highlightCharId === characterId) {
        activeFilterCharId.set(null);
      }
      lastHighlightRequestKey = '';
    }, 2500);
  }
  
  async function loadRatTodoMatrixSources(): Promise<{ baseMatrix: TodoMatrixResponse; raidConfigs: RaidConfigEntry[] }> {
    // Merge every roster locally, then keep only non-gold earners for the RAT view.
    const rosterIds = $rosters.map((roster) => roster.id).filter(Boolean);
    const rosterPayloads = await Promise.all(rosterIds.map(async (rosterId) => {
      const [matrix, raidConfigs] = await Promise.all([
        loadTodoMatrixForRoster(rosterId),
        loadRaidConfigsForRoster(rosterId)
      ]);
      return { matrix, raidConfigs };
    }));

    const ratCharacters = rosterPayloads.flatMap(({ matrix }) =>
      (matrix.characters || []).filter((character) => !character.earns_gold)
    );
    const ratCharacterIds = new Set(ratCharacters.map((character) => character.id));
    const characterStates: Record<string, any> = {};
    const restedEntries: Array<[number, string, number]> = [];
    const todoEntries: Array<[number, string, boolean]> = [];

    for (const { matrix } of rosterPayloads) {
      for (const [key, state] of Object.entries(matrix.character_states || {})) {
        const charId = Number(key.split('_')[0]);
        if (ratCharacterIds.has(charId)) characterStates[key] = state;
      }
      restedEntries.push(...(matrix.rested_entries || []).filter(([charId]) => ratCharacterIds.has(charId)));
      todoEntries.push(...(matrix.todo_entries || []).filter(([charId]) => ratCharacterIds.has(charId)));
    }

    return {
      baseMatrix: {
        characters: ratCharacters,
        daily_tasks: [],
        roster_tasks: [],
        weekly_tasks: [],
        raids: [],
        character_states: characterStates,
        rested_entries: restedEntries,
        todo_entries: todoEntries
      },
      raidConfigs: rosterPayloads
        .flatMap(({ raidConfigs }) => raidConfigs)
        .filter((config) => ratCharacterIds.has(config.char_id))
    };
  }

  function selectTodoRoster(event: CustomEvent<string>) {
    selectedTodoRosterId = event.detail;
    if (event.detail !== VIRTUAL_RAT_ROSTER_ID) {
      activeRosterId.set(event.detail);
      lastActiveRosterId = event.detail;
    }
  }

  function isTaskCompleted(charId: number, taskId: string, isRoster: boolean = false): boolean {
    if (!matrixData?.todo_entries) return false;

    // For roster tasks, check using the first character's ID since roster tasks
    // are stored with the first character's char_id in the new implementation
    const searchId = isRoster && matrixData.characters.length > 0 ? matrixData.characters[0].id : charId;

    const entry = matrixData.todo_entries.find(
      (e) => e[0] === searchId && e[1] === taskId
    );
    
    return entry ? entry[2] : false;
  }
  
  function getCharacterTaskState(task: TodoTask, character: TodoCharacter): CharacterTaskState | undefined {
    const key = `${character.id}_${task.id}`;
    const backendState = matrixData?.character_states?.[key];
    
    let restedValue = undefined;
    if (matrixData?.rested_entries && task.logic_type === 'rested') {
      const restedEntry = matrixData.rested_entries.find(([charId, contentId]) => 
        charId === character.id && contentId === task.id
      );
      restedValue = restedEntry?.[2];
    }
    
    // Check if character is tracked for this task
    const isTracked = backendState?.tracked || false;
    
    // Check if character's item level is too low for this task
    let ilvlTooLow = false;
    const gameTask = GAME_TASKS[task.id];
    if (gameTask?.min_ilvl && character.ilvl) {
      ilvlTooLow = character.ilvl < gameTask.min_ilvl;
    }
    
    return {
      tracked: isTracked,
      completed: backendState?.completed || false,
      rested_value: restedValue,
      ilvl_too_low: ilvlTooLow,
    };
  }
  
  async function handleTaskToggle(characterId: number, taskId: string, currentState: CharacterTaskState) {
    try {
      await updateTodoTaskStatus(characterId, taskId, !currentState.completed);
      
      // Update local state immediately instead of full reload
      const key = `${characterId}_${taskId}`;
      if (matrixData?.character_states) {
        matrixData.character_states[key] = {
          tracked: currentState.tracked,
          completed: !currentState.completed
        };
        
        // Rested tasks consume 20 rest on completion so the UI mirrors backend reset math immediately.
        if ((taskId === 'chaos' || taskId === 'guardian') && !currentState.completed) {
          const restedKey = matrixData.rested_entries?.find(([charId, contentId]) => 
            charId === characterId && contentId === taskId
          );
          const currentRested = restedKey?.[2] || 0;
          
          if (currentRested >= 20) {
            const newRested = currentRested - 20;
            
            if (matrixData?.rested_entries) {
              const restedIndex = matrixData.rested_entries.findIndex(([charId, contentId]) => 
                charId === characterId && contentId === taskId
              );
              if (restedIndex !== -1) {
                matrixData.rested_entries[restedIndex][2] = newRested;
              }
            }
            
          }
        }
        
        // Force reactivity while maintaining type safety
        matrixData = {
          characters: matrixData.characters || [],
          daily_tasks: matrixData.daily_tasks || [],
          roster_tasks: matrixData.roster_tasks || [],
          weekly_tasks: matrixData.weekly_tasks || [],
          raids: matrixData.raids || [],
          character_states: matrixData.character_states,
          rested_entries: matrixData.rested_entries,
          todo_entries: matrixData.todo_entries
        };
      }

      window.dispatchEvent(new CustomEvent('todo-task-status-changed', {
        detail: { characterId, taskId, completed: !currentState.completed }
      }));
      
    } catch (err) {
      console.error('Failed to update task status:', err);
      // Fallback to full reload if local update fails
      await loadMatrix();
    }
  }
  
  async function handleRosterTaskToggle(taskId: string) {
    try {
      const currentCompleted = rosterTaskStates[taskId] || false;
      
      // Allow both completion and uncompletion of roster tasks
      const newState = !currentCompleted;
      rosterTaskStates[taskId] = newState;
      
      // Update reactive state object to trigger reactivity
      rosterTaskStates = { ...rosterTaskStates, [taskId]: newState };
      
      await updateTodoRosterTaskStatus($activeRosterId, taskId, newState);
      
      // Update local state immediately instead of full reload
      // Update matrix data for all characters in roster
      const currentMatrix = matrixData;
      if (currentMatrix?.character_states && currentMatrix.characters) {
        currentMatrix.characters.forEach(character => {
          const key = `${character.id}_${taskId}`;
          currentMatrix.character_states![key] = {
            tracked: true, // Roster tasks are always tracked
            completed: newState
          };
        });
        
        // Force reactivity while maintaining type safety
        matrixData = {
          characters: currentMatrix.characters,
          daily_tasks: currentMatrix.daily_tasks || [],
          roster_tasks: currentMatrix.roster_tasks || [],
          weekly_tasks: currentMatrix.weekly_tasks || [],
          raids: currentMatrix.raids || [],
          character_states: currentMatrix.character_states,
          rested_entries: currentMatrix.rested_entries,
          todo_entries: currentMatrix.todo_entries
        };
      }
      
    } catch (err) {
      console.error('Failed to update roster task status:', err);
      // Fallback to full reload if local update fails
      await loadMatrix();
    }
  }
  
  async function handleRaidGateToggle(characterId: number, raidId: string, gateId: string) {
    try {
      const currentMatrix = matrixData;
      if (!currentMatrix) return;

      // contentId is always the raid_id - the gate is identified by gateId ("Gate N")
      // The session_id written to DB is "<raidId>_<gateId>" e.g. "shadow_serca_Gate 1"
      const contentId = raidId;
      
      // Get current state from raids structure
      const raidIdx = currentMatrix.raids.findIndex(r => r.id === raidId);
      
      if (raidIdx >= 0) {
        // Find the correct character index based on characterId using matrixData.characters
        const charIdx = currentMatrix.characters.findIndex(c => c.id === characterId);
        
        if (charIdx >= 0) {
          const raidState = currentMatrix.raids[raidIdx].character_states?.[charIdx];
          if (!raidState) return;

          const gateNumIndex = parseInt(gateId.split(' ')[1]) || 0;
          const currentGateState = raidState.gate_states?.[gateNumIndex - 1] ?? false;
          
          const newGateState = !currentGateState;
          
          // Optimistic gate updates keep the large matrix from reloading and jumping scroll position.
          const newGateStates = [...(raidState.gate_states || [])];
          newGateStates[gateNumIndex - 1] = newGateState;
          
          const updatedRaidState = {
            ...raidState,
            gate_states: newGateStates
          };
          
          // Update raids structure directly with array spread for reactivity
          const newRaids = [...currentMatrix.raids];
          const newCharacterStates = [...newRaids[raidIdx].character_states];
          newCharacterStates[charIdx] = updatedRaidState;
          newRaids[raidIdx] = {
            ...newRaids[raidIdx],
            character_states: newCharacterStates
          };
          
          // Force reactivity with new array
          matrixData = {
            characters: currentMatrix.characters,
            daily_tasks: currentMatrix.daily_tasks || [],
            roster_tasks: currentMatrix.roster_tasks || [],
            weekly_tasks: currentMatrix.weekly_tasks || [],
            raids: newRaids,
            character_states: currentMatrix.character_states,
            rested_entries: currentMatrix.rested_entries,
            todo_entries: currentMatrix.todo_entries
          };
          
          try {
            await updateTodoRaidGateStatus(characterId, raidId, gateId, contentId, newGateState);
          } catch (invokeErr) {
            console.error('Failed to update raid gate status:', invokeErr);
            // Revert local state if backend fails
            const revertedGateStates = [...newGateStates];
            revertedGateStates[gateNumIndex - 1] = currentGateState;
            
            const revertedRaidState = {
              ...raidState,
              gate_states: revertedGateStates
            };
            
            const revertedCharacterStates = [...newCharacterStates];
            revertedCharacterStates[charIdx] = revertedRaidState;
            
            const activeMatrix = matrixData || currentMatrix;
            const revertedRaids = [...activeMatrix.raids];
            revertedRaids[raidIdx] = {
              ...revertedRaids[raidIdx],
              character_states: revertedCharacterStates
            };
            
            matrixData = {
              characters: activeMatrix.characters,
              daily_tasks: activeMatrix.daily_tasks || [],
              roster_tasks: activeMatrix.roster_tasks || [],
              weekly_tasks: activeMatrix.weekly_tasks || [],
              raids: revertedRaids,
              character_states: activeMatrix.character_states,
              rested_entries: activeMatrix.rested_entries,
              todo_entries: activeMatrix.todo_entries
            };
          }
          
          return;
        }
      }
    } catch (err) {
      console.error('Failed to update raid gate status:', err);
    }
  }

  async function handleRosterEventToggle(taskId: string) {
    try {
      const progress = rosterEventProgress[taskId];
      if (!progress) return;

      const newState = !progress.completed_today;

      await updateTodoRosterEventStatus($activeRosterId, taskId, newState);

      const updatedProgress = await loadTodoRosterEventProgress($activeRosterId, taskId);

      rosterEventProgress = { ...rosterEventProgress, [taskId]: updatedProgress };
      rosterTaskStates = {
        ...rosterTaskStates,
        [taskId]: updatedProgress.completed_this_week >= updatedProgress.weekly_limit
      };
      window.dispatchEvent(new CustomEvent('roster-event-progress-updated', { detail: { taskId, source: 'todo' } }));

      if (matrixData?.character_states && matrixData.characters) {
        matrixData.characters.forEach(character => {
          matrixData!.character_states![`${character.id}_${taskId}`] = {
            tracked: true,
            completed: updatedProgress.completed_this_week >= updatedProgress.weekly_limit
          };
        });
        matrixData = { ...matrixData };
      }
    } catch (err) {
      console.error('Failed to update roster event status:', err);
      await loadMatrix();
    }
  }
  
    
  onMount(() => {
    const handleRosterEventProgressUpdated = (event: Event) => {
      const detail = (event as CustomEvent<{ source?: string }>).detail;
      if (detail?.source === 'todo') return;
      loadMatrix();
    };
    const handleRaidCompleted = () => {
      loadMatrix();
    };

    window.addEventListener('roster-event-progress-updated', handleRosterEventProgressUpdated);
    window.addEventListener('raid-completed', handleRaidCompleted);
    loadMatrix();

    return () => {
      window.removeEventListener('roster-event-progress-updated', handleRosterEventProgressUpdated);
      window.removeEventListener('raid-completed', handleRaidCompleted);
      if (highlightClearTimer) {
        clearTimeout(highlightClearTimer);
      }
    };
  });
  
  $: if (effectiveTodoRosterId && (effectiveTodoRosterId !== VIRTUAL_RAT_ROSTER_ID || rosterCount > 0)) {
    loadMatrix();
  }
</script>

<div class="todo-container" data-guide="todo">
  <!-- Todo Matrix -->
  {#if loading}
    <div class="loading-state">
      <div class="loading-spinner"></div>
      <p>Loading todo matrix...</p>
    </div>
  {:else if error}
    <div class="error-state">
      <p>{error}</p>
      <button on:click={loadMatrix}>Retry</button>
    </div>
  {:else if matrixData}
    <TodoMatrixTable
      {matrixData}
      {effectiveTodoRosterId}
      {todoRosterOptions}
      {highlightCharId}
      splitRatTodoView={$splitRatTodoView}
      virtualRatRosterId={VIRTUAL_RAT_ROSTER_ID}
      {rosterTaskStates}
      {rosterEventProgress}
      {raidConfigMap}
      onSelectTodoRoster={selectTodoRoster}
      {getCharacterTaskState}
      {isRosterEventTask}
      onTaskToggle={handleTaskToggle}
      onRosterTaskToggle={handleRosterTaskToggle}
      onRosterEventToggle={handleRosterEventToggle}
      onRaidGateToggle={handleRaidGateToggle}
    />
  {:else}
    <div class="empty-state">
      <div class="empty-icon">Users</div>
      <h3>No Characters Found</h3>
      <p>Add a roster and characters to get started with your daily tasks!</p>
    </div>
  {/if}
</div>
<style>
  .todo-container {
    --app-control-accent: var(--app-todo-accent);
    --app-control-on-accent: var(--md-sys-color-on-primary);
    --app-control-accent-container: var(--app-todo-accent-soft);
    --app-control-hover-border: var(--app-todo-accent);
    --app-matrix-ilvl-color: var(--app-todo-matrix-ilvl-color);
    --app-matrix-cp-color: var(--app-todo-matrix-cp-color);
    padding: 1rem;
    display: flex;
    flex-direction: column;
    flex: 1 1 0;
    min-height: 0;
  }

  .loading-state,
  .error-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    text-align: center;
  }

  .loading-spinner {
    width: 40px;
    height: 40px;
    border: 3px solid color-mix(in srgb, var(--md-sys-color-primary) 20%, transparent);
    border-top: 3px solid var(--md-sys-color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .empty-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  .error-state button {
    margin-top: 1rem;
    padding: 0.5rem 1rem;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
</style>
