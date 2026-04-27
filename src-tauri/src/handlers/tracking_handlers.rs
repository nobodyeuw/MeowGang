use serde::{Deserialize, Serialize};
use tauri::State;
use crate::database::repositories::{TrackingRepository, CharacterRepository};
use crate::models::{TaskStatusStruct as TaskStatus};

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
pub async fn get_tracking_config_matrix(
    roster_id: &str,
    tasks: Vec<serde_json::Value>,
    raids: Vec<serde_json::Value>,
    task_repo: State<'_, TrackingRepository>,
    character_repo: State<'_, CharacterRepository>
) -> Result<crate::models::TodoConfigMatrix, String> {
    // Get characters for this roster
    task_repo.get_tracking_config_matrix(roster_id)
        .map_err(|e| format!("Failed to get tracking config matrix: {}", e))
}

#[tauri::command]
pub async fn update_tracking_config(
    character_id: i64,
    task_id: String,
    tracked: bool,
    _current_value: Option<i64>,
    task_repo: State<'_, TrackingRepository>
) -> Result<(), String> {
    task_repo.update_tracking_config(character_id, &task_id, tracked)
        .map_err(|e| format!("Failed to update tracking config: {}", e))
}

#[tauri::command]
pub async fn save_tracking_config(
    character_id: i64,
    task_updates: Vec<TaskStatus>,
    task_repo: State<'_, TrackingRepository>
) -> Result<(), String> {
    task_repo.batch_update_task_status(character_id, task_updates)
        .map_err(|e| format!("Failed to save tracking config: {}", e))
}

#[tauri::command]
pub async fn save_rested_value(
    character_id: i64,
    task_id: String,
    rested_value: i64,
    task_repo: State<'_, TrackingRepository>
) -> Result<(), String> {
    task_repo.save_rested_value(character_id, &task_id, rested_value)
        .map_err(|e| format!("Failed to save rested value: {}", e))
}

#[tauri::command]
pub async fn set_todo_tracked(
    character_id: i64,
    task_id: String,
    tracked: bool,
    task_repo: State<'_, TrackingRepository>
) -> Result<(), String> {
    task_repo.set_todo_tracked(character_id, &task_id, tracked)
        .map_err(|e| format!("Failed to set todo tracked: {}", e))
}
