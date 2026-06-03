use crate::database::repositories::TodoRepository;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

use crate::handlers::encounter_sync_handlers::get_encounters_db_path_from_settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityData {
    pub name: String,
    pub encounter_id: i64,
    pub gear_score: f64,
    pub combat_power: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySyncResult {
    pub processed_count: usize,
    pub updated_count: usize,
    pub skipped_count: usize,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[tauri::command]
/// Syncs gear score and combat power from one LOA Logs encounter's entity rows.
pub async fn sync_entity_data(
    todo_repo: State<'_, Arc<TodoRepository>>,
    settings_manager: State<'_, crate::settings::SettingsManager>,
    encounter_id: i64,
) -> Result<EntitySyncResult, String> {
    let start_time = std::time::Instant::now();

    // Get encounters.db path from JSON settings
    let encounters_db_path = get_encounters_db_path_from_settings(&*settings_manager)?;

    // Get entity data for this encounter
    let entities = match get_entities_for_encounter(&encounters_db_path, encounter_id) {
        Ok(entities) => entities,
        Err(e) => {
            return Ok(EntitySyncResult {
                processed_count: 0,
                updated_count: 0,
                skipped_count: 0,
                errors: vec![format!("Failed to read entity data: {}", e)],
                duration_ms: start_time.elapsed().as_millis() as u64,
            });
        }
    };

    let mut processed_count = 0;
    let mut updated_count = 0;
    let mut skipped_count = 0;
    let mut errors = Vec::new();

    // Process each entity
    for entity in entities {
        processed_count += 1;

        match update_character_from_entity(&*todo_repo, &entity) {
            Ok(updated) => {
                if updated {
                    updated_count += 1;
                } else {
                    skipped_count += 1;
                }
            }
            Err(e) => {
                errors.push(format!("Failed to update character {}: {}", entity.name, e));
            }
        }
    }

    Ok(EntitySyncResult {
        processed_count,
        updated_count,
        skipped_count,
        errors,
        duration_ms: start_time.elapsed().as_millis() as u64,
    })
}

/// Reads character entity rows for one encounter from LOA Logs.
pub fn get_entities_for_encounter(encounters_db_path: &str, encounter_id: i64) -> Result<Vec<EntityData>, String> {
    let conn = rusqlite::Connection::open_with_flags(encounters_db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("Failed to open encounters.db in read-only mode: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT name, encounter_id, gear_score, combat_power 
         FROM entity 
         WHERE encounter_id = ? AND gear_score > 0",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let entities_iter = stmt
        .query_map([encounter_id], |row| {
            Ok(EntityData {
                name: row.get(0)?,
                encounter_id: row.get(1)?,
                gear_score: row.get(2)?,
                combat_power: row.get(3).unwrap_or(0.0),
            })
        })
        .map_err(|e| format!("Failed to query entities: {}", e))?;

    let mut entities = Vec::new();
    for entity_result in entities_iter {
        entities.push(entity_result.map_err(|e| format!("Failed to parse entity: {}", e))?);
    }

    Ok(entities)
}

/// Updates a known local character from a LOA Logs entity row.
///
/// Entity sync only refreshes scraped live stats. It does not create characters,
/// move rosters, or touch raid/tracking settings.
pub fn update_character_from_entity(todo_repo: &TodoRepository, entity: &EntityData) -> Result<bool, String> {
    let conn = todo_repo
        .pool
        .get()
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let current_stats: Option<(f64, f64)> = conn
        .query_row(
            "SELECT COALESCE(item_level, 0), COALESCE(combat_power, 0)
             FROM conf_character
             WHERE char_name = ?1
             LIMIT 1",
            [entity.name.clone()],
            |row| Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?)),
        )
        .optional()
        .map_err(|e| format!("Failed to load current character stats: {}", e))?;

    let Some((current_item_level, current_combat_power)) = current_stats else {
        return Ok(false); // Skip if character doesn't exist
    };

    // Format gear_score to 2 decimal places
    let formatted_item_level = (entity.gear_score * 100.0).round() / 100.0;

    let item_level_update = if formatted_item_level > current_item_level {
        Some(formatted_item_level)
    } else {
        None
    };

    // Only update combat_power when LOA Logs has a real value and it is newer/higher.
    let combat_power_update = if entity.combat_power > current_combat_power {
        Some(entity.combat_power)
    } else {
        None
    };

    if item_level_update.is_none() && combat_power_update.is_none() {
        return Ok(false);
    }

    let rows_affected = match (item_level_update, combat_power_update) {
        (Some(ilvl), Some(cp)) => conn.execute(
            "UPDATE conf_character
             SET item_level = ?1, combat_power = ?2
             WHERE char_name = ?3",
            params![ilvl, cp, entity.name.clone()],
        ),
        (Some(ilvl), None) => conn.execute(
            "UPDATE conf_character
             SET item_level = ?1
             WHERE char_name = ?2",
            params![ilvl, entity.name.clone()],
        ),
        (None, Some(cp)) => conn.execute(
            "UPDATE conf_character
             SET combat_power = ?1
             WHERE char_name = ?2",
            params![cp, entity.name.clone()],
        ),
        (None, None) => Ok(0),
    }
    .map_err(|e| format!("Failed to update character: {}", e))?;

    Ok(rows_affected > 0)
}

#[tauri::command]
/// Syncs entity data for all encounters from the last 24 hours.
pub async fn sync_all_recent_entities(
    todo_repo: State<'_, Arc<TodoRepository>>,
    settings_manager: State<'_, crate::settings::SettingsManager>,
) -> Result<EntitySyncResult, String> {
    let start_time = std::time::Instant::now();

    // Get encounters.db path from JSON settings
    let encounters_db_path = get_encounters_db_path_from_settings(&*settings_manager)?;

    // Get recent encounter IDs (last 24 hours)
    let recent_encounters = match get_recent_encounter_ids(&encounters_db_path) {
        Ok(encounters) => encounters,
        Err(e) => {
            return Ok(EntitySyncResult {
                processed_count: 0,
                updated_count: 0,
                skipped_count: 0,
                errors: vec![format!("Failed to get recent encounters: {}", e)],
                duration_ms: start_time.elapsed().as_millis() as u64,
            });
        }
    };

    let mut total_processed = 0;
    let mut total_updated = 0;
    let mut total_skipped = 0;
    let mut all_errors = Vec::new();

    // Process each encounter
    for encounter_id in recent_encounters {
        match sync_entity_data(todo_repo.clone(), settings_manager.clone(), encounter_id).await {
            Ok(result) => {
                total_processed += result.processed_count;
                total_updated += result.updated_count;
                total_skipped += result.skipped_count;
                all_errors.extend(result.errors);
            }
            Err(e) => {
                all_errors.push(format!("Failed to sync encounter {}: {}", encounter_id, e));
            }
        }
    }

    Ok(EntitySyncResult {
        processed_count: total_processed,
        updated_count: total_updated,
        skipped_count: total_skipped,
        errors: all_errors,
        duration_ms: start_time.elapsed().as_millis() as u64,
    })
}

/// Returns encounter ids from the last 24 hours.
fn get_recent_encounter_ids(encounters_db_path: &str) -> Result<Vec<i64>, String> {
    let conn = rusqlite::Connection::open_with_flags(encounters_db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("Failed to open encounters.db in read-only mode: {}", e))?;

    // `encounter_preview.fight_start` is stored in milliseconds, matching the
    // timestamps used by local completion_status.
    let one_day_ago = chrono::Utc::now().timestamp_millis() - (86400 * 1000);

    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT id FROM encounter_preview
         WHERE fight_start >= ?
         ORDER BY id DESC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let encounters_iter = stmt
        .query_map([one_day_ago], |row| Ok(row.get(0)?))
        .map_err(|e| format!("Failed to query recent encounters: {}", e))?;

    let mut encounter_ids = Vec::new();
    for encounter_result in encounters_iter {
        encounter_ids.push(encounter_result.map_err(|e| format!("Failed to parse encounter: {}", e))?);
    }

    Ok(encounter_ids)
}
