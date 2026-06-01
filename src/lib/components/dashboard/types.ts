import type { Character } from '$lib/store';

export interface CompletionStatusEntry {
  content_id: string;
  is_completed: number;
  details?: string | null;
  session_id?: string | null;
}

export interface RestedValueEntry {
  content_id: string;
  current_value: number;
}

export interface TrackingStatusEntry {
  content_id: string;
  is_tracked: number;
  lazy_daily?: number;
}

export interface RaidConfigEntry {
  content_id: string;
  gate: string;
  difficulty: string;
  take_gold: number;
  buy_box: number;
  reserved_for_static?: number;
  static_group_tag?: string;
}

export interface DashboardSnapshot {
  characters: Character[];
  rested_by_character: Record<string, RestedValueEntry[]>;
  completion_by_character: Record<string, CompletionStatusEntry[]>;
  tracking_by_character: Record<string, TrackingStatusEntry[]>;
  raid_configs_by_character: Record<string, RaidConfigEntry[]>;
}

export interface RosterEventProgress {
  task_id: string;
  completed_this_week: number;
  weekly_limit: number;
  completed_today: boolean;
  available: boolean;
}

export interface DashboardCharacterData {
  restedValues: RestedValueEntry[];
  completionStatus: CompletionStatusEntry[];
  raidConfigs: RaidConfigEntry[];
  trackingStatus: TrackingStatusEntry[];
}

export type OpenStatusKind = 'empty' | 'idle' | 'done' | 'open';
export type ArgeosStatusKind = 'empty' | 'done' | 'today' | 'open';
