use serde::{Deserialize, Serialize};

// Shared data models used across the application

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardCharacter {
    pub char_id: i64,
    pub char_name: String,
    pub class_id: String,
    pub class_display_name: Option<String>,
    pub item_level: f64,
    pub combat_power: f64,
    pub roster_name: String,
    pub last_active: Option<String>,
    pub earns_gold: bool,
    pub display_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterMatrixInfo {
    pub char_id: i64,
    pub char_name: String,
    pub item_level: f64,
    pub combat_power: f64,
    pub class_id: String,
    pub display_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterUpdate {
    pub name: String,
    pub gear_level: f64,
    pub combat_power: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidCompletion {
    pub character_name: String,
    pub raid_name: String,
    pub difficulty: String,
    pub clear_time: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameClass {
    pub id: String,
    pub display_name: String,
    pub icon_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub content_id: String,
    pub content_name: String,
    pub category: String,
    pub reset_schedule: String,
    pub logic_type: String,
    pub max_rest_value: Option<i64>,
    pub min_ilvl: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMatrix {
    pub character_id: i64,
    pub character_name: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub reset_time: String,
    pub completed_at: Option<String>,
    pub character_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Daily,
    Weekly,
    Una,
    Raid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Completed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub dashboard: DashboardSettings,
    pub tasks: TaskSettings,
    pub sync: SyncSettings,
    pub general: GeneralSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSettings {
    pub columns: Vec<String>,
    pub show_inactive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSettings {
    pub auto_mark_raids: bool,
    pub show_completed: bool,
    pub roster_selection: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSettings {
    pub encounters_path: Option<String>,
    pub auto_import: bool,
    pub update_interval: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub theme: Theme,
    pub language: Language,
    pub start_minimized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Language {
    De,
    En,
}

// Additional models needed for repositories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosterResources {
    pub bound_gold: i64,
    pub trade_gold: i64,
    pub total_gold: i64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidGateCompletion {
    pub gate: String,
    pub completed: bool,
    pub completion_time: Option<i64>,
    pub session_id: Option<String>,
}

// Handler-specific structs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Raid {
    pub id: String,
    pub name: String,
    pub difficulty: String,
    pub min_ilvl: i64,
    pub max_players: i32,
    pub gates: Vec<RaidGate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidGate {
    pub gate: String,
    pub name: String,
    pub min_ilvl: i64,
    pub tradable_gold: Option<i32>,
    pub bound_gold: Option<i32>,
    pub box_price: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRaidState {
    pub char_id: i64,
    pub content_id: String,
    pub tracked: bool,
    pub current_value: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidMatrixItem {
    pub raid_id: String,
    pub raid_name: String,
    pub min_ilvl: i64,
    pub character_states: Vec<CharacterRaidState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidGateStatus {
    pub character_id: i64,
    pub raid_id: String,
    pub gate: String,
    pub cleared: bool,
    pub clear_time: Option<i64>,
    pub session_id: Option<String>,
    pub take_gold: Option<bool>,
    pub buy_box: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidSettingsEntry {
    pub raid_id: String,
    pub raid_name: String,
    pub difficulty: String,
    pub take_gold: bool,
    pub buy_box: bool,
    pub gate_count: i64,
    pub completion_status: i64,
    pub max_difficulty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidSettings {
    pub auto_track_gold: bool,
    pub auto_buy_boxes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameTask {
    pub id: String,
    pub name: String,
    pub category: String,
    pub reset_schedule: String,
    pub logic_type: String,
    pub max_rest_value: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatusStruct {
    pub character_id: i64,
    pub task_id: String,
    pub tracked: bool,
    pub completed: bool,
    pub current_value: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMatrixItem {
    pub content_id: String,
    pub content_name: String,
    pub category: String,
    pub reset_schedule: String,
    pub logic_type: String,
    pub max_rest_value: Option<i64>,
    pub character_states: Vec<CharacterRaidState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoConfigMatrix {
    pub characters: Vec<CharacterMatrixInfo>,
    pub daily_tasks: Vec<TaskMatrixItem>,
    pub roster_tasks: Vec<TaskMatrixItem>,
    pub weekly_tasks: Vec<TaskMatrixItem>,
    pub raids: Vec<RaidMatrixItem>,
    pub todo_entries: Option<Vec<(i64, String, bool)>>,
    pub rested_entries: Option<Vec<(i64, String, i64)>>,
    pub character_states: Option<Vec<CharacterRaidState>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSettingsStruct {
    pub auto_track_daily: bool,
    pub auto_track_weekly: bool,
    pub auto_track_roster: bool,
    pub auto_mark_raids: bool,
    pub show_completed: bool,
    pub roster_selection: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSettings {
    pub earns_gold: Option<bool>,
    pub hide_from_dashboard: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Roster {
    pub id: String,
    pub roster_name: String,
    pub last_updated: Option<i64>,
}
