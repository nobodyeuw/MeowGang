import type { Character } from '$lib/store';

export interface FriendOption {
  id: string;
  name: string;
  testRosterId?: string;
}

export interface DiscordAuthResult {
  approved: boolean;
  user_id?: string;
  username?: string;
}

export interface PlannedMember {
  id: string;
  name: string;
  type: 'self' | 'friend';
  testRosterId?: string;
  color?: string;
}

export interface RaidLane {
  id: string;
  name: string;
  minIlvl: number;
  maxIlvl: number;
  raidIds: string[];
  difficulties: Array<{
    difficulty: string;
    minIlvl: number;
  }>;
  assignments: string[];
}

export interface CompletionStatusEntry {
  content_id: string;
  is_completed: number;
  details?: string | null;
  session_id?: string | null;
  timestamp?: number;
}

export interface DashboardSnapshot {
  completion_by_character: Record<string, CompletionStatusEntry[]>;
  raid_configs_by_character: Record<string, CharacterRaidConfig[]>;
}

export interface CharacterRaidConfig {
  content_id: string;
  gate?: string;
  difficulty: string;
}

export interface EncounterPreview {
  id: number;
  current_boss: string;
  local_player: string;
  difficulty: string;
  fight_start: number;
  cleared: boolean;
  players: string[];
}

export interface CharacterRosterGroup {
  rosterId: string;
  rosterName: string;
  characters: Character[];
}
