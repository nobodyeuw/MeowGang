use tauri::State;
use serde::{Deserialize, Serialize};
use crate::database::repositories::CharacterRepository;
use crate::models::{CharacterSettings, DashboardCharacter};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSettingsRequest {
    pub character_id: i64,
    pub settings: CharacterSettings,
}

#[tauri::command]
pub async fn update_character_settings(
    request: CharacterSettingsRequest,
    character_repo: State<'_, CharacterRepository>
) -> Result<(), String> {
    crate::validation::validate_character_id(request.character_id)?;
    character_repo.update_character_settings(request.character_id, &request.settings)
        .map_err(|e| format!("Failed to update character settings: {}", e))
}

#[tauri::command]
pub async fn update_character_earns_gold(
    character_id: i64,
    earns_gold: bool,
    character_repo: State<'_, CharacterRepository>
) -> Result<(), String> {
    crate::validation::validate_character_id(character_id)?;
    character_repo.update_character_earns_gold(character_id, earns_gold)
        .map_err(|e| format!("Failed to update character earns gold: {}", e))
}

#[tauri::command]
pub async fn get_character_details(
    character_id: i64,
    character_repo: State<'_, CharacterRepository>
) -> Result<Option<crate::roster::Character>, String> {
    crate::validation::validate_character_id(character_id)?;
    character_repo.get_character_by_id(character_id)
        .map_err(|e| format!("Failed to get character details: {}", e))
}

#[tauri::command]
pub async fn get_dashboard_characters(
    character_repo: State<'_, CharacterRepository>
) -> Result<Vec<DashboardCharacter>, String> {
    character_repo.get_dashboard_characters()
        .map_err(|e| format!("Failed to get dashboard characters: {}", e))
}

#[tauri::command]
pub async fn get_character_rested_values(
    characterId: i64,
    character_repo: State<'_, CharacterRepository>
) -> Result<Vec<crate::database::repositories::character_repository::RestedValue>, String> {
    crate::validation::validate_character_id(characterId)?;
    character_repo.get_character_rested_values(characterId)
        .map_err(|e| format!("Failed to get character rested values: {}", e))
}

#[tauri::command]
pub async fn get_character_completion_status(
    characterId: i64,
    character_repo: State<'_, CharacterRepository>
) -> Result<Vec<crate::database::repositories::character_repository::CompletionStatus>, String> {
    crate::validation::validate_character_id(characterId)?;
    character_repo.get_character_completion_status(characterId)
        .map_err(|e| format!("Failed to get character completion status: {}", e))
}

#[tauri::command]
pub async fn get_character_raid_configs(
    characterId: i64,
    character_repo: State<'_, CharacterRepository>
) -> Result<Vec<crate::database::repositories::character_repository::CharacterRaidConfig>, String> {
    crate::validation::validate_character_id(characterId)?;
    character_repo.get_character_raid_configs(characterId)
        .map_err(|e| format!("Failed to get character raid configs: {}", e))
}

#[tauri::command]
pub async fn get_character_tracking_status(
    characterId: i64,
    character_repo: State<'_, CharacterRepository>
) -> Result<Vec<crate::database::repositories::character_repository::TrackingStatus>, String> {
    crate::validation::validate_character_id(characterId)?;
    character_repo.get_character_tracking_status(characterId)
        .map_err(|e| format!("Failed to get character tracking status: {}", e))
}


#[tauri::command]
pub async fn test_character_query(
    character_id: i64,
    character_repo: State<'_, CharacterRepository>
) -> Result<String, String> {
    let character = character_repo.get_character_by_id(character_id)
        .map_err(|e| format!("Failed to query character: {}", e))?;
    
    match character {
        Some(char) => Ok(format!("Found character: {} (ILvl: {:.1})", char.char_name, char.item_level)),
        None => Ok("Character not found".to_string())
    }
}
