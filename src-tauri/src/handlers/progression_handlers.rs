use serde::Deserialize;
use tauri::State;

use crate::database::repositories::progression_repository::{
    CharacterEngravingInput, CharacterEquipmentInput, CharacterGemInput, CharacterProgressionSnapshot,
    ProgressionGoalInput,
};
use crate::database::repositories::ProgressionRepository;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveScrapedProgressionPayload {
    pub character_id: i64,
    pub engravings: Vec<CharacterEngravingInput>,
    pub equipment: Vec<CharacterEquipmentInput>,
    pub gems: Vec<CharacterGemInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertProgressionGoalRequest {
    pub character_id: i64,
    pub goal: ProgressionGoalInput,
}

#[tauri::command]
pub fn get_character_progression_snapshot(
    character_id: i64,
    progression_repo: State<'_, ProgressionRepository>,
) -> Result<CharacterProgressionSnapshot, String> {
    crate::validation::validate_character_id(character_id)?;
    progression_repo
        .get_snapshot(character_id)
        .map_err(|e| format!("Failed to load progression data: {}", e))
}

/// Bulk-replace engravings, equipment, and gems (one transaction). Intended for scraper or full sync.
#[tauri::command]
pub fn save_scraped_character_progression(
    payload: SaveScrapedProgressionPayload,
    progression_repo: State<'_, ProgressionRepository>,
) -> Result<(), String> {
    crate::validation::validate_character_id(payload.character_id)?;
    progression_repo
        .replace_scraped_progression(
            payload.character_id,
            &payload.engravings,
            &payload.equipment,
            &payload.gems,
        )
        .map_err(|e| format!("Failed to save progression data: {}", e))
}

#[tauri::command]
pub fn upsert_progression_goal(
    request: UpsertProgressionGoalRequest,
    progression_repo: State<'_, ProgressionRepository>,
) -> Result<i64, String> {
    crate::validation::validate_character_id(request.character_id)?;
    progression_repo
        .upsert_goal(request.character_id, &request.goal)
        .map_err(|e| format!("Failed to save goal: {}", e))
}

#[tauri::command]
pub fn delete_progression_goal(goal_id: i64, progression_repo: State<'_, ProgressionRepository>) -> Result<bool, String> {
    if goal_id <= 0 {
        return Err("Invalid goal id".to_string());
    }
    progression_repo
        .delete_goal(goal_id)
        .map_err(|e| format!("Failed to delete goal: {}", e))
}
