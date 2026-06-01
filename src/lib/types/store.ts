export interface Roster {
  id: string;
  roster_name: string;
  roster_display_order?: number;
  last_updated?: string;
}

export interface CharacterSettings {
  earns_gold?: boolean;
  hide_from_dashboard?: boolean;
  meow_connect_enabled?: boolean;
  removed_from_roster?: boolean;
}

export interface Character {
  char_id: number;
  char_name: string;
  roster_id: string;
  roster_name: string;
  class_id: string;
  item_level: number;
  combat_power: number;
  display_order: number;
  earns_gold: boolean;
  hide_from_dashboard?: boolean;
  meow_connect_enabled?: boolean;
  removed_from_roster?: boolean;
  icon_id?: string;
  class_display_name?: string;
  last_active?: string;
}

export interface GameClass {
  id: string;
  display_name: string;
  icon_id: string;
}

export interface GameTask {
  id: string;
  name: string;
  category: string;
  reset_schedule: string;
  logic_type: string;
  max_rest_value?: number;
}

export interface Raid {
  id: string;
  name: string;
  difficulty: string;
  gates: RaidGate[];
}

export interface RaidGate {
  gate: string;
  min_ilvl: number;
  tradable_gold: number;
  bound_gold: number;
  box_price: number;
}

export interface TodoSettingsEntry {
  content_id: string;
  content_name: string;
  tracked: boolean;
  category: string;
  reset_schedule: string;
}

export interface RaidSettingsEntry {
  raid_id: string;
  raid_name: string;
  difficulty: string;
  take_gold: boolean;
  buy_box: boolean;
  gate_count: number;
  completion_status: number;
  max_difficulty: string;
}

export interface SystemSettings {
  loa_logs_path?: string;
  encountersDbPath?: string;
  lostArkExePath?: string;
  loaLogsExePath?: string;
  startWithWindows?: boolean;
  startWithLostArk?: boolean;
  startWithLoaLogs?: boolean;
  showSetupGuideButton?: boolean;
  showHaalsHourglassReminder?: boolean;
  next_daily_reset: string;
  next_weekly_reset: string;
  current_theme: string;
}

export interface BootstrapSnapshot {
  rosters: Roster[];
  characters: Character[];
  next_daily_reset: string;
}

export interface TodoMatrixResponse {
  characters: TodoCharacter[];
  daily_tasks: TodoTask[];
  roster_tasks: TodoTask[];
  weekly_tasks: TodoTask[];
  raids: TodoRaid[];
  character_states?: Record<string, CharacterTaskState>;
  rested_entries?: Array<[number, string, number]>;
  todo_entries?: Array<[number, string, boolean]>;
}

export interface TodoCharacter {
  id: number;
  name: string;
  class: string;
  ilvl: number;
  display_order: string;
}

export interface TodoTask {
  id: string;
  name: string;
  category: string;
  reset_schedule: string;
  logic_type: string;
  max_rest_value?: number;
  character_states: CharacterTaskState[];
}

export interface CharacterTaskState {
  tracked: boolean;
  completed: boolean;
  rested_value?: number;
  ilvl_too_low?: boolean;
}

export interface TodoRaid {
  id: string;
  raid_name: string;
  difficulty: string;
  gates: TodoRaidGate[];
  character_states: CharacterRaidState[];
}

export interface TodoRaidGate {
  gate: string;
  name: string;
}

export interface CharacterRaidState {
  tracked: boolean;
  gate_states: RaidGateState[];
  ilvl_too_low: boolean;
}

export interface RaidGateState {
  gate: string;
  cleared: boolean;
  clear_time?: string;
}
