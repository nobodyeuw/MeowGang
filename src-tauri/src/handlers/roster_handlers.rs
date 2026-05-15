use serde::{Deserialize, Serialize};
use tauri::State;
use crate::database::repositories::RosterRepository;
use crate::roster::{HumanizedScraper, scraper::Character as RosterCharacter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Roster {
    pub id: String,
    pub name: String,
    pub last_updated: Option<String>,
}

#[tauri::command]
pub async fn get_rosters(
    roster_repo: State<'_, RosterRepository>
) -> Result<Vec<crate::models::Roster>, String> {
    
    roster_repo.get_all_rosters()
        .map_err(|e| format!("Failed to get rosters: {}", e))
}

#[tauri::command]
pub async fn get_characters(
    rosterId: Option<String>,
    roster_repo: State<'_, RosterRepository>
) -> Result<Vec<crate::roster::Character>, String> {
    
    
    let characters = match rosterId {
        Some(rosterId) => {
            roster_repo.get_characters_by_roster(&rosterId)
                .map_err(|e| format!("Failed to get characters for roster {}: {}", rosterId, e))?
        }
        None => {
            let rosters = roster_repo.get_all_rosters()
                .map_err(|e| format!("Failed to get rosters: {}", e))?;
            
            let mut all_characters = Vec::new();
            for roster in rosters {
                let roster_characters = roster_repo.get_characters_by_roster(&roster.id)
                    .map_err(|e| format!("Failed to get characters for roster {}: {}", roster.id, e))?;
                all_characters.extend(roster_characters);
            }
            all_characters
        }
    };
    
    crate::log_debug!("Returning {} characters", characters.len());
    Ok(characters)
}

#[tauri::command]
pub async fn scrape_roster(
    rosterName: String,
    roster_repo: State<'_, RosterRepository>,
    scraper: State<'_, HumanizedScraper>
) -> Result<Vec<crate::roster::Character>, String> {
    
    crate::validation::validate_roster_name(&rosterName)?;
    
    // Scrape roster from LostArk Bible
    let mut scraper_instance = HumanizedScraper::new(rosterName.clone(), rosterName.clone());
    let scraper_data = scraper_instance.scrape_roster()
        .await
        .map_err(|e| format!("Failed to scrape roster: {}", e))?;
    
    // Save to database
    roster_repo.save_roster_from_scraper(&scraper_data.scraper_data)
        .map_err(|e| format!("Failed to save roster: {}", e))?;
    
    // Return characters
    let characters: Vec<RosterCharacter> = scraper_data.scraper_data.characters.into_iter().map(|char| RosterCharacter {
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
        class_display_name: char.class_display_name,
    }).collect();
    
    Ok(characters)
}

#[tauri::command]
pub async fn check_and_sync_roster_if_needed(
    rosterName: String,
    roster_repo: State<'_, RosterRepository>,
    scraper: State<'_, HumanizedScraper>
) -> Result<bool, String> {
    crate::log_debug!("check_and_sync_roster_if_needed called for: {}", rosterName);
    
    // Check if update is needed
    let scraper_ref = scraper.inner();
    if !scraper_ref.can_make_request() {
        return Ok(false); // No update needed
    }
    
    // Check if roster exists and needs update
    let should_update = roster_repo.should_update_roster(&rosterName)
        .map_err(|e| format!("Failed to check roster update status: {}", e))?;
    
    if !should_update {
        return Ok(false);
    }
    
    // Perform sync
    let mut scraper_instance = HumanizedScraper::new(rosterName.clone(), rosterName.clone());
    match scraper_instance.scrape_roster().await {
        Ok(scraper_data) => {
            roster_repo.save_roster_from_scraper(&scraper_data.scraper_data)
                .map_err(|e| format!("Failed to save roster: {}", e))?;
            Ok(true)
        }
        Err(e) => Err(format!("Failed to sync roster: {}", e))
    }
}

#[tauri::command]
pub async fn update_character_order(
    characters: Vec<serde_json::Value>,
    roster_repo: State<'_, RosterRepository>
) -> Result<(), String> {
    for character in characters {
        if let (Some(char_id), Some(display_order)) = (
            character.get("char_id").and_then(|v| v.as_i64()),
            character.get("display_order").and_then(|v| v.as_i64())
        ) {
            roster_repo.update_character_order(char_id, &display_order.to_string())
                .map_err(|e| format!("Failed to update character order for {}: {}", char_id, e))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn update_character_roster_name(
    character_id: i64,
    new_roster_name: String,
    roster_repo: State<'_, RosterRepository>
) -> Result<(), String> {
    roster_repo.update_character_roster_name(character_id, &new_roster_name)
        .map_err(|e| format!("Failed to update character roster name: {}", e))
}

#[tauri::command]
pub async fn sync_roster_data(
    rosterId: String,
    tasks: std::collections::HashMap<String, crate::database::data_manager::GameTask>,
    raids: Vec<crate::database::data_manager::Raid>,
    db_manager: tauri::State<'_, crate::database::DatabaseManager>
) -> Result<String, String> {
    
    // Use ensure_character_data_complete to initialize ALL characters in the roster
    crate::database::data_manager::DataManager::ensure_character_data_complete(
        &db_manager.pool,
        tasks,
        raids,
    ).map_err(|e| format!("Failed to sync roster data: {}", e))?;
    
    Ok("Roster data synced successfully".to_string())
}

#[tauri::command]
pub async fn sync_roster_if_needed(
    rosterId: String,
    tasks: std::collections::HashMap<String, crate::database::data_manager::GameTask>,
    raids: Vec<crate::database::data_manager::Raid>,
    db_manager: tauri::State<'_, crate::database::DatabaseManager>
) -> Result<bool, String> {
    crate::log_debug!("sync_roster_if_needed called for roster: {}", rosterId);
    
    // Check if roster exists and needs update
    let rosters = db_manager.pool.get().unwrap()
        .prepare("SELECT id FROM conf_character WHERE roster_id = ?1 LIMIT 1")
        .unwrap()
        .query_map([&rosterId], |row| {
            Ok(row.get::<_, String>(0)?)
        })
        .map_err(|e| format!("Failed to check roster: {}", e))?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|e| format!("Failed to collect roster check: {}", e))?;
    
    if rosters.is_empty() {
        return Ok(false);
    }
    
    
    // Actually sync the data
    crate::database::data_manager::DataManager::ensure_character_data_complete(
        &db_manager.pool,
        tasks,
        raids,
    ).map_err(|e| format!("Failed to sync roster data: {}", e))?;
    
    Ok(true)
}

#[tauri::command]
pub async fn delete_roster(
    rosterId: String,
    roster_repo: State<'_, RosterRepository>
) -> Result<(), String> {
    crate::validation::validate_non_empty(&rosterId, "roster_id")?;
    match roster_repo.delete_roster(&rosterId) {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            Err(format!("Failed to delete roster: {}", e))
        }
    }
}
