#![allow(non_snake_case)]

use crate::database::repositories::RosterRepository;
use crate::roster::{scraper::Character as RosterCharacter, HumanizedScraper};
use tauri::State;

/// Returns all locally known rosters.
#[tauri::command]
pub async fn get_rosters(roster_repo: State<'_, RosterRepository>) -> Result<Vec<crate::models::Roster>, String> {
    roster_repo
        .get_all_rosters()
        .map_err(|e| format!("Failed to get rosters: {}", e))
}

/// Returns active characters for one roster, or all active characters when no roster is selected.
#[tauri::command]
pub async fn get_characters(
    rosterId: Option<String>,
    roster_repo: State<'_, RosterRepository>,
) -> Result<Vec<crate::roster::Character>, String> {
    let characters = match rosterId {
        Some(rosterId) => roster_repo
            .get_characters_by_roster(&rosterId)
            .map_err(|e| format!("Failed to get characters for roster {}: {}", rosterId, e))?,
        None => {
            let rosters = roster_repo
                .get_all_rosters()
                .map_err(|e| format!("Failed to get rosters: {}", e))?;

            let mut all_characters = Vec::new();
            for roster in rosters {
                let roster_characters = roster_repo
                    .get_characters_by_roster(&roster.id)
                    .map_err(|e| format!("Failed to get characters for roster {}: {}", roster.id, e))?;
                all_characters.extend(roster_characters);
            }
            all_characters
        }
    };

    crate::log_debug!("Returning {} characters", characters.len());
    Ok(characters)
}

/// Scrapes a roster from LostArk Bible and stores the resulting characters locally.
///
/// New characters are added to `conf_character`; existing character stats are refreshed while
/// local user settings remain preserved by the repository upsert.
#[tauri::command]
pub async fn scrape_roster(
    rosterName: String,
    roster_repo: State<'_, RosterRepository>,
) -> Result<Vec<crate::roster::Character>, String> {
    crate::validation::validate_roster_name(&rosterName)?;

    // Scrape roster from LostArk Bible
    let mut scraper_instance = HumanizedScraper::new(rosterName.clone(), rosterName.clone());
    let scraper_data = scraper_instance
        .scrape_roster()
        .await
        .map_err(|e| format!("Failed to scrape roster: {}", e))?;

    // Save to database
    roster_repo
        .save_roster_from_scraper(&scraper_data.scraper_data)
        .map_err(|e| format!("Failed to save roster: {}", e))?;

    // Return characters
    let characters: Vec<RosterCharacter> = scraper_data
        .scraper_data
        .characters
        .into_iter()
        .map(|char| RosterCharacter {
            char_id: char.char_id,
            char_name: char.char_name,
            roster_id: char.roster_id,
            roster_name: char.roster_name,
            class_id: char.class_id,
            item_level: char.item_level,
            combat_power: char.combat_power,
            display_order: char.display_order,
            earns_gold: char.earns_gold,
            hide_from_dashboard: false,
            meow_connect_enabled: char.meow_connect_enabled,
            removed_from_roster: false,
            class_display_name: char.class_display_name,
        })
        .collect();

    Ok(characters)
}

/// Persists drag-and-drop character order from Settings > Roster.
#[tauri::command]
pub async fn update_character_order(
    characters: Vec<serde_json::Value>,
    roster_repo: State<'_, RosterRepository>,
) -> Result<(), String> {
    for character in characters {
        if let (Some(char_id), Some(display_order)) = (
            character.get("char_id").and_then(|v| v.as_i64()),
            character.get("display_order").and_then(|v| v.as_i64()),
        ) {
            roster_repo
                .update_character_order(char_id, &display_order.to_string())
                .map_err(|e| format!("Failed to update character order for {}: {}", char_id, e))?;
        }
    }
    Ok(())
}

