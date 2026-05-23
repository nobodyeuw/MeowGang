<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { isTaskAvailable } from '../utils/availability';
  import Countdown from './Countdown.svelte';
  import { activeRosterId, activeFilterCharId, rosters } from '$lib/store';
  import { GAME_TASKS } from '$lib/data/tasks';
  import { RAIDS } from '$lib/data/raids';
  import { GAME_CLASSES } from '$lib/data/classes';
  import { encounterMap } from '$lib/data/encounters';
  import RosterButtonGroup from '$lib/components/common/RosterButtonGroup.svelte';
  import { splitRatTodoView } from '$lib/services/todo-preferences';
  
  export let highlightCharId: number | null = null;
  
  // Types
  interface TodoCharacter {
    id: number;
    name: string;
    class: string;
    ilvl?: number;
    combat_power?: number;
    earns_gold?: boolean;
    display_order?: string;
  }
  
  interface TodoTask {
    id: string;
    name: string;
    category: string;
    reset_schedule: string;
    logic_type: string;
    max_rest_value?: number;
    min_ilvl?: number;
    character_states: CharacterTaskState[];
  }
  
  interface CharacterTaskState {
    tracked: boolean;
    completed: boolean;
    rested_value?: number;
    ilvl_too_low: boolean;
  }
  
  interface TodoRaid {
    id: string;
    raid_name: string;
    difficulty: string;
    gates: Array<{ gate: string; name: string; min_ilvl?: number }>;
    character_states: Array<{
      tracked: boolean;
      gate_states: boolean[];
      ilvl_too_low: boolean;
      difficulty?: string;
    }>;
  }
  
  interface TodoMatrixResponse {
    characters: TodoCharacter[];
    daily_tasks: TodoTask[];
    roster_tasks: TodoTask[];
    weekly_tasks: TodoTask[];
    raids: TodoRaid[];
    character_states?: Record<string, any>;
    rested_entries?: Array<[number, string, number]>;
    todo_entries?: Array<[number, string, boolean]>;
  }

  interface RaidGateCompletionRequest {
    character_id: number;
    raid_id: string;
    gate_id: string;
    difficulty: string;
  }

  interface RaidGateCompletionResponse {
    character_id: number;
    raid_id: string;
    gate_id: string;
    completed: boolean;
  }

  interface RaidConfigEntry {
    char_id: number;
    content_id: string;
    difficulty: string;
  }

  interface RosterEventProgress {
    task_id: string;
    completed_this_week: number;
    weekly_limit: number;
    completed_today: boolean;
    available: boolean;
  }
  
  // State
  let matrixData: TodoMatrixResponse | null = null;
  let rosterTaskStates: Record<string, boolean> = {};
  let rosterEventProgress: Record<string, RosterEventProgress> = {};
  let raidConfigMap: Map<string, Map<number, string>> = new Map();
  let loading = true;
  let error = '';
  let selectedTodoRosterId = '';
  let lastActiveRosterId = '';
  let lastHighlightRequestKey = '';
  let highlightClearTimer: ReturnType<typeof setTimeout> | null = null;

  const VIRTUAL_RAT_ROSTER_ID = '__todo_virtual_rat__';

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
  
  // Functions
  async function loadMatrix() {
    try {
      loading = true;
      error = '';
      const rosterId = effectiveTodoRosterId;
      const isRatView = $splitRatTodoView && rosterId === VIRTUAL_RAT_ROSTER_ID;
      let { baseMatrix, raidConfigs } = isRatView
        ? await loadRatTodoMatrixSources()
        : {
            baseMatrix: await invoke<TodoMatrixResponse>('get_todo_matrix', { rosterId }),
            raidConfigs: await invoke<RaidConfigEntry[]>('get_raid_configs_for_roster', { rosterId })
          };
      if ($splitRatTodoView && !isRatView) {
        const goldCharacterIds = new Set(baseMatrix.characters.filter((character) => character.earns_gold).map((character) => character.id));
        baseMatrix = filterTodoMatrixCharacters(baseMatrix, goldCharacterIds);
        raidConfigs = raidConfigs.filter((config) => goldCharacterIds.has(config.char_id));
      }
      
      // Initialize raid config map
      raidConfigMap = new Map();
      
      // Initialize roster task states
      rosterTaskStates = {};
      rosterEventProgress = {};
      
      // Debug logging for baseMatrix structure
      console.log(`DEBUG BASEMATRIX: character_states exists=${!!baseMatrix.character_states}, characters count=${baseMatrix.characters?.length || 0}`);
      
      // Initialize roster task states from GAME_TASKS
      Object.values(GAME_TASKS).forEach((task: any) => {
        if (task.category === 'roster') {
          // Check for roster task completion using character_states
          let taskCompleted = false;
          if (baseMatrix.character_states && baseMatrix.characters.length > 0) {
            const firstCharId = baseMatrix.characters[0].id;
            const key = `${firstCharId}_${task.id}`;
            const state = baseMatrix.character_states[key];
            taskCompleted = state ? state.completed : false;
            
            // Debug logging for roster task loading
            console.log(`DEBUG ROSTER LOAD: task=${task.id}, key=${key}, state=`, state, `taskCompleted=${taskCompleted}`);
          } else {
            console.log(`DEBUG ROSTER LOAD: No character_states or characters for task=${task.id}`);
          }
          
          rosterTaskStates[task.id] = taskCompleted;
        }
      });

      if (!isRatView) {
        const rosterEvents = Object.values(GAME_TASKS).filter((task: any) => isRosterEventTask(task.id));
        await Promise.all(rosterEvents.map(async (task: any) => {
          const progress = await invoke<RosterEventProgress>('get_roster_event_progress', {
            rosterId,
            taskId: task.id
          });
          rosterEventProgress[task.id] = progress;
          rosterTaskStates[task.id] = progress.completed_this_week >= progress.weekly_limit;
        }));
        rosterEventProgress = { ...rosterEventProgress };
      }
      
      raidConfigs.forEach(config => {
        if (!raidConfigMap.has(config.content_id)) {
          raidConfigMap.set(config.content_id, new Map());
        }
        raidConfigMap.get(config.content_id)!.set(config.char_id, config.difficulty);
      });
      
      // Transform tasks from GAME_TASKS
      const allTasks = Object.values(GAME_TASKS).map(task => {
        const characterStates = baseMatrix.characters.map((char: any) => {
          const key = `${char.id}_${task.id}`;
          const backendState = baseMatrix.character_states?.[key];
          
          let restedValue = undefined;
          if (baseMatrix.rested_entries && task.logic_type === 'rested') {
            const restedEntry = baseMatrix.rested_entries.find(([charId, contentId]) => 
              charId === char.id && contentId === task.id
            );
            restedValue = restedEntry?.[2];
          }
          
          return {
            tracked: backendState?.tracked || false,
            completed: backendState?.completed || false,
            rested_value: restedValue,
            ilvl_too_low: false,
          };
        });
        
        return {
          id: task.id,
          name: task.name,
          category: task.category,
          reset_schedule: task.reset_schedule,
          logic_type: task.logic_type,
          max_rest_value: task.max_rest_value,
          character_states: characterStates
        };
      });
      
      // Categorize tasks and filter out fully untracked ones
      const isAnyCharTracked = (task: any) => task.character_states.some((s: any) => s.tracked);
      const dailyTasks = allTasks.filter(task => task.reset_schedule === 'daily' && task.category === 'character' && isAnyCharTracked(task));
      const rosterTasks = isRatView ? [] : allTasks.filter(task => task.category === 'roster' && isAnyCharTracked(task));
      const weeklyTasks = allTasks.filter(task => task.reset_schedule === 'weekly' && task.category === 'character' && isAnyCharTracked(task));
      
      // Transform raids from RAIDS - only show raids that at least one character tracks
      const raidMap = new Map<string, typeof RAIDS[0]>();
      RAIDS.forEach(raid => {
        if (!raidMap.has(raid.id)) {
          raidMap.set(raid.id, raid);
        }
      });
      
      const candidateRaids = Array.from(raidMap.values())
        .filter(raid => {
          return baseMatrix.characters.some((char: any) => {
            const key = `${char.id}_${raid.id}`;
            const backendState = baseMatrix.character_states?.[key];
            return backendState?.tracked || false;
          });
        })
        .sort((a, b) => {
          const aMinIlvl = a.gates[0]?.minIlvl || 0;
          const bMinIlvl = b.gates[0]?.minIlvl || 0;
          return aMinIlvl - bMinIlvl;
        });

      // Build one bulk request for raid gate completion.
      // Always use the canonical "Gate N" string as gate_id so the session_id
      // written by encounter-sync ("<raidId>_Gate N") and manual toggle match.
      const gateCompletionRequests: RaidGateCompletionRequest[] = [];
      candidateRaids.forEach((raid) => {
        baseMatrix.characters.forEach((char: any) => {
          const difficulty = raidConfigMap.get(raid.id)?.get(char.id) || 'Normal';
          raid.gates.forEach((gate: any) => {
            // Use the gate string directly (e.g. "Gate 1") - never resolve to boss name
            gateCompletionRequests.push({
              character_id: char.id,
              raid_id: raid.id,
              gate_id: gate.gate,
              difficulty
            });
          });
        });
      });

      const gateCompletionResponses = await invoke<RaidGateCompletionResponse[]>('get_raid_gate_completions_bulk', {
        requests: gateCompletionRequests
      });

      const gateCompletionMap = new Map<string, boolean>();
      gateCompletionResponses.forEach((response) => {
        const completionKey = `${response.character_id}_${response.raid_id}_${response.gate_id}`;
        gateCompletionMap.set(completionKey, response.completed);
      });

      // Build tracked raids using the same canonical gate_id
      const trackedRaids = await Promise.all(
        candidateRaids.map(async raid => {
          const characterStates = await Promise.all(
            baseMatrix.characters.map(async (char: any) => {
              const key = `${char.id}_${raid.id}`;
              const backendState = baseMatrix.character_states?.[key];
              const difficulty = raidConfigMap.get(raid.id)?.get(char.id) || 'Normal';
              
              const gateStates = raid.gates.map((gate: any) => {
                const completionKey = `${char.id}_${raid.id}_${gate.gate}`;
                return gateCompletionMap.get(completionKey) || false;
              });
              
              return {
                tracked: backendState?.tracked || false,
                gate_states: gateStates,
                ilvl_too_low: false,
                difficulty: difficulty,
              };
            })
          );
          
          return {
            id: raid.id,
            raid_name: raid.name,
            difficulty: raid.difficulty,
            gates: raid.gates.map(gate => ({ gate: gate.gate, name: gate.gate, min_ilvl: gate.minIlvl })),
            character_states: characterStates
          };
        })
      );
      
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

    const scrollContainer = document.querySelector<HTMLElement>('.matrix-content');
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
  
  function getTaskIcon(taskId: string): string {
    if (taskId.startsWith('event_')) {
      return '/images/event_quest.webp';
    }

    const raidIds = RAIDS.map(raid => raid.id);
    if (raidIds.includes(taskId)) {
      return '/images/kazeros-raid.webp';
    }
    
    const iconMap: Record<string, string> = {
      'chaos': '/images/chaos-dungeon.webp',
      'guardian': '/images/guardian.png',
      'gate': '/images/chaos_gate.png',
      'boss': '/images/boss.png',
      'guild': '/images/guild.webp',
      'cube': '/images/ebony1720.png',
      'paradise': '/images/paradise.webp',
      'shop': '/images/daily.webp',
    };
    
    return iconMap[taskId] || '/images/daily.webp';
  }

  async function loadRatTodoMatrixSources(): Promise<{ baseMatrix: TodoMatrixResponse; raidConfigs: RaidConfigEntry[] }> {
    const rosterIds = $rosters.map((roster) => roster.id).filter(Boolean);
    const rosterPayloads = await Promise.all(rosterIds.map(async (rosterId) => {
      const [matrix, raidConfigs] = await Promise.all([
        invoke<TodoMatrixResponse>('get_todo_matrix', { rosterId }),
        invoke<RaidConfigEntry[]>('get_raid_configs_for_roster', { rosterId })
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

  function filterTodoMatrixCharacters(matrix: TodoMatrixResponse, characterIds: Set<number>): TodoMatrixResponse {
    const characterStates: Record<string, any> = {};
    for (const [key, state] of Object.entries(matrix.character_states || {})) {
      const charId = Number(key.split('_')[0]);
      if (characterIds.has(charId)) characterStates[key] = state;
    }

    return {
      ...matrix,
      characters: matrix.characters.filter((character) => characterIds.has(character.id)),
      character_states: characterStates,
      rested_entries: (matrix.rested_entries || []).filter(([charId]) => characterIds.has(charId)),
      todo_entries: (matrix.todo_entries || []).filter(([charId]) => characterIds.has(charId))
    };
  }

  function selectTodoRoster(event: CustomEvent<string>) {
    selectedTodoRosterId = event.detail;
    if (event.detail !== VIRTUAL_RAT_ROSTER_ID) {
      activeRosterId.set(event.detail);
      lastActiveRosterId = event.detail;
    }
  }

  function isRosterEventTask(taskId: string): boolean {
    return taskId === 'event_argeos_winter';
  }
  
  function getClassName(classId: string): string {
    const classInfo = GAME_CLASSES[classId];
    return classInfo ? classInfo.displayName : 'Unknown';
  }
  
  function getClassIcon(classId: string): string {
    const classInfo = GAME_CLASSES[classId];
    return classInfo ? classInfo.iconId : '0';
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
      await invoke('update_task_status', {
        characterId,
        taskId,
        completed: !currentState.completed
      });
      
      // Update local state immediately instead of full reload
      const key = `${characterId}_${taskId}`;
      if (matrixData?.character_states) {
        matrixData.character_states[key] = {
          tracked: currentState.tracked,
          completed: !currentState.completed
        };
        
        // Update rested value for chaos/guardian tasks
        if ((taskId === 'chaos' || taskId === 'guardian') && !currentState.completed) {
          // Chaos/guardian completed - consume 20 rested points if available
          const restedKey = matrixData.rested_entries?.find(([charId, contentId]) => 
            charId === characterId && contentId === taskId
          );
          const currentRested = restedKey?.[2] || 0;
          
          if (currentRested >= 20) {
            const newRested = currentRested - 20;
            
            // Update rested_entries
            if (matrixData?.rested_entries) {
              const restedIndex = matrixData.rested_entries.findIndex(([charId, contentId]) => 
                charId === characterId && contentId === taskId
              );
              if (restedIndex !== -1) {
                matrixData.rested_entries[restedIndex][2] = newRested;
              }
            }
            
            console.log(`Consumed 20 rested points for ${taskId} (character ${characterId}): ${currentRested} -> ${newRested}`);
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
      
      await invoke('update_roster_task_status', {
        rosterId: $activeRosterId,
        taskId,
        completed: newState
      });
      
      // Update local state immediately instead of full reload
      console.log(`DEBUG STATE UPDATE: task=${taskId}, oldState=${currentCompleted}, newState=${newState}`);
      
      // Update matrix data for all characters in roster
      if (matrixData && matrixData.character_states && matrixData.characters) {
        matrixData.characters.forEach(character => {
          const key = `${character.id}_${taskId}`;
          matrixData.character_states![key] = {
            tracked: true, // Roster tasks are always tracked
            completed: newState
          };
        });
        
        // Force reactivity while maintaining type safety
        matrixData = {
          characters: matrixData.characters,
          daily_tasks: matrixData.daily_tasks || [],
          roster_tasks: matrixData.roster_tasks || [],
          weekly_tasks: matrixData.weekly_tasks || [],
          raids: matrixData.raids || [],
          character_states: matrixData.character_states,
          rested_entries: matrixData.rested_entries,
          todo_entries: matrixData.todo_entries
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
      // contentId is always the raid_id - the gate is identified by gateId ("Gate N")
      // The session_id written to DB is "<raidId>_<gateId>" e.g. "shadow_serca_Gate 1"
      const contentId = raidId;
      
      // Get current state from raids structure
      const raidIdx = matrixData?.raids?.findIndex(r => r.id === raidId);
      
      if (raidIdx !== undefined && raidIdx >= 0) {
        // Find the correct character index based on characterId using matrixData.characters
        const charIdx = matrixData?.characters?.findIndex(c => c.id === characterId);
        
        if (charIdx !== undefined && charIdx >= 0) {
          const raidState = matrixData?.raids[raidIdx].character_states?.[charIdx];
          const gateNumIndex = parseInt(gateId.split(' ')[1]) || 0;
          const currentGateState = raidState?.gate_states?.[gateNumIndex - 1];
          
          const newGateState = !currentGateState;
          
          // Update local state immediately for visual feedback
          const newGateStates = [...(raidState.gate_states || [])];
          newGateStates[gateNumIndex - 1] = newGateState;
          
          const updatedRaidState = {
            ...raidState,
            gate_states: newGateStates
          };
          
          // Update raids structure directly with array spread for reactivity
          const newRaids = [...matrixData.raids];
          const newCharacterStates = [...newRaids[raidIdx].character_states];
          newCharacterStates[charIdx] = updatedRaidState;
          newRaids[raidIdx] = {
            ...newRaids[raidIdx],
            character_states: newCharacterStates
          };
          
          // Force reactivity with new array
          matrixData = {
            characters: matrixData.characters,
            daily_tasks: matrixData.daily_tasks || [],
            roster_tasks: matrixData.roster_tasks || [],
            weekly_tasks: matrixData.weekly_tasks || [],
            raids: newRaids,
            character_states: matrixData.character_states,
            rested_entries: matrixData.rested_entries,
            todo_entries: matrixData.todo_entries
          };
          
          try {
            await invoke('update_raid_gate_status', {
              characterId,
              raidId,
              gateId,
              contentId,
              completed: newGateState
            });
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
            
            const revertedRaids = [...matrixData.raids];
            revertedRaids[raidIdx] = {
              ...revertedRaids[raidIdx],
              character_states: revertedCharacterStates
            };
            
            matrixData = {
              characters: matrixData.characters,
              daily_tasks: matrixData.daily_tasks || [],
              roster_tasks: matrixData.roster_tasks || [],
              weekly_tasks: matrixData.weekly_tasks || [],
              raids: revertedRaids,
              character_states: matrixData.character_states,
              rested_entries: matrixData.rested_entries,
              todo_entries: matrixData.todo_entries
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

      await invoke('update_roster_event_status', {
        rosterId: $activeRosterId,
        taskId,
        completed: newState
      });

      const updatedProgress = await invoke<RosterEventProgress>('get_roster_event_progress', {
        rosterId: $activeRosterId,
        taskId
      });

      rosterEventProgress = { ...rosterEventProgress, [taskId]: updatedProgress };
      rosterTaskStates = {
        ...rosterTaskStates,
        [taskId]: updatedProgress.completed_this_week >= updatedProgress.weekly_limit
      };
      window.dispatchEvent(new CustomEvent('roster-event-progress-updated', { detail: { taskId } }));

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
    const handleRosterEventProgressUpdated = () => {
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

<div class="todo-container">
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
    <div class="matrix-content">
      <RosterButtonGroup
        selectedRosterId={effectiveTodoRosterId}
        extraRosters={todoRosterOptions}
        on:select={selectTodoRoster}
      />

      {#if matrixData.characters.length > 0}
        <table class="todo-matrix">
        <thead>
          <tr class="header-row">
            <th class="sticky-col first-col">Tasks/Character</th>
            {#each matrixData.characters as character}
              <th class="char-header sticky-col {character.id === highlightCharId ? 'highlighted' : ''}">
                <div class="char-info">
                  <img 
                    src="/images/classes/{getClassIcon(character.class)}.png" 
                    alt={getClassName(character.class)}
                    class="class-icon"
                    on:error={(e: any) => { e.currentTarget.style.display = 'none'; }}
                  />
                  <div class="char-name-section">
                    <span class="char-name">{character.name}</span>
                    {#if character.earns_gold}
                      <img src="/images/gold.png" alt="Gold Earner" class="gold-earner-icon" />
                    {/if}
                  </div>
                  <div class="char-stats">
                    <div class="stat-pair">
                      <span class="stat-label">iLvl</span>
                      <span class="char-ilvl">{character.ilvl?.toFixed(0) || '0'}</span>
                    </div>
                    <div class="stat-pair">
                      <span class="stat-label cp-label">CP</span>
                      <span class="char-cp">{character.combat_power?.toFixed(0) || '0'}</span>
                    </div>
                  </div>
                </div>
              </th>
            {/each}
          </tr>
        </thead>
        <tbody>
          <!-- DAILY TASKS -->
          {#if matrixData.daily_tasks.length > 0}
          <tr class="section-separator">
            <td class="section-title-cell sticky-col first-col">
              <div class="section-title">DAILY</div>
            </td>
            <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
          </tr>
          {#each matrixData.daily_tasks as task}
            <tr>
              <td class="task-name-cell sticky-col first-col">
                <div class="task-info">
                  <img src={getTaskIcon(task.id)} alt={task.name} class="task-icon" />
                  <span class="task-name">{task.name}</span>
                </div>
              </td>
              {#each matrixData.characters as character}
                {@const state = getCharacterTaskState(task, character)}
                <td class="toggle-cell">
                  <div class="cell-content">
                    {#if state?.tracked && !state?.ilvl_too_low}
                      <div class="task-toggle-container">
                        <button 
                          class="task-toggle"
                          class:completed={state.completed}
                          on:click={() => handleTaskToggle(character.id, task.id, state)}
                        >
                          {#if state.completed}
                            <span class="checkbox checked">✓</span>
                          {:else}
                            <span class="checkbox">◯</span>
                          {/if}
                        </button>
                        {#if task.logic_type === 'rested' && state.rested_value !== undefined}
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
                        <span class="ilvl-warning">⚠️</span>
                      </div>
                    {/if}
                  </div>
                </td>
              {/each}
            </tr>
          {/each}
          {/if}
          
          <!-- ROSTER WIDE TASKS -->
          {#if matrixData.roster_tasks.length > 0}
            <tr class="section-separator">
              <td class="section-title-cell sticky-col first-col">
                <div class="section-title">ROSTER WIDE</div>
              </td>
              <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
            </tr>
            {#each matrixData.roster_tasks as task}
              <tr>
                <td class="task-name-cell sticky-col first-col">
                  <div class="task-info">
                    <img src={getTaskIcon(task.id)} alt={task.name} class="task-icon" />
                    <span class="task-name">{task.name}</span>
                  </div>
                </td>
                <td colspan={matrixData.characters.length} class="roster-task-cell">
                  <div class="roster-toggle-container">
                    {#if isRosterEventTask(task.id)}
                      {@const progress = rosterEventProgress[task.id]}
                      <button
                        class="roster-toggle event-toggle"
                        class:completed={progress?.completed_today}
                        disabled={!progress?.available && !progress?.completed_today}
                        on:click={() => handleRosterEventToggle(task.id)}
                      >
                        <span class="roster-label event-label">
                          {progress ? `${progress.completed_this_week}/${progress.weekly_limit}` : '0/3'}
                        </span>
                        <span class="roster-label">
                          {progress?.completed_today ? 'Completed' : progress?.available ? 'Available' : 'Weekly done'}
                        </span>
                      </button>
                    {:else if isTaskAvailable(task.id)}
                      <button 
                        class="roster-toggle"
                        class:completed={rosterTaskStates[task.id]}
                        on:click={() => {
                          const currentState = rosterTaskStates[task.id];
                          console.log(`DEBUG BUTTON CLICK: task=${task.id}, currentState=${currentState}, finalState=${rosterTaskStates[task.id]}`);
                          handleRosterTaskToggle(task.id);
                        }}
                      >
                        <span class="roster-label">
                          {rosterTaskStates[task.id] ? 'Completed' : 'Available'}
                        </span>
                      </button>
                    {:else}
                      <Countdown taskId={task.id} taskName={task.name} />
                    {/if}
                  </div>
                </td>
              </tr>
            {/each}
          {/if}
          
          <!-- WEEKLY TASKS -->
          {#if matrixData.weekly_tasks.length > 0}
            <tr class="section-separator">
              <td class="section-title-cell sticky-col first-col">
                <div class="section-title">WEEKLY</div>
              </td>
              <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
            </tr>
            {#each matrixData.weekly_tasks as task}
              <tr>
                <td class="task-name-cell sticky-col first-col">
                  <div class="task-info">
                    <img src={getTaskIcon(task.id)} alt={task.name} class="task-icon" />
                    <span class="task-name">{task.name}</span>
                  </div>
                </td>
                {#each matrixData.characters as character}
                  {@const state = getCharacterTaskState(task, character)}
                  <td class="toggle-cell">
                    <div class="cell-content">
                      {#if state?.tracked && !state?.ilvl_too_low}
                        <div class="task-toggle-container">
                          <button 
                            class="task-toggle"
                            class:completed={state.completed}
                            on:click={() => handleTaskToggle(character.id, task.id, state)}
                          >
                            {#if state.completed}
                              <span class="checkbox checked">✓</span>
                            {:else}
                              <span class="checkbox">◯</span>
                            {/if}
                          </button>
                        </div>
                      {:else if state?.ilvl_too_low}
                        <div class="untracked-task">
                          <span class="ilvl-warning">⚠️</span>
                        </div>
                      {/if}
                    </div>
                  </td>
                {/each}
              </tr>
            {/each}
          {/if}
          
          <!-- RAIDS -->
          {#if matrixData.raids.length > 0}
            <tr class="section-separator">
              <td class="section-title-cell sticky-col first-col">
                <div class="section-title">RAIDS</div>
              </td>
              <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
            </tr>
            {#each matrixData.raids as raid}
              <tr>
                <td class="task-name-cell sticky-col first-col">
                  <div class="task-info">
                    <img src={getTaskIcon(raid.id)} alt={raid.raid_name} class="task-icon" />
                    <span class="task-name">{raid.raid_name}</span>
                  </div>
                </td>
                {#each matrixData.characters as character, charIndex}
                  {@const state = raid.character_states[charIndex]}
                  {@const difficulty = raidConfigMap.get(raid.id)?.get(character.id) || 'Normal'}
                  {@const raidIlvlTooLow = raid.gates.some(gate => gate.min_ilvl && character.ilvl && character.ilvl < gate.min_ilvl)}
                  <td class="toggle-cell">
                    <div class="cell-content">
                      {#if state?.tracked && !raidIlvlTooLow}
                        <div class="raid-gates">
                          {#each raid.gates as gate}
                            {@const gateIlvlTooLow = gate.min_ilvl && character.ilvl && character.ilvl < gate.min_ilvl}
                            {@const gateNumber = parseInt(gate.gate.split(' ')[1]) || 0}
                            {@const gateIndex = gateNumber - 1}
                            {@const gateState = state.gate_states[gateIndex]}
                            <button 
                              class="gate-toggle"
                              class:completed={gateState}
                              on:click={() => {
                                handleRaidGateToggle(character.id, raid.id, gate.gate);
                              }}
                              title="{difficulty}"
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
                          <span class="ilvl-warning">⚠️</span>
                        </div>
                      {/if}
                    </div>
                  </td>
                {/each}
              </tr>
            {/each}
          {/if}
        </tbody>
        </table>
      {:else}
        <div class="empty-state todo-empty-view">
          <div class="empty-icon">👥</div>
          <h3>No Characters In This View</h3>
          {#if $splitRatTodoView && effectiveTodoRosterId !== VIRTUAL_RAT_ROSTER_ID}
            <p>This roster currently has no gold earners because RAT characters are separated into the RAT To Do view.</p>
          {:else if $splitRatTodoView && effectiveTodoRosterId === VIRTUAL_RAT_ROSTER_ID}
            <p>No RAT characters found across your rosters.</p>
          {:else}
            <p>Add characters to this roster to get started with your daily tasks.</p>
          {/if}
        </div>
      {/if}
    </div>
  {:else}
    <div class="empty-state">
      <div class="empty-icon">👥</div>
      <h3>No Characters Found</h3>
      <p>Add a roster and characters to get started with your daily tasks!</p>
    </div>
  {/if}
</div>

<style>
  .todo-container {
    padding: 1rem;
    display: flex;
    flex-direction: column;
    flex: 1 1 0;
    min-height: 0;
  }

  .roster-selector {
    margin-bottom: 1rem;
    display: flex;
    align-items: center;
    gap: 1rem;
    position: sticky;
    left: 0;
    z-index: 40;
    width: max-content;
  }

  .roster-selector label {
    font-weight: 600;
    color: #ff6b35;
  }

  .roster-selector select {
    padding: 0.5rem;
    border-radius: 6px;
    border: 2px solid rgba(255, 107, 53, 0.3);
    background: #374151;
    color: #f3f4f6;
    font-size: 1rem;
    min-width: 200px;
    cursor: pointer;
    transition: border-color 0.2s ease;
  }

  .roster-selector select:hover {
    border-color: #ff6b35;
  }

  .roster-selector select:focus {
    outline: none;
    border-color: #ff6b35;
    box-shadow: 0 0 0 3px rgba(255, 107, 53, 0.1);
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
    border: 3px solid rgba(255, 107, 53, 0.2);
    border-top: 3px solid #ff6b35;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .matrix-content {
    flex: 1;
    min-height: 0;
    overflow: auto;
    position: relative;
    container-type: inline-size;
  }

  .todo-matrix {
    --task-column-width: 200px;
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    background: var(--md-sys-color-surface);
  }

  .header-row {
    background: var(--md-sys-color-surface-container);
  }

  .header-row th {
    position: sticky;
    top: 0;
    z-index: 20;
    background: var(--md-sys-color-surface-container);
  }

  .header-row th.first-col {
    z-index: 30;
  }

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

  .char-header {
    min-width: 120px;
    text-align: center;
  }

  .char-header.highlighted {
    background:
      linear-gradient(180deg, rgba(255, 168, 76, 0.24), rgba(255, 107, 53, 0.1)),
      var(--md-sys-color-surface-container);
    box-shadow:
      inset 0 3px 0 rgba(255, 168, 76, 0.85),
      inset 0 -18px 28px rgba(255, 107, 53, 0.08),
      0 8px 22px rgba(255, 107, 53, 0.12);
  }

  .char-header.highlighted .char-name {
    color: #ffb45f !important;
    text-shadow: 0 1px 10px rgba(255, 107, 53, 0.3);
  }

  .char-info {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
  }

  .char-name-section {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .class-icon {
    width: 24px;
    height: 24px;
    border-radius: 50%;
  }

  .char-name {
    font-weight: 600;
    font-size: 0.875rem;
  }

  .gold-earner-icon {
    width: 14px;
    height: 14px;
    object-fit: contain;
  }

  .char-stats {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    justify-content: center;
    font-size: 0.75rem;
    opacity: 0.8;
  }

  .stat-pair {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    white-space: nowrap;
  }

  .stat-label {
    font-size: 0.625rem;
    color: var(--md-sys-color-on-surface-variant);
    opacity: 0.6;
  }

  .cp-label {
    color: rgba(255, 107, 53, 0.6);
  }

  .char-ilvl {
    font-weight: 500;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.625rem;
  }

  .char-cp {
    font-weight: 500;
    color: rgba(255, 107, 53, 0.8);
    font-size: 0.625rem;
  }

  .section-separator td {
    background: rgba(255, 107, 53, 0.02);
    border-bottom: 1px solid rgba(255, 107, 53, 0.08);
    padding: 0.5rem;
    text-align: left;
    font-weight: 600;
    color: rgba(255, 107, 53, 0.7);
  }

  .section-separator .section-title-cell {
    background: color-mix(in srgb, var(--md-sys-color-surface-variant) 92%, rgba(255, 107, 53, 0.08));
    min-width: 200px;
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
  }

  .task-name-cell {
    padding: 0.75rem;
    border-bottom: 2px solid rgba(255, 107, 53, 0.15);
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
    border-bottom: 2px solid rgba(255, 107, 53, 0.15);
  }

  .task-toggle-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
  }

  .task-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0.25rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 4px;
    background: var(--md-sys-color-surface);
    cursor: pointer;
    transition: all 0.2s ease;
    min-width: 32px;
    min-height: 32px;
  }

  .task-toggle:hover {
    background: var(--md-sys-color-surface-container-highest);
    transform: translateY(-1px);
  }

  .task-toggle.completed {
    background: var(--md-sys-color-primary);
    border-color: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    box-shadow: 0 2px 8px rgba(var(--md-sys-color-primary), 0.3);
  }

  .task-toggle.completed:hover {
    background: var(--md-sys-color-primary);
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(var(--md-sys-color-primary), 0.4);
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

  .gate-toggle:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .gate-toggle:disabled .gate-button {
    background: var(--md-sys-color-surface-variant);
    border-color: var(--md-sys-color-outline);
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
    background: #e5e7eb;
    transition: background-color 0.2s ease;
  }

  .rested-segment.filled {
    background: #10b981;
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

  .roster-task-cell {
    text-align: center;
    border-bottom: 2px solid rgba(255, 107, 53, 0.15);
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

  .roster-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .roster-toggle:hover {
    background: var(--md-sys-color-surface-container-highest);
    transform: translateY(-1px);
  }

  .roster-toggle.completed {
    background: var(--md-sys-color-primary);
    border-color: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    box-shadow: 0 2px 8px rgba(var(--md-sys-color-primary), 0.3);
  }

  .roster-toggle.completed:hover {
    background: var(--md-sys-color-primary);
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(var(--md-sys-color-primary), 0.4);
  }

  .roster-toggle.completed .checkbox {
    color: var(--md-sys-color-on-primary);
  }

  .roster-toggle:disabled {
    cursor: default;
    opacity: 0.55;
    transform: none;
  }

  .roster-toggle:disabled:hover {
    background: var(--md-sys-color-surface);
    transform: none;
  }

  .event-toggle {
    min-width: 8.5rem;
    padding-inline: 0.75rem;
    justify-content: center;
  }

  .event-label {
    font-weight: 700;
    color: inherit;
  }

  .roster-label {
    font-size: 0.875rem;
    font-weight: 500;
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
    background: var(--md-sys-color-surface-container-highest);
  }

  .gate-toggle.completed {
    background: var(--md-sys-color-surface-container);
  }

  .gate-button {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--md-sys-color-surface);
    border: 2px solid var(--md-sys-color-outline);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
  }

  .gate-toggle:hover .gate-button {
    border-color: var(--md-sys-color-primary);
    transform: scale(1.1);
  }

  .gate-toggle.completed .gate-button {
    background: var(--md-sys-color-primary);
    border-color: var(--md-sys-color-primary);
    box-shadow: 0 1px 4px rgba(var(--md-sys-color-primary), 0.2);
  }

  .gate-number {
    font-size: 0.5625rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
  }

  .gate-toggle.completed .gate-number {
    color: var(--md-sys-color-on-primary);
  }

  .empty-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  .error-state button {
    margin-top: 1rem;
    padding: 0.5rem 1rem;
    background: #ff6b35;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
</style>
