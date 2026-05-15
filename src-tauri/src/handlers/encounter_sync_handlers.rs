use anyhow::Result;
use tauri::{AppHandle, State, command, Emitter};
use std::sync::Arc;
use rusqlite::{params, OptionalExtension};
use serde_json::json;

use crate::database::repositories::todo_repository::TodoRepository;
use crate::sync::boss_mapping::BossMapper;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncResult {
    pub synced_count: usize,
    pub skipped_count: usize,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EncounterPreview {
    pub id: i64,
    pub current_boss: String,
    pub local_player: String,
    pub difficulty: String,
    pub fight_start: i64,
    pub cleared: bool,
}

pub fn sync_encounters_to_completions_internal(
    app: AppHandle,
    todo_repo: Arc<TodoRepository>,
    settings_manager: &crate::settings::SettingsManager,
) -> Result<SyncResult, String> {
    let start_time = std::time::Instant::now();
    let boss_mapper = BossMapper::new();
    
    // Emit sync start event
    app.emit("encounter-sync-start", json!({})).map_err(|e| e.to_string())?;
    
    let mut synced_count = 0;
    let mut skipped_count = 0;
    let mut errors = Vec::new();

    // Get encounters.db path from JSON settings
    let encounters_db_path = get_encounters_db_path_from_settings(settings_manager)?;
        
    // Get new cleared encounters from encounters.db
    let encounters = match get_cleared_encounters(&encounters_db_path, &todo_repo.pool) {
        Ok(encounters) => encounters,
        Err(e) => {
            errors.push(format!("Failed to read encounters.db: {}", e));
            return Ok(SyncResult {
                synced_count: 0,
                skipped_count: 0,
                errors,
                duration_ms: start_time.elapsed().as_millis() as u64,
            });
        }
    };

        
    // Process each encounter
    let total_encounters = encounters.len();
    for encounter in encounters {
        match process_encounter(&encounter, &boss_mapper, &todo_repo, settings_manager) {
            Ok(success) => {
                if success {
                    synced_count += 1;
                    
                    // Emit progress event
                    app.emit("encounter-sync-progress", json!({
                        "current": synced_count,
                        "total": total_encounters,
                        "encounter": encounter.current_boss,
                        "player": encounter.local_player
                    })).map_err(|e| e.to_string())?;
                } else {
                    skipped_count += 1;
                }
            }
            Err(e) => {
                errors.push(format!("Failed to process encounter {}: {}", encounter.current_boss, e));
            }
        }
    }

    let duration = start_time.elapsed().as_millis() as u64;
    
    // Emit sync complete event
    app.emit("encounter-sync-complete", json!({
        "synced_count": synced_count,
        "skipped_count": skipped_count,
        "errors": &errors,
        "duration_ms": duration
    })).map_err(|e| e.to_string())?;

    Ok(SyncResult {
        synced_count,
        skipped_count,
        errors,
        duration_ms: duration,
    })
}

#[command]
pub fn sync_encounters_to_completions(
    app: AppHandle,
    todo_repo: State<'_, Arc<TodoRepository>>,
    settings_manager: State<'_, crate::settings::SettingsManager>,
) -> Result<SyncResult, String> {
    sync_encounters_to_completions_internal(
        app,
        todo_repo.inner().clone(),
        settings_manager.inner(),
    )
}

#[command]
pub fn get_encounters_preview(
    todo_repo: State<'_, Arc<TodoRepository>>,
    settings_manager: State<'_, crate::settings::SettingsManager>,
    limit: Option<i32>,
) -> Result<Vec<EncounterPreview>, String> {
    let limit = limit.unwrap_or(50);
    
    // Get encounters.db path from JSON settings
    let encounters_db_path = get_encounters_db_path_from_settings(settings_manager.inner())?;
    
    match get_cleared_encounters(&encounters_db_path, &todo_repo.pool) {
        Ok(encounters) => Ok(encounters.into_iter().take(limit as usize).collect()),
        Err(e) => Err(e.to_string()),
    }
}

#[command]
pub fn test_boss_mapping(boss_name: String) -> Result<Option<EncounterMappingResult>, String> {
    let boss_mapper = BossMapper::new();
    
    if let Some(mapping) = boss_mapper.map_boss_to_encounter(&boss_name) {
        Ok(Some(EncounterMappingResult {
            content_id: mapping.content_id.clone(),
            gate: mapping.gate,
            boss_names: mapping.boss_names.clone(),
        }))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EncounterMappingResult {
    pub content_id: String,
    pub gate: u8,
    pub boss_names: Vec<String>,
}

pub fn get_encounters_db_path_from_settings(settings_manager: &crate::settings::SettingsManager) -> Result<String, String> {
    let settings = settings_manager.read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());
    
    settings.system.encounters_db_path
        .ok_or_else(|| "encounters_db_path not found in settings".to_string())
}

fn get_weekly_reset_from_app_state(pool: &r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>) -> Result<i64, String> {
    let conn = pool.get().map_err(|e| format!("Database connection failed: {}", e))?;
    
    let weekly_reset: i64 = conn.query_row(
        "SELECT last_weekly_reset FROM app_state LIMIT 1",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
        
    Ok(weekly_reset)
}

fn calculate_next_weekly_reset(last_reset: i64, _current_time: i64) -> i64 {
    let last_reset_dt = chrono::DateTime::from_timestamp(last_reset, 0)
        .unwrap_or_else(|| chrono::Utc::now());
    
    // Add 7 days to get next reset
    let next_reset = last_reset_dt + chrono::Duration::days(7);
    next_reset.timestamp()
}

fn get_cleared_encounters(encounters_db_path: &str, pool: &r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>) -> Result<Vec<EncounterPreview>, String> {
    let conn = rusqlite::Connection::open_with_flags(
        encounters_db_path, 
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
    ).map_err(|e| format!("Failed to open encounters.db in read-only mode: {}", e))?;
    
    // Get weekly reset timestamp from app_state table
    let weekly_reset_ts = get_weekly_reset_from_app_state(pool)?;
    crate::log_debug!("Filtering encounters since weekly_reset_ts: {} (milliseconds)", weekly_reset_ts);
    
    let mut stmt = conn.prepare(
        "SELECT id, current_boss, local_player, difficulty, fight_start, cleared 
         FROM encounter_preview 
         WHERE cleared = 1 AND fight_start >= ? 
         ORDER BY fight_start DESC"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let encounters_iter = stmt.query_map([weekly_reset_ts], |row| {
        Ok(EncounterPreview {
            id: row.get(0).unwrap_or(0),
            current_boss: row.get(1).unwrap_or_else(|_| "Unknown".to_string()),
            local_player: row.get(2).unwrap_or_else(|_| "Unknown".to_string()),
            difficulty: row.get(3).unwrap_or_else(|_| "Unknown".to_string()),
            fight_start: row.get(4).unwrap_or(0),
            cleared: row.get::<_, i64>(5).unwrap_or(0) == 1,
        })
    }).map_err(|e| format!("Failed to query encounters: {}", e))?;
    
    let mut encounters = Vec::new();
    for encounter in encounters_iter {
        encounters.push(encounter.map_err(|e| format!("Failed to process encounter: {}", e))?);
    }
    
    crate::log_debug!("Found {} cleared encounters since last weekly reset", encounters.len());
    
    Ok(encounters)
}


fn process_encounter(
    encounter: &EncounterPreview,
    boss_mapper: &BossMapper,
    todo_repo: &TodoRepository,
    settings_manager: &crate::settings::SettingsManager,
) -> Result<bool> {
        
    // Map boss name to encounter
    let mapping = match boss_mapper.map_boss_to_encounter(&encounter.current_boss) {
        Some(mapping) => {
                        mapping
        },
        None => {
                        crate::log_debug!("Skipping encounter: Boss '{}' not found in boss_mapping", encounter.current_boss);
                        return Ok(false);
        }
    };

    // Normalize difficulty
    let normalized_difficulty = boss_mapper.normalize_difficulty(&encounter.difficulty);
    
    // Find character ID by name
    let char_id = match find_character_id_by_name(todo_repo, &encounter.local_player)? {
        Some(char_id) => {
                        char_id
        },
        None => {
                        crate::log_debug!("Skipping encounter: Character '{}' not found in database", encounter.local_player);
                        return Ok(false); // Skip this encounter
        }
    };

    // Generate session_id for the gate; only create a new entry if it doesn't already exist
    let session_id = format!(
        "{}_Gate {}",
        mapping.content_id, mapping.gate
    );

    let conn = todo_repo.pool.get()?;
    let existing_entry: Option<(i64, Option<i64>)> = conn.query_row(
        "SELECT is_completed, timestamp FROM completion_status 
         WHERE char_id = ?1 AND content_id = ?2 AND session_id = ?3 LIMIT 1",
        params![char_id, &mapping.content_id, &session_id],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).optional()?;

    match existing_entry {
        Some((is_completed, existing_timestamp)) => {
            if is_completed == 1 {
                crate::log_debug!("Skipping encounter: {} - {} already completed for char_id {}", encounter.local_player, encounter.current_boss, char_id);
                return Ok(false);
            }

            // Mark existing entry as completed instead of inserting a duplicate row
            conn.execute(
                "UPDATE completion_status SET is_completed = 1, timestamp = ?1, completion_source = ?2, details = ?3 WHERE char_id = ?4 AND content_id = ?5 AND session_id = ?6",
                params![encounter.fight_start, "LOAlogs", normalized_difficulty, char_id, &mapping.content_id, &session_id],
            )?;

            crate::log_debug!(
                "Updated completion_status for {} (char_id={}, content_id={}, session_id={})",
                encounter.current_boss, char_id, mapping.content_id, session_id
            );
        }
        None => {
            // Create new completion entry when there is no existing gate-specific session_id
            create_completion_entry(
                todo_repo,
                char_id,
                &mapping.content_id,
                &session_id,
                encounter.fight_start,
                &normalized_difficulty,
            )?;
        }
    }

    // Sync entity data for this encounter
    if let Err(e) = sync_entity_data_for_encounter(todo_repo, &*settings_manager, encounter.id) {
        // Log error but don't fail the encounter processing
        crate::log_warn!("Failed to sync entity data for encounter {}: {}", encounter.id, e);
    }

    crate::log_debug!(
        "Synced encounter: {} (Player: {}) -> {} (Gate {})",
        encounter.current_boss, encounter.local_player, mapping.content_id, mapping.gate
    );

    Ok(true)
}

fn sync_entity_data_for_encounter(todo_repo: &TodoRepository, settings_manager: &crate::settings::SettingsManager, encounter_id: i64) -> Result<()> {
    use crate::handlers::entity_sync_handlers::{get_entities_for_encounter, update_character_from_entity};
    
    // Get encounters.db path from JSON settings
    let encounters_db_path = get_encounters_db_path_from_settings(settings_manager)
        .map_err(|e| anyhow::anyhow!("Failed to get encounters db path: {}", e))?;
    
    // Get entity data for this encounter
    let entities = get_entities_for_encounter(&encounters_db_path, encounter_id)
        .map_err(|e| anyhow::anyhow!("Failed to get entities: {}", e))?;
    
    // Process each entity
    for entity in entities {
        if let Err(e) = update_character_from_entity(todo_repo, &entity) {
            crate::log_warn!("Failed to update character {}: {}", entity.name, e);
        }
    }
    
    Ok(())
}

fn find_character_id_by_name(todo_repo: &TodoRepository, character_name: &str) -> Result<Option<i64>> {
    let conn = todo_repo.pool.get()?;
    
    // Try exact match first
    let mut stmt = conn.prepare(
        "SELECT char_id FROM conf_character WHERE char_name = ?1"
    )?;
    
    let result: Option<i64> = stmt
        .query_row([character_name], |row| row.get(0))
        .optional()?;
    
    if result.is_some() {
        return Ok(result);
    }
    
    // If exact match fails, try case-insensitive match
    let mut stmt = conn.prepare(
        "SELECT char_id FROM conf_character WHERE LOWER(char_name) = LOWER(?1)"
    )?;
    
    let result: Option<i64> = stmt
        .query_row([character_name], |row| row.get(0))
        .optional()?;
    
    if result.is_none() {
        crate::log_debug!("Character '{}' not found in conf_character table", character_name);
    }
    
    Ok(result)
}

fn entry_already_exists(
    todo_repo: &TodoRepository,
    char_id: i64,
    content_id: &str,
    session_id: &str,
) -> Result<bool> {
    let conn = todo_repo.pool.get()?;
    
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) as count FROM completion_status 
         WHERE char_id = ?1 AND content_id = ?2 AND session_id = ?3"
    )?;
    
    let count: i64 = stmt.query_row(params![char_id, content_id, session_id], |row| {
        row.get(0)
    })?;
    
    Ok(count > 0)
}