/// Persists drag-and-drop roster order from Settings > Roster.
#[tauri::command]
pub async fn update_roster_order(
    rosters: Vec<serde_json::Value>,
    roster_repo: State<'_, RosterRepository>,
) -> Result<(), String> {
    for roster in rosters {
        if let (Some(roster_id), Some(display_order)) = (
            roster.get("roster_id").and_then(|v| v.as_str()),
            roster.get("display_order").and_then(|v| v.as_i64()),
        ) {
            roster_repo
                .update_roster_order(roster_id, display_order)
                .map_err(|e| format!("Failed to update roster order for {}: {}", roster_id, e))?;
        }
    }
    Ok(())
}

/// Renames the roster display name on one character row.
#[tauri::command]
pub async fn update_character_roster_name(
    character_id: i64,
    new_roster_name: String,
    roster_repo: State<'_, RosterRepository>,
) -> Result<(), String> {
    crate::validation::validate_character_id(character_id)?;
    crate::validation::validate_roster_name(&new_roster_name)?;

    roster_repo
        .update_character_roster_name(character_id, &new_roster_name)
        .map_err(|e| format!("Failed to update character roster name: {}", e))
}

/// Renames the roster display name on every character in the roster.
#[tauri::command]
pub async fn update_roster_name(
    roster_id: String,
    new_roster_name: String,
    roster_repo: State<'_, RosterRepository>,
) -> Result<(), String> {
    crate::validation::validate_non_empty(&roster_id, "roster_id")?;
    crate::validation::validate_roster_name(&new_roster_name)?;

    roster_repo
        .update_roster_name(&roster_id, &new_roster_name)
        .map_err(|e| format!("Failed to update roster name: {}", e))
}

/// Backfills local task/raid rows for all characters using frontend game-data payloads.
#[tauri::command]
pub async fn sync_roster_data(
    rosterId: String,
    tasks: std::collections::HashMap<String, crate::database::data_manager::GameTask>,
    raids: Vec<crate::database::data_manager::Raid>,
    db_manager: tauri::State<'_, crate::database::DatabaseManager>,
) -> Result<String, String> {
    crate::validation::validate_non_empty(&rosterId, "roster_id")?;

    // `ensure_character_data_complete` currently scans all local characters so
    // static game-data additions are applied across every roster, not only the
    // roster that triggered the sync.
    crate::database::data_manager::DataManager::ensure_character_data_complete(&db_manager.pool, tasks, raids)
        .map_err(|e| format!("Failed to sync roster data: {}", e))?;

    Ok("Roster data synced successfully".to_string())
}

/// Backfills task/raid rows only when the requested roster exists locally.
#[tauri::command]
pub async fn sync_roster_if_needed(
    rosterId: String,
    tasks: std::collections::HashMap<String, crate::database::data_manager::GameTask>,
    raids: Vec<crate::database::data_manager::Raid>,
    db_manager: tauri::State<'_, crate::database::DatabaseManager>,
) -> Result<bool, String> {
    crate::log_debug!("sync_roster_if_needed called for roster: {}", rosterId);
    crate::validation::validate_non_empty(&rosterId, "roster_id")?;

    let conn = db_manager
        .pool
        .get()
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let roster_exists: bool = conn
        .query_row(
            "SELECT EXISTS (
                SELECT 1
                FROM conf_character
                WHERE roster_id = ?1
                  AND COALESCE(removed_from_roster, 0) = 0
            )",
            [&rosterId],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to check roster: {}", e))?;

    if !roster_exists {
        return Ok(false);
    }

    crate::database::data_manager::DataManager::ensure_character_data_complete(&db_manager.pool, tasks, raids)
        .map_err(|e| format!("Failed to sync roster data: {}", e))?;

    Ok(true)
}

/// Permanently deletes one local roster and its dependent local state.
#[tauri::command]
pub async fn delete_roster(rosterId: String, roster_repo: State<'_, RosterRepository>) -> Result<(), String> {
    crate::validation::validate_non_empty(&rosterId, "roster_id")?;
    match roster_repo.delete_roster(&rosterId) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to delete roster: {}", e)),
    }
}
