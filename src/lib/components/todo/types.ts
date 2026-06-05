export interface TodoCharacter {
  id: number;
  name: string;
  class: string;
  ilvl?: number;
  combat_power?: number;
  earns_gold?: boolean;
  display_order?: string;
}

export interface TodoTask {
  id: string;
  name: string;
  category: string;
  reset_schedule: string;
  logic_type: string;
  max_rest_value?: number;
  min_ilvl?: number;
  character_states: CharacterTaskState[];
}

export interface CharacterTaskState {
  tracked: boolean;
  completed: boolean;
  rested_value?: number;
  ilvl_too_low: boolean;
}

export interface TodoRaid {
  id: string;
  raid_name: string;
  difficulty: string;
  gates: Array<{ gate: string; name: string; min_ilvl?: number }>;
  character_states: TodoRaidCharacterState[];
}

export interface TodoRaidCharacterState {
  tracked: boolean;
  gate_states: boolean[];
  gate_actual_difficulties?: Array<string | null>;
  ilvl_too_low: boolean;
  difficulty?: string;
}

export interface TodoMatrixResponse {
  characters: TodoCharacter[];
  daily_tasks: TodoTask[];
  roster_tasks: TodoTask[];
  weekly_tasks: TodoTask[];
  raids: TodoRaid[];
  character_states?: Record<string, any>;
  rested_entries?: Array<[number, string, number]>;
  todo_entries?: Array<[number, string, boolean]>;
}

export interface RaidGateCompletionRequest {
  character_id: number;
  raid_id: string;
  gate_id: string;
  difficulty: string;
}

export interface RaidGateCompletionResponse {
  character_id: number;
  raid_id: string;
  gate_id: string;
  completed: boolean;
  actual_difficulty?: string | null;
}

export interface RaidConfigEntry {
  char_id: number;
  content_id: string;
  gate?: string;
  difficulty: string;
}

export type RaidGateDifficultyMap = Map<string, Map<number, Map<string, string>>>;

export interface RosterEventProgress {
  task_id: string;
  completed_this_week: number;
  weekly_limit: number;
  completed_today: boolean;
  available: boolean;
}
