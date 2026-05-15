use tauri::State;
use crate::database::repositories::TodoRepository;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

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
}

#[tauri::command]
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
pub async fn get_raid_gate_completions_bulk(
    todo_repo: tauri::State<'_, Arc<TodoRepository>>,
    requests: Vec<RaidGateCompletionRequest>,
) -> Result<Vec<RaidGateCompletionResponse>, String> {
    let mut responses = Vec::with_capacity(requests.len());

    for request in requests {
        let completed = todo_repo
            .get_raid_gate_completed(
                request.character_id,
                &request.raid_id,
                &request.gate_id,
                &request.difficulty,
            )
            .map_err(|e| e.to_string())?
            .unwrap_or(false);

        responses.push(RaidGateCompletionResponse {
            character_id: request.character_id,
            raid_id: request.raid_id,
            gate_id: request.gate_id,
            completed,
        });
    }

    Ok(responses)
}
