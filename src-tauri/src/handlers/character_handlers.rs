#![allow(non_snake_case)]

use crate::database::repositories::CharacterRepository;
use crate::database::repositories::ProgressionRepository;
use crate::models::{CharacterSettings, DashboardCharacter};
use crate::roster::DetailedCharacterScraper;
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSettingsRequest {
    pub character_id: i64,
    pub settings: CharacterSettings,
}

/// Updates optional per-character settings from Settings > Roster and MeowConnect toggles.
#[tauri::command]
pub async fn update_character_settings(
    request: CharacterSettingsRequest,
    character_repo: State<'_, CharacterRepository>,
) -> Result<(), String> {
    crate::validation::validate_character_id(request.character_id)?;
    character_repo
        .update_character_settings(request.character_id, &request.settings)
        .map_err(|e| format!("Failed to update character settings: {}", e))
}

/// Updates whether one character counts toward weekly gold.
#[tauri::command]
pub async fn update_character_earns_gold(
    character_id: i64,
    earns_gold: bool,
    character_repo: State<'_, CharacterRepository>,
) -> Result<(), String> {
    crate::validation::validate_character_id(character_id)?;
    character_repo
        .update_character_earns_gold(character_id, earns_gold)
        .map_err(|e| format!("Failed to update character earns gold: {}", e))
}

/// Loads one character by id for detail/debug views.
#[tauri::command]
pub async fn get_character_details(
    character_id: i64,
    character_repo: State<'_, CharacterRepository>,
) -> Result<Option<crate::roster::Character>, String> {
    crate::validation::validate_character_id(character_id)?;
    character_repo
        .get_character_by_id(character_id)
        .map_err(|e| format!("Failed to get character details: {}", e))
}

/// Loads the compact dashboard character list used by legacy dashboard paths.
#[tauri::command]
pub async fn get_dashboard_characters(
    character_repo: State<'_, CharacterRepository>,
) -> Result<Vec<DashboardCharacter>, String> {
    character_repo
        .get_dashboard_characters()
        .map_err(|e| format!("Failed to get dashboard characters: {}", e))
}

/// Loads rested values for one character.
#[tauri::command]
pub async fn get_character_rested_values(
    characterId: i64,
    character_repo: State<'_, CharacterRepository>,
) -> Result<Vec<crate::database::repositories::character_repository::RestedValue>, String> {
    crate::validation::validate_character_id(characterId)?;
    character_repo
        .get_character_rested_values(characterId)
        .map_err(|e| format!("Failed to get character rested values: {}", e))
}

/// Loads task and raid completion state for one character.
#[tauri::command]
pub async fn get_character_completion_status(
    characterId: i64,
    character_repo: State<'_, CharacterRepository>,
) -> Result<Vec<crate::database::repositories::character_repository::CompletionStatus>, String> {
    crate::validation::validate_character_id(characterId)?;
    character_repo
        .get_character_completion_status(characterId)
        .map_err(|e| format!("Failed to get character completion status: {}", e))
}

/// Loads Settings > Raids configuration for one character.
#[tauri::command]
pub async fn get_character_raid_configs(
    characterId: i64,
    character_repo: State<'_, CharacterRepository>,
) -> Result<Vec<crate::database::repositories::character_repository::CharacterRaidConfig>, String> {
    crate::validation::validate_character_id(characterId)?;
    character_repo
        .get_character_raid_configs(characterId)
        .map_err(|e| format!("Failed to get character raid configs: {}", e))
}

/// Loads Settings > Tracking visibility for one character.
#[tauri::command]
pub async fn get_character_tracking_status(
    characterId: i64,
    character_repo: State<'_, CharacterRepository>,
) -> Result<Vec<crate::database::repositories::character_repository::TrackingStatus>, String> {
    crate::validation::validate_character_id(characterId)?;
    character_repo
        .get_character_tracking_status(characterId)
        .map_err(|e| format!("Failed to get character tracking status: {}", e))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrapeCharacterDetailsRequest {
    pub character_name: String,
    pub character_id: i64,
    pub roster_name: String,
}

/// Scrapes detailed progression data for the hidden progression planner.
///
/// This path writes engravings, equipment, and gems only; it does not mutate
/// roster membership or raid/tracking configuration.
#[tauri::command]
pub async fn scrape_character_details(
    request: ScrapeCharacterDetailsRequest,
    progression_repo: State<'_, ProgressionRepository>,
) -> Result<String, String> {
    crate::validation::validate_character_id(request.character_id)?;

    let mut scraper = DetailedCharacterScraper::new(request.roster_name);

    let detail_data = scraper
        .scrape_character_details(request.character_name.clone())
        .await
        .map_err(|e| format!("Failed to scrape character details: {}", e))?;

    let engravings: Vec<crate::database::repositories::progression_repository::CharacterEngravingInput> = detail_data
        .engravings
        .into_iter()
        .map(
            |e| crate::database::repositories::progression_repository::CharacterEngravingInput {
                engraving_name: e.engraving_name,
                books_read: e.books_read as i64,
                max_books: e.max_books as i64,
                stone_bonus: e.stone_bonus as i64,
                is_manual_entry: false,
            },
        )
        .collect();

    let equipment: Vec<crate::database::repositories::progression_repository::CharacterEquipmentInput> = detail_data
        .equipment
        .into_iter()
        .map(
            |e| crate::database::repositories::progression_repository::CharacterEquipmentInput {
                slot: e.slot,
                enhancement_level: e.enhancement_level.map(|v| v as i64),
                tier: e.tier,
                quality: e.quality.map(|v| v as i64),
                item_level: e.item_level,
                effects_json: e.effects_json,
                is_manual_entry: false,
            },
        )
        .collect();

    let gems: Vec<crate::database::repositories::progression_repository::CharacterGemInput> = detail_data
        .gems
        .into_iter()
        .map(
            |g| crate::database::repositories::progression_repository::CharacterGemInput {
                slot_index: g.slot_index,
                gem_name: g.gem_name,
                gem_item_id: g.gem_item_id,
                skill_id: g.skill_id,
                skill_name: g.skill_name,
                skill_icon: g.skill_icon,
                gem_type: g.gem_type,
                gem_level: g.gem_level,
                effect_value: g.effect_value,
                is_bound: g.is_bound,
                is_manual_entry: false,
            },
        )
        .collect();

    progression_repo
        .replace_scraped_progression(request.character_id, &engravings, &equipment, &gems)
        .map_err(|e| format!("Failed to save character details: {}", e))?;

    Ok(format!(
        "Successfully scraped and saved details for character: {} ({} engravings, {} equipment, {} gems)",
        detail_data.character_name,
        engravings.len(),
        equipment.len(),
        gems.len()
    ))
}