fn create_completion_entry(
    todo_repo: &TodoRepository,
    char_id: i64,
    content_id: &str,
    session_id: &str,
    timestamp: i64,
    difficulty: &str,
) -> Result<()> {
    let conn = todo_repo.pool.get()?;
    
    // Get roster_id for this character
    let roster_id: String = conn.query_row(
        "SELECT roster_id FROM conf_character WHERE char_id = ?1",
        [char_id],
        |row| row.get(0)
    )?;
    
    // Insert new completion entry
    // Keep timestamp in milliseconds for consistency
    let normalized_timestamp = timestamp;
    
    conn.execute(
        "INSERT INTO completion_status 
         (roster_id, char_id, content_id, is_completed, timestamp, session_id, completion_source, details) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            roster_id,
            char_id,
            content_id,
            1i64, // is_completed = true
            normalized_timestamp,
            session_id,
            "LOAlogs",
            difficulty
        ]
    )?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_generation() {
        let session_id = format!("act_3_mordum_Gate {}_hard", 1);
        assert_eq!(session_id, "act_3_mordum_Gate 1_hard");
    }

    #[test]
    fn test_boss_mapping_integration() {
        let boss_mapper = BossMapper::new();
        
        let mapping = boss_mapper.map_boss_to_encounter("Thaemine, Master of Darkness");
        assert!(mapping.is_some());
        
        let mapping = mapping.unwrap();
        assert_eq!(mapping.content_id, "act_3_mordum");
        assert_eq!(mapping.gate, 1);
    }
}
