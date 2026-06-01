import { invoke } from '@tauri-apps/api/core';

export function loadCharacterProgressionSnapshot<TSnapshot>(characterId: number): Promise<TSnapshot> {
  return invoke<TSnapshot>('get_character_progression_snapshot', { characterId });
}

export function scrapeCharacterProgressionDetails(request: {
  characterName: string;
  characterId: number;
  rosterName: string;
}): Promise<string> {
  return invoke<string>('scrape_character_details', { request });
}
