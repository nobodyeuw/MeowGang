import { invoke } from '@tauri-apps/api/core';

export interface RosterEventProgress {
  task_id: string;
  completed_this_week: number;
  weekly_limit: number;
  completed_today: boolean;
  available: boolean;
}

// Tauri command boundary for Settings > Tracking.
export function loadTrackingConfigMatrix(rosterId: string): Promise<any> {
  return invoke<any>('get_tracking_config_matrix', {
    rosterId,
    tasks: [],
    raids: []
  });
}

export function updateTrackingConfigCommand(
  characterId: number,
  taskId: string,
  tracked: boolean,
  currentValue: number | null = null
): Promise<void> {
  return invoke('update_tracking_config', { characterId, taskId, tracked, currentValue });
}

export function updateLazyDailyConfigCommand(characterId: number, taskId: string, lazyDaily: boolean): Promise<void> {
  return invoke('update_lazy_daily_config', { characterId, taskId, lazyDaily });
}

export function saveRestedValueCommand(characterId: number, taskId: string, restedValue: number): Promise<void> {
  return invoke('save_rested_value', { characterId, taskId, restedValue });
}

export function loadRosterEventProgressCommand(rosterId: string, taskId: string): Promise<RosterEventProgress> {
  return invoke<RosterEventProgress>('get_roster_event_progress', { rosterId, taskId });
}

export function updateRosterEventWeeklyCountCommand(
  rosterId: string,
  taskId: string,
  completedCount: number
): Promise<void> {
  return invoke('update_roster_event_weekly_count', { rosterId, taskId, completedCount });
}
