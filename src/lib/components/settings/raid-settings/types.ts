import type { Raid, RaidGate } from '$lib/data/raids';

export type { Raid, RaidGate };

export interface RaidGroup {
  content_id: string;
  raid_name: string;
  difficulties: Map<string, Raid>;
  gates: Map<string, Map<string, RaidGate>>;
}

export interface RaidGateConfig {
  gate: string;
  difficulty?: string;
  take_gold?: boolean;
  buy_box?: boolean;
  reserved_for_static?: boolean;
}

export interface RaidConfig {
  content_id: string;
  gates: RaidGateConfig[];
  take_gold: boolean;
  buy_box: boolean;
  reserved_for_static: boolean;
}

export interface CharacterRaidConfig {
  char_id: number;
  char_name: string;
  item_level: number;
  combat_power: number;
  class_id: string;
  earns_gold: boolean;
  raid_configs: RaidConfig[];
  tracked_raid_ids: string[];
  available_difficulties: string[];
  master_difficulty: string;
  is_locked: boolean;
  gold_values: {
    totalGold: number;
    tradableGold: number;
    boundGold: number;
    boxPrice: number;
  };
}

export interface RaidMatrixData extends RaidGroup {
  characters: CharacterRaidConfig[];
  is_expanded: boolean;
  unique_key?: string;
}

export type RaidBulkToggleType = 'take_gold' | 'reserved_for_static';
