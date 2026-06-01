import { invoke } from '@tauri-apps/api/core';
import type { Character } from '$lib/store';
import type { SyncMetadata } from '$lib/components/settings/roster-settings/helpers';

export interface CharacterOrderUpdate {
  char_id: number;
  display_order: number;
}

export interface RosterOrderUpdate {
  roster_id: string;
  display_order: number;
}

// Tauri command boundary for Settings > Roster.
export function loadAllCharactersCommand(): Promise<Character[]> {
  return invoke<Character[]>('get_characters', {});
}

export function updateRosterNameCommand(rosterId: string, newRosterName: string): Promise<void> {
  return invoke('update_roster_name', { rosterId, newRosterName });
}

export function deleteRosterCommand(rosterId: string): Promise<void> {
  return invoke('delete_roster', { rosterId });
}

export function loadRosterScrapeHistoryCommand(rosterId: string, limit = 5): Promise<SyncMetadata[]> {
  return invoke<SyncMetadata[]>('get_roster_scrape_history', { rosterId, limit });
}

export function updateRosterOrderCommand(rosters: RosterOrderUpdate[]): Promise<void> {
  return invoke('update_roster_order', { rosters });
}

export function updateCharacterOrderCommand(characters: CharacterOrderUpdate[]): Promise<void> {
  return invoke('update_character_order', { characters });
}
