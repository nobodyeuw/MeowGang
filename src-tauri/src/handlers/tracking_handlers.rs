use crate::database::repositories::{CharacterRepository, TrackingRepository};
use crate::models::TaskStatusStruct as TaskStatus;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMatrixItem {
    pub content_id: String,
    pub content_name: String,
    pub category: String,
    pub reset_schedule: String,
    pub logic_type: String,
    pub max_rest_value: Option<i64>,
    pub character_states: Vec<CharacterTaskState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterTaskState {
    pub char_id: i64,
    pub tracked: bool,
    pub _current_value: Option<i64>,
}

#[tauri::command]
/// Loads backend-owned state for Settings > Tracking.
pub async fn get_tracking_config_matrix(
    roster_id: &str,
    tasks: Vec<serde_json::Value>,
    raids: Vec<serde_json::Value>,
    task_repo: State<'_, TrackingRepository>,
    character_repo: State<'_, CharacterRepository>,
) -> Result<crate::models::TodoConfigMatrix, String> {
    let _ = (&tasks, &raids, &character_repo);
    task_repo
        .get_tracking_config_matrix(roster_id)
        .map_err(|e| format!("Failed to get tracking config matrix: {}", e))
}

#[tauri::command]
/// Updates one tracking visibility toggle in `conf_tracking`.
pub async fn update_tracking_config(
    character_id: i64,
    task_id: String,
    tracked: bool,
    _current_value: Option<i64>,
    task_repo: State<'_, TrackingRepository>,
) -> Result<(), String> {
    task_repo
        .update_tracking_config(character_id, &task_id, tracked)
        .map_err(|e| format!("Failed to update tracking config: {}", e))
}

#[tauri::command]
/// Updates the lazy-daily preference for a tracked character task.
pub async fn update_lazy_daily_config(
    character_id: i64,
    task_id: String,
    lazy_daily: bool,
    task_repo: State<'_, TrackingRepository>,
) -> Result<(), String> {
    task_repo
        .update_lazy_daily_config(character_id, &task_id, lazy_daily)
        .map_err(|e| format!("Failed to update lazy daily config: {}", e))
}

#[tauri::command]
/// Legacy bulk tracking save command retained for older UI paths.
pub async fn save_tracking_config(
    character_id: i64,
    task_updates: Vec<TaskStatus>,
    task_repo: State<'_, TrackingRepository>,
) -> Result<(), String> {
    task_repo
        .batch_update_task_status(character_id, task_updates)
        .map_err(|e| format!("Failed to save tracking config: {}", e))
}

#[tauri::command]
/// Saves a manually edited rested value.
pub async fn save_rested_value(
    character_id: i64,
    task_id: String,
    rested_value: i64,
    task_repo: State<'_, TrackingRepository>,
) -> Result<(), String> {
    task_repo
        .save_rested_value(character_id, &task_id, rested_value)
        .map_err(|e| format!("Failed to save rested value: {}", e))
}

#[tauri::command]
/// Legacy To Do tracking toggle command retained for compatibility.
pub async fn set_todo_tracked(
    character_id: i64,
    task_id: String,
    tracked: bool,
    task_repo: State<'_, TrackingRepository>,
) -> Result<(), String> {
    task_repo
        .set_todo_tracked(character_id, &task_id, tracked)
        .map_err(|e| format!("Failed to set todo tracked: {}", e))
}
