use tauri::State;
use crate::database::repositories::TodoRepository;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use rusqlite::params;
use rusqlite::OptionalExtension;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    pub sync_id: String,
    pub table_name: Option<String>,
    pub record_id: Option<String>,
    pub operation: Option<String>,
    pub timestamp: i64,
    pub sync_status: String,
    pub source: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosterScrapeResult {
    pub roster_id: String,
    pub scraped: bool,
    pub characters_updated: usize,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

/// Get the last scrape timestamp for a specific roster
pub fn get_last_roster_scrape_time(todo_repo: &TodoRepository, roster_id: &str) -> Result<Option<i64>, String> {
    let conn = todo_repo.pool.get().map_err(|e| format!("Database connection failed: {}", e))?;
    
    let sync_id = format!("scraper_roster_{}", roster_id);
    
    let timestamp: Option<i64> = conn.query_row(
        "SELECT timestamp FROM sync_metadata 
         WHERE sync_id = ?1 AND sync_status = 'completed' 
         ORDER BY timestamp DESC LIMIT 1",
        [sync_id],
        |row| row.get(0)
    ).optional().map_err(|e| format!("Failed to query sync metadata: {}", e))?;
    
    Ok(timestamp)
}

/// Update roster scrape metadata
pub fn update_roster_scrape_metadata(
    todo_repo: &TodoRepository, 
    roster_id: &str, 
    status: &str,
    data: Option<&str>
) -> Result<(), String> {
    let conn = todo_repo.pool.get().map_err(|e| format!("Database connection failed: {}", e))?;
    
    let sync_id = format!("scraper_roster_{}", roster_id);
    let timestamp = chrono::Utc::now().timestamp_millis();
    
    conn.execute(
        "INSERT OR REPLACE INTO sync_metadata 
         (sync_id, table_name, record_id, operation, timestamp, sync_status, source, data) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            sync_id,
            "conf_character",
            roster_id,
            "scrape",
            timestamp,
            status,
            "scraper",
            data.unwrap_or("")
        ]
    ).map_err(|e| format!("Failed to update sync metadata: {}", e))?;
    
    Ok(())
}

/// Check if a roster should be scraped (more than 24 hours since last scrape)
pub fn should_scrape_roster_daily(todo_repo: &TodoRepository, roster_id: &str) -> Result<bool, String> {
    let last_scrape = get_last_roster_scrape_time(todo_repo, roster_id)?;
    
    let should_scrape = match last_scrape {
        Some(timestamp) => {
            let twenty_four_hours_ago = chrono::Utc::now().timestamp_millis() - (24 * 60 * 60 * 1000);
            timestamp < twenty_four_hours_ago
        },
        None => true, // Never scraped before
    };
    
    Ok(should_scrape)
}

