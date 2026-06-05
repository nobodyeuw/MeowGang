use crate::database::repositories::TodoRepository;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Deserialize)]
pub struct RaidGateCompletionRequest {
    pub character_id: i64,
    pub raid_id: String,
    pub gate_id: String,
    pub difficulty: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RaidGateCompletionResponse {
    pub character_id: i64,
    pub raid_id: String,
    pub gate_id: String,
    pub completed: bool,
    pub actual_difficulty: Option<String>,
}

#[tauri::command]
/// Reads weekly/daily progress for a roster event task.
pub async fn get_roster_event_progress(
    todo_repo: State<'_, Arc<TodoRepository>>,
    roster_id: String,
    task_id: String,
) -> Result<crate::database::repositories::todo_repository::RosterEventProgress, String> {
    crate::validation::validate_non_empty(&roster_id, "roster_id")?;
    crate::validation::validate_non_empty(&task_id, "task_id")?;
    todo_repo
        .get_roster_event_progress(&roster_id, &task_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
/// Toggles today's completion state for a roster event task.
pub async fn update_roster_event_status(
    todo_repo: State<'_, Arc<TodoRepository>>,
    roster_id: String,
    task_id: String,
    completed: bool,
) -> Result<(), String> {
    crate::validation::validate_non_empty(&roster_id, "roster_id")?;
    crate::validation::validate_non_empty(&task_id, "task_id")?;
    todo_repo
        .set_roster_event_completed(&roster_id, &task_id, completed)
        .map_err(|e| e.to_string())
}

#[tauri::command]
/// Sets the manual weekly completion count for a roster event task.
pub async fn update_roster_event_weekly_count(
    todo_repo: State<'_, Arc<TodoRepository>>,
    roster_id: String,
    task_id: String,
    completed_count: i64,
) -> Result<(), String> {
    crate::validation::validate_non_empty(&roster_id, "roster_id")?;
    crate::validation::validate_non_empty(&task_id, "task_id")?;
    if !(0..=3).contains(&completed_count) {
        return Err("Roster event completion count must be 0, 1, 2, or 3".to_string());
    }
    todo_repo
        .set_roster_event_weekly_count(&roster_id, &task_id, completed_count)
        .map_err(|e| e.to_string())
}

#[tauri::command]
/// Loads backend-owned To Do state for one roster.
pub async fn get_todo_matrix(
    todo_repo: State<'_, Arc<TodoRepository>>,
    roster_id: String,
) -> Result<crate::database::repositories::todo_repository::TodoMatrixResponse, String> {
    crate::validation::validate_non_empty(&roster_id, "roster_id")?;
    match todo_repo.get_todo_matrix(&roster_id) {
        Ok(matrix) => Ok(matrix),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
/// Toggles a character task completion state.
pub async fn update_task_status(
    todo_repo: State<'_, Arc<TodoRepository>>,
    character_id: i64,
    task_id: String,
    completed: bool,
) -> Result<(), String> {
    crate::validation::validate_character_id(character_id)?;
    crate::validation::validate_non_empty(&task_id, "task_id")?;
    match todo_repo.set_task_completed(character_id, &task_id, completed) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
/// Toggles a roster-wide task completion state.
pub async fn update_roster_task_status(
    todo_repo: State<'_, Arc<TodoRepository>>,
    roster_id: String,
    task_id: String,
    completed: bool,
) -> Result<(), String> {
    crate::validation::validate_non_empty(&roster_id, "roster_id")?;
    crate::validation::validate_non_empty(&task_id, "task_id")?;
    match todo_repo.set_roster_task_completed(&roster_id, &task_id, completed) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
/// Toggles a raid gate clear state.
pub async fn update_raid_gate_status(
    todo_repo: State<'_, Arc<TodoRepository>>,
    character_id: i64,
    raid_id: String,
    gate_id: String,
    content_id: String,
    completed: bool,
) -> Result<(), String> {
    crate::validation::validate_character_id(character_id)?;
    crate::validation::validate_content_id(&content_id)?;
    // Get difficulty from conf_raid table
    let difficulty = match todo_repo.get_raid_gate_difficulty(character_id, &raid_id, &gate_id) {
        Ok(Some(diff)) => diff,
        Ok(None) => "Normal".to_string(), // Default difficulty
        Err(e) => return Err(e.to_string()),
    };

    // Use a custom method that includes difficulty and session_id
    match todo_repo.set_raid_gate_completed(character_id, &content_id, completed, &difficulty, &gate_id) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
/// Reads one raid gate clear state.
pub async fn get_raid_gate_completed(
    todo_repo: tauri::State<'_, Arc<TodoRepository>>,
    character_id: i64,
    raid_id: String,
    gate_id: String,
    difficulty: String,
) -> Result<Option<bool>, String> {
    match todo_repo.get_raid_gate_completed(character_id, &raid_id, &gate_id, &difficulty) {
        Ok(completed) => Ok(completed),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
/// Reads many raid gate clear states in one command call.
pub async fn get_raid_gate_completions_bulk(
    todo_repo: tauri::State<'_, Arc<TodoRepository>>,
    requests: Vec<RaidGateCompletionRequest>,
) -> Result<Vec<RaidGateCompletionResponse>, String> {
    let mut responses = Vec::with_capacity(requests.len());

    for request in requests {
        let details = todo_repo
            .get_raid_gate_completion_details(request.character_id, &request.raid_id, &request.gate_id)
            .map_err(|e| e.to_string())?;

        responses.push(RaidGateCompletionResponse {
            character_id: request.character_id,
            raid_id: request.raid_id,
            gate_id: request.gate_id,
            completed: details.completed,
            actual_difficulty: details.actual_difficulty,
        });
    }

    Ok(responses)
}
