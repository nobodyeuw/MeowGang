import type {
  GameClass as FrontendGameClass,
  GameTask as FrontendGameTask,
  Raid as FrontendRaid
} from '$lib/data';
import type { GameClass, GameTask } from '$lib/types/store';

export function buildStoreGameClasses(classes: Record<string, FrontendGameClass>): GameClass[] {
  return Object.values(classes).map((cls) => ({
    id: cls.id,
    display_name: cls.displayName,
    icon_id: cls.iconId
  }));
}

export function buildStoreGameTasks(tasks: Record<string, FrontendGameTask>): GameTask[] {
  return Object.values(tasks).map((task) => ({
    id: task.id,
    name: task.name,
    category: task.category,
    reset_schedule: task.reset_schedule,
    logic_type: task.logic_type,
    max_rest_value: task.max_rest_value
  }));
}

export function buildSyncTaskPayload(tasks: Record<string, FrontendGameTask>) {
  return Object.fromEntries(
    Object.entries(tasks).map(([key, task]) => [
      key,
      {
        id: task.id,
        name: task.name,
        category: task.category,
        reset_schedule: task.reset_schedule,
        logic_type: task.logic_type,
        max_rest_value: task.max_rest_value
      }
    ])
  );
}

export function buildSyncRaidPayload(raids: FrontendRaid[]) {
  return raids.map((raid) => ({
    id: raid.id,
    name: raid.name,
    difficulty: raid.difficulty,
    gates: raid.gates.map((gate) => ({
      gate: gate.gate,
      minIlvl: gate.minIlvl,
      tradableGold: gate.tradableGold,
      boundGold: gate.boundGold,
      boxPrice: gate.boxPrice
    }))
  }));
}
