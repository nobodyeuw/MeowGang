import { getGameClassIconId } from '$lib/data/classes';
import { GAME_TASKS } from '$lib/data/index';
import { RAIDS } from '$lib/data/raids';

const COLLAPSE_UNTRACKED_ROWS_STORAGE_KEY = 'trackingSettings.collapseUntrackedRows';

export function isRosterEventTask(taskId: string): boolean {
  return false;
}

export function loadCollapseUntrackedRows(): boolean {
  try {
    return localStorage.getItem(COLLAPSE_UNTRACKED_ROWS_STORAGE_KEY) === '1';
  } catch {
    return false;
  }
}

export function saveCollapseUntrackedRows(value: boolean) {
  try {
    localStorage.setItem(COLLAPSE_UNTRACKED_ROWS_STORAGE_KEY, value ? '1' : '0');
  } catch {
    // Ignore storage failures; the in-memory view state still updates.
  }
}

export function getVisibleTrackingRows(rows: any[], collapseRows: boolean): any[] {
  return collapseRows ? rows.filter(isTrackingRowEnabled) : rows;
}

export function isTrackingRowEnabled(row: any): boolean {
  return (row?.character_states || []).some((state: any) => state.tracked === true);
}

export function getClassIcon(classId: string): string {
  return getGameClassIconId(classId);
}

export function supportsLazyDaily(taskId: string): boolean {
  return taskId === 'chaos' || taskId === 'guardian';
}

export function buildTrackingMatrixData(baseMatrix: any) {
  const tasksArray = Object.values(GAME_TASKS).map(task => {
    const characterStates = baseMatrix.characters.map((char: any) => {
      const backendState = baseMatrix.character_states?.find((state: any) =>
        state.char_id === char.char_id && state.content_id === task.id
      );

      return {
        char_id: char.char_id,
        tracked: backendState?.tracked || false,
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

  const raidsMap = new Map<string, any>();
  [...RAIDS].forEach(raid => {
    const baseName = raid.name;
    const existingSortOrder = raidsMap.get(baseName)?.sortOrder || 0;
    if (!raidsMap.has(baseName) || raid.sortOrder < existingSortOrder) {
      raidsMap.set(baseName, raid);
    }
  });

  const lowIlvlTrackingClears: Array<{ characterId: number; taskId: string }> = [];

  const raidsArray = Array.from(raidsMap.values()).sort((a, b) => {
    return a.sortOrder - b.sortOrder; // Sort by explicit sortOrder
  }).map((raid: any) => {
    const characterStates = baseMatrix.characters.map((char: any) => {
      const backendState = baseMatrix.character_states?.find((state: any) =>
        state.char_id === char.char_id && state.content_id === raid.id
      );

      const eligible = raid.gates[0]?.minIlvl === undefined || raid.gates[0].minIlvl <= char.item_level;
      const tracked = eligible ? (backendState?.tracked || false) : false;

      if (!eligible && backendState?.tracked) {
        lowIlvlTrackingClears.push({
          characterId: char.char_id,
          taskId: raid.id
        });
      }

      return {
        char_id: char.char_id,
        tracked,
        current_value: null,
        lazy_daily: false
      };
    });

    return {
      raid_id: raid.id,
      raid_name: raid.name,
      min_ilvl: raid.gates[0]?.minIlvl || 0,
      character_states: characterStates
    };
  });

  return {
    matrixData: {
      characters: baseMatrix.characters,
      daily_tasks: tasksArray.filter((task: any) =>
        task.reset_schedule === 'daily' &&
        (task.content_id === 'chaos' || task.content_id === 'guardian')
      ),
      weekly_tasks: tasksArray.filter((task: any) => task.reset_schedule === 'weekly' && task.category === 'character'),
      roster_tasks: tasksArray.filter((task: any) => task.category === 'roster'),
      raids: raidsArray
    },
    lowIlvlTrackingClears
  };
}

