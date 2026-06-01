import { invoke } from '@tauri-apps/api/core';
import type {
  RaidConfigEntry,
  RaidGateCompletionRequest,
  RaidGateCompletionResponse,
  RosterEventProgress,
  TodoMatrixResponse
} from '$lib/components/todo/types';

export function loadTodoMatrixForRoster(rosterId: string): Promise<TodoMatrixResponse> {
  return invoke<TodoMatrixResponse>('get_todo_matrix', { rosterId });
}

export function loadRaidConfigsForRoster(rosterId: string): Promise<RaidConfigEntry[]> {
  return invoke<RaidConfigEntry[]>('get_raid_configs_for_roster', { rosterId });
}

export function loadTodoRosterEventProgress(rosterId: string, taskId: string): Promise<RosterEventProgress> {
  return invoke<RosterEventProgress>('get_roster_event_progress', { rosterId, taskId });
}

export function loadRaidGateCompletionsBulk(requests: RaidGateCompletionRequest[]): Promise<RaidGateCompletionResponse[]> {
  return invoke<RaidGateCompletionResponse[]>('get_raid_gate_completions_bulk', { requests });
}

export function updateTodoTaskStatus(characterId: number, taskId: string, completed: boolean) {
  return invoke('update_task_status', { characterId, taskId, completed });
}

export function updateTodoRosterTaskStatus(rosterId: string, taskId: string, completed: boolean) {
  return invoke('update_roster_task_status', { rosterId, taskId, completed });
}

export function updateTodoRaidGateStatus(
  characterId: number,
  raidId: string,
  gateId: string,
  contentId: string,
  completed: boolean
) {
  return invoke('update_raid_gate_status', {
    characterId,
    raidId,
    gateId,
    contentId,
    completed
  });
}

export function updateTodoRosterEventStatus(rosterId: string, taskId: string, completed: boolean) {
  return invoke('update_roster_event_status', { rosterId, taskId, completed });
}
