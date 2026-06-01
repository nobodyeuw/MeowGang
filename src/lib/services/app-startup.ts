import { invoke } from '@tauri-apps/api/core';
import { GAME_TASKS, RAIDS } from '$lib/data';

export async function updateRestedValuesNow() {
  await invoke('update_rested_values_now');
}

export async function ensureCharacterDataComplete() {
  await invoke('ensure_character_data_complete', {
    data: {
      tasks: GAME_TASKS,
      raids: RAIDS
    }
  });
}

export async function updateResetTimestamps() {
  await invoke('update_reset_timestamps');
}

export async function syncEncountersToCompletions() {
  await invoke('sync_encounters_to_completions');
}

export function getNextDailyResetTime(): Promise<string> {
  return invoke<string>('get_next_daily_reset_time');
}
