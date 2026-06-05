import { GAME_TASKS } from '$lib/data/tasks';
import { RAIDS, type Raid } from '$lib/data/raids';
import type {
  RaidConfigEntry,
  RaidGateDifficultyMap,
  RaidGateCompletionRequest,
  TodoMatrixResponse,
  TodoRaid,
  TodoTask
} from '$lib/components/todo/types';

export const VIRTUAL_RAT_ROSTER_ID = '__todo_virtual_rat__';

export function isRosterEventTask(taskId: string): boolean {
  return taskId === 'event_argeos_winter';
}

export function buildRosterTaskStates(baseMatrix: TodoMatrixResponse): Record<string, boolean> {
  const states: Record<string, boolean> = {};

  Object.values(GAME_TASKS).forEach((task: any) => {
    if (task.category !== 'roster') return;

    let taskCompleted = false;
    if (baseMatrix.character_states && baseMatrix.characters.length > 0) {
      const firstCharId = baseMatrix.characters[0].id;
      const key = `${firstCharId}_${task.id}`;
      const state = baseMatrix.character_states[key];
      taskCompleted = state ? state.completed : false;
    }

    states[task.id] = taskCompleted;
  });

  return states;
}

export function buildRaidConfigMap(raidConfigs: RaidConfigEntry[]): RaidGateDifficultyMap {
  const raidConfigMap: RaidGateDifficultyMap = new Map();

  raidConfigs.forEach((config) => {
    if (!raidConfigMap.has(config.content_id)) {
      raidConfigMap.set(config.content_id, new Map());
    }
    const characterMap = raidConfigMap.get(config.content_id)!;
    if (!characterMap.has(config.char_id)) {
      characterMap.set(config.char_id, new Map());
    }
    const gateMap = characterMap.get(config.char_id)!;
    if (config.gate) {
      gateMap.set(config.gate, config.difficulty);
    }
    if (!gateMap.has('__default')) {
      gateMap.set('__default', config.difficulty);
    }
  });

  return raidConfigMap;
}

export function getRaidGateDifficulty(
  raidConfigMap: RaidGateDifficultyMap,
  raidId: string,
  characterId: number,
  gateId?: string
): string {
  const gateMap = raidConfigMap.get(raidId)?.get(characterId);
  if (!gateMap) return 'Normal';
  return (gateId ? gateMap.get(gateId) : undefined) || gateMap.get('__default') || 'Normal';
}

export function filterTodoMatrixCharacters(matrix: TodoMatrixResponse, characterIds: Set<number>): TodoMatrixResponse {
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

export function buildTodoTasks(baseMatrix: TodoMatrixResponse, isRatView: boolean) {
  const allTasks = Object.values(GAME_TASKS).map((task): TodoTask => {
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
        ilvl_too_low: false
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

  const isAnyCharTracked = (task: TodoTask) => task.character_states.some((state) => state.tracked);

  return {
    dailyTasks: allTasks.filter((task) => task.reset_schedule === 'daily' && task.category === 'character' && isAnyCharTracked(task)),
    rosterTasks: isRatView ? [] : allTasks.filter((task) => task.category === 'roster' && isAnyCharTracked(task)),
    weeklyTasks: allTasks.filter((task) => task.reset_schedule === 'weekly' && task.category === 'character' && isAnyCharTracked(task))
  };
}

export function getTrackedTodoRaidCandidates(baseMatrix: TodoMatrixResponse): Raid[] {
  const raidMap = new Map<string, Raid>();
  RAIDS.forEach((raid) => {
    if (!raidMap.has(raid.id)) {
      raidMap.set(raid.id, raid);
    }
  });

  return Array.from(raidMap.values())
    .filter((raid) => {
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
}

export function buildRaidGateCompletionRequests(
  raids: Raid[],
  baseMatrix: TodoMatrixResponse,
  raidConfigMap: RaidGateDifficultyMap
): RaidGateCompletionRequest[] {
  const requests: RaidGateCompletionRequest[] = [];

  raids.forEach((raid) => {
    baseMatrix.characters.forEach((char: any) => {
      raid.gates.forEach((gate: any) => {
        requests.push({
          character_id: char.id,
          raid_id: raid.id,
          gate_id: gate.gate,
          difficulty: getRaidGateDifficulty(raidConfigMap, raid.id, char.id, gate.gate)
        });
      });
    });
  });

  return requests;
}

export function buildTrackedTodoRaids(
  raids: Raid[],
  baseMatrix: TodoMatrixResponse,
  raidConfigMap: RaidGateDifficultyMap,
  gateCompletionMap: Map<string, { completed: boolean; actualDifficulty?: string | null }>
): TodoRaid[] {
  return raids.map((raid) => {
    const characterStates = baseMatrix.characters.map((char: any) => {
      const key = `${char.id}_${raid.id}`;
      const backendState = baseMatrix.character_states?.[key];
      const difficulty = getRaidGateDifficulty(raidConfigMap, raid.id, char.id);

      const gateStates = raid.gates.map((gate: any) => {
        const completionKey = `${char.id}_${raid.id}_${gate.gate}`;
        return gateCompletionMap.get(completionKey)?.completed || false;
      });
      const gateActualDifficulties = raid.gates.map((gate: any) => {
        const completionKey = `${char.id}_${raid.id}_${gate.gate}`;
        return gateCompletionMap.get(completionKey)?.actualDifficulty ?? null;
      });

      return {
        tracked: backendState?.tracked || false,
        gate_states: gateStates,
        gate_actual_difficulties: gateActualDifficulties,
        ilvl_too_low: false,
        difficulty
      };
    });

    return {
      id: raid.id,
      raid_name: raid.name,
      difficulty: raid.difficulty,
      gates: raid.gates.map((gate) => ({ gate: gate.gate, name: gate.gate, min_ilvl: gate.minIlvl })),
      character_states: characterStates
    };
  });
}