/// Get all roster IDs that need daily scraping
pub fn get_rosters_needing_daily_scrape(todo_repo: &TodoRepository) -> Result<Vec<String>, String> {
    let conn = todo_repo.pool.get().map_err(|e| format!("Database connection failed: {}", e))?;
    
    // Get all unique roster IDs
    let mut stmt = conn.prepare(
        "SELECT DISTINCT roster_id FROM conf_character WHERE roster_id IS NOT NULL"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let roster_iter = stmt.query_map([], |row| {
        Ok(row.get::<_, String>(0)?)
    }).map_err(|e| format!("Failed to query rosters: {}", e))?;
    
    let mut rosters_needing_scrape = Vec::new();
    
    for roster_result in roster_iter {
        let roster_id = roster_result.map_err(|e| format!("Failed to parse roster: {}", e))?;
        
        if should_scrape_roster_daily(todo_repo, &roster_id)? {
            rosters_needing_scrape.push(roster_id);
        }
    }
    
    Ok(rosters_needing_scrape)
}

/// Perform daily scraping for all rosters that need it
#[tauri::command]
pub async fn perform_daily_roster_scraping(
    todo_repo: State<'_, Arc<TodoRepository>>,
    roster_repo: State<'_, crate::database::repositories::RosterRepository>,
    scraper: State<'_, crate::roster::HumanizedScraper>,
) -> Result<Vec<RosterScrapeResult>, String> {
    let start_time = std::time::Instant::now();
    
    // Get rosters that need scraping
    let rosters_needing_scrape = get_rosters_needing_daily_scrape(&*todo_repo)?;
    
    let mut results = Vec::new();
    
    for roster_id in rosters_needing_scrape {
        let roster_start_time = std::time::Instant::now();
        let mut characters_updated = 0;
        let mut errors = Vec::new();
        let mut scraped = false;
        
        // Mark as started
        if let Err(e) = update_roster_scrape_metadata(&*todo_repo, &roster_id, "started", None) {
            errors.push(format!("Failed to mark scrape as started: {}", e));
        }
        
        // Perform scraping
        match scrape_roster_for_updates(&*todo_repo, &roster_id).await {
            Ok(updated_count) => {
                characters_updated = updated_count;
                scraped = true;
                
                // Mark as completed
                if let Err(e) = update_roster_scrape_metadata(
                    &*todo_repo, 
                    &roster_id, 
                    "completed", 
                    Some(&format!("Updated {} characters", updated_count))
                ) {
                    errors.push(format!("Failed to mark scrape as completed: {}", e));
                }
            },
            Err(e) => {
                errors.push(format!("Scraping failed: {}", e));
                
                // Mark as failed
                if let Err(e2) = update_roster_scrape_metadata(&*todo_repo, &roster_id, "failed", Some(&e)) {
                    errors.push(format!("Failed to mark scrape as failed: {}", e2));
                }
            }
        }
        
        results.push(RosterScrapeResult {
            roster_id: roster_id.clone(),
            scraped,
            characters_updated,
            errors,
            duration_ms: roster_start_time.elapsed().as_millis() as u64,
        });
    }
    
    Ok(results)
}

/// Scrape a specific roster and update character item_level and combat_power
async fn scrape_roster_for_updates(
    todo_repo: &TodoRepository,
    roster_id: &str
) -> Result<usize, String> {
    // Get characters for this roster
    let characters = get_characters_for_roster(todo_repo, roster_id)?;
    
    let mut updated_count = 0;
    
    // Create a new scraper for this roster
    let mut scraper = crate::roster::HumanizedScraper::new(
        characters.first().map(|c| c.char_name.clone()).unwrap_or_else(|| "Unknown".to_string()),
        roster_id.to_string()
    );
    
    // Scrape the entire roster at once
    match scraper.scrape_roster().await {
        Ok(scraper_result) => {
            // Update each character from the scraped data
            for scraped_char in scraper_result.mapped_for_models.roster.characters {
                if update_character_stats(
                    todo_repo, 
                    &scraped_char.char_name, 
                    scraped_char.item_level, 
                    scraped_char.combat_power
                )? {
                    updated_count += 1;
                }
            }
        },
        Err(e) => {
            return Err(format!("Scraping failed for roster {}: {}", roster_id, e));
        }
    }
    
    Ok(updated_count)
}

/// Get all characters for a specific roster
pub fn get_characters_for_roster(todo_repo: &TodoRepository, roster_id: &str) -> Result<Vec<CharacterInfo>, String> {
    let conn = todo_repo.pool.get().map_err(|e| format!("Database connection failed: {}", e))?;
    
    let mut stmt = conn.prepare(
        "SELECT char_name, char_id FROM conf_character WHERE roster_id = ?1"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let char_iter = stmt.query_map([roster_id], |row| {
        Ok(CharacterInfo {
            char_name: row.get(0)?,
            char_id: row.get(1)?,
        })
    }).map_err(|e| format!("Failed to query characters: {}", e))?;
    
    let mut characters = Vec::new();
    for char_result in char_iter {
        characters.push(char_result.map_err(|e| format!("Failed to parse character: {}", e))?);
    }
    
    Ok(characters)
}

#[derive(Debug)]
pub struct CharacterInfo {
    pub char_name: String,
    pub char_id: i64,
}


/// Update character stats in database
pub fn update_character_stats(
    todo_repo: &TodoRepository,
    char_name: &str,
    item_level: f64,
    combat_power: f64
) -> Result<bool, String> {
    let conn = todo_repo.pool.get().map_err(|e| format!("Database connection failed: {}", e))?;
    
    // Format item_level to 2 decimal places
    let formatted_item_level = (item_level * 100.0).round() / 100.0;
    
    let rows_affected = conn.execute(
        "UPDATE conf_character 
         SET item_level = ?1, combat_power = ?2 
         WHERE char_name = ?3",
        params![formatted_item_level, combat_power, char_name]
    ).map_err(|e| format!("Failed to update character: {}", e))?;
    
    Ok(rows_affected > 0)
}

/// Get scrape history for a roster
#[tauri::command]
pub async fn get_roster_scrape_history(
    todo_repo: State<'_, Arc<TodoRepository>>,
    roster_id: String,
    limit: Option<i64>
) -> Result<Vec<SyncMetadata>, String> {
    let conn = todo_repo.pool.get().map_err(|e| format!("Database connection failed: {}", e))?;
    
    let sync_id_pattern = format!("scraper_roster_{}", roster_id);
    let limit_val = limit.unwrap_or(10);
    
    let mut stmt = conn.prepare(
        "SELECT sync_id, table_name, record_id, operation, timestamp, sync_status, source, data 
         FROM sync_metadata 
         WHERE sync_id LIKE ?1 
         ORDER BY timestamp DESC 
         LIMIT ?2"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let metadata_iter = stmt.query_map(
        params![format!("{}%", sync_id_pattern.to_string()), limit_val], 
        |row: &rusqlite::Row| {
            Ok(SyncMetadata {
                sync_id: row.get(0)?,
                table_name: row.get(1)?,
                record_id: row.get(2)?,
                operation: row.get(3)?,
                timestamp: row.get(4)?,
                sync_status: row.get(5)?,
                source: row.get(6)?,
                data: row.get(7)?,
            })
        }
    ).map_err(|e| format!("Failed to query sync metadata: {}", e))?;
    
    let mut metadata_list = Vec::new();
    for metadata_result in metadata_iter {
        metadata_list.push(metadata_result.map_err(|e| format!("Failed to parse metadata: {}", e))?);
    }
    
    Ok(metadata_list)
}
