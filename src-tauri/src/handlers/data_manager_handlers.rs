use crate::database::data_manager::{DataManager, GameClass, GameTask, Raid};
use crate::database::repositories::CharacterRepository;
use crate::database::repositories::RosterRepository;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializationData {
    pub tasks: HashMap<String, GameTask>,
    pub raids: Vec<Raid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializationDataWithClasses {
    pub tasks: HashMap<String, GameTask>,
    pub raids: Vec<Raid>,
    pub classes: HashMap<String, GameClass>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BootstrapSnapshot {
    pub rosters: Vec<crate::models::Roster>,
    pub characters: Vec<crate::roster::Character>,
    pub next_daily_reset: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    pub characters: Vec<crate::roster::Character>,
    pub rested_by_character: HashMap<i64, Vec<crate::database::repositories::character_repository::RestedValue>>,
    pub completion_by_character:
        HashMap<i64, Vec<crate::database::repositories::character_repository::CompletionStatus>>,
    pub roster_completion_status: Vec<crate::database::repositories::character_repository::CompletionStatus>,
    pub tracking_by_character: HashMap<i64, Vec<crate::database::repositories::character_repository::TrackingStatus>>,
    pub roster_tracking_status: Vec<crate::database::repositories::character_repository::TrackingStatus>,
    pub raid_configs_by_character:
        HashMap<i64, Vec<crate::database::repositories::character_repository::CharacterRaidConfig>>,
}

/// Loads the minimal app startup snapshot in one command.
///
/// The frontend uses this to hydrate rosters, characters, and the next daily
/// reset without issuing separate startup requests for each piece.
#[tauri::command]
pub async fn get_app_bootstrap_snapshot(
    roster_repo: tauri::State<'_, RosterRepository>,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> Result<BootstrapSnapshot, String> {
    let rosters = roster_repo
        .get_all_rosters()
        .map_err(|e| format!("Failed to load rosters for bootstrap snapshot: {}", e))?;

    let mut characters = Vec::new();
    for roster in &rosters {
        let mut roster_characters = roster_repo
            .get_characters_by_roster(&roster.id)
            .map_err(|e| format!("Failed to load characters for roster {}: {}", roster.id, e))?;
        characters.append(&mut roster_characters);
    }

    let reset_service = crate::services::reset_service::ResetService::new(pool.inner().clone());
    let next_daily_reset = reset_service
        .get_next_reset_time("chaos")
        .map_err(|e| format!("Failed to calculate next daily reset: {}", e))?
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();

    Ok(BootstrapSnapshot {
        rosters,
        characters,
        next_daily_reset,
    })
}

/// Loads dashboard-specific character state in batch form.
///
/// This avoids one command per character for rested values, completion state,
/// tracking visibility, and raid settings.
#[tauri::command]
pub async fn get_dashboard_snapshot(
    roster_id: Option<String>,
    roster_repo: tauri::State<'_, RosterRepository>,
    character_repo: tauri::State<'_, CharacterRepository>,
) -> Result<DashboardSnapshot, String> {
    let characters = if let Some(ref id) = roster_id {
        roster_repo
            .get_characters_by_roster(id)
            .map_err(|e| format!("Failed to load characters for roster {}: {}", id, e))?
    } else {
        let rosters = roster_repo
            .get_all_rosters()
            .map_err(|e| format!("Failed to load rosters for dashboard snapshot: {}", e))?;
        let mut all_characters = Vec::new();
        for roster in rosters {
            let mut roster_characters = roster_repo
                .get_characters_by_roster(&roster.id)
                .map_err(|e| format!("Failed to load characters for roster {}: {}", roster.id, e))?;
            all_characters.append(&mut roster_characters);
        }
        all_characters
    };

    let char_ids: Vec<i64> = characters.iter().map(|c| c.char_id).collect();

    let rested_by_character = character_repo
        .get_batch_rested_values(&char_ids)
        .map_err(|e| format!("Failed to load rested values: {}", e))?;

    let completion_by_character = character_repo
        .get_batch_completion_status(&char_ids)
        .map_err(|e| format!("Failed to load completion status: {}", e))?;

    let roster_completion_status = if let Some(ref id) = roster_id {
        character_repo
            .get_roster_completion_status(id)
            .map_err(|e| format!("Failed to load roster completion status: {}", e))?
    } else {
        Vec::new()
    };

    let tracking_by_character = character_repo
        .get_batch_tracking_status(&char_ids)
        .map_err(|e| format!("Failed to load tracking status: {}", e))?;

    let roster_tracking_status = if let Some(ref id) = roster_id {
        character_repo
            .get_roster_tracking_status(id)
            .map_err(|e| format!("Failed to load roster tracking status: {}", e))?
    } else {
        Vec::new()
    };

    let raid_configs_by_character = character_repo
        .get_batch_raid_configs(&char_ids)
        .map_err(|e| format!("Failed to load raid configs: {}", e))?;

    Ok(DashboardSnapshot {
        characters,
        rested_by_character,
        completion_by_character,
        roster_completion_status,
        tracking_by_character,
        roster_tracking_status,
        raid_configs_by_character,
    })
}

/// Initializes static app data from the frontend source-of-truth payload.
///
/// The frontend owns the display/game-data definitions in `src/lib/data/*`;
/// the backend stores a local fallback copy for reset and migration paths.
#[tauri::command]
pub async fn initialize_application_data(
    data: InitializationDataWithClasses,
    db_manager: tauri::State<'_, crate::database::DatabaseManager>,
) -> Result<String, String> {
    match DataManager::initialize_default_data(&db_manager.pool, data.tasks, data.raids, data.classes) {
        Ok(_) => Ok("Application data initialized successfully".to_string()),
        Err(e) => Err(format!("Failed to initialize application data: {}", e)),
    }
}

/// Backfills missing per-character reset/tracking/raid rows after data changes.
#[tauri::command]
pub async fn ensure_character_data_complete(
    data: InitializationData,
    db_manager: tauri::State<'_, crate::database::DatabaseManager>,
) -> Result<String, String> {
    match DataManager::ensure_character_data_complete(&db_manager.pool, data.tasks, data.raids) {
        Ok(_) => Ok("Character data consistency check completed".to_string()),
        Err(e) => Err(format!("Failed to ensure character data completeness: {}", e)),
    }
}

/// Initializes local reset/tracking/raid rows for one newly scraped character.
#[tauri::command]
pub async fn initialize_character_data(
    character_id: i64,
    roster_id: String,
    data: InitializationDataWithClasses,
    db_manager: tauri::State<'_, crate::database::DatabaseManager>,
) -> Result<String, String> {
    match DataManager::initialize_character_data(&db_manager.pool, character_id, &roster_id, data.tasks, data.raids) {
        Ok(_) => Ok("Character data initialized successfully".to_string()),
        Err(e) => Err(format!("Failed to initialize character data: {}", e)),
    }
}

/// Refreshes reset metadata timestamps after startup or manual reset checks.
#[tauri::command]
pub async fn update_reset_timestamps(
    db_manager: tauri::State<'_, crate::database::DatabaseManager>,
) -> Result<String, String> {
    match DataManager::update_reset_timestamps(&db_manager.pool) {
        Ok(_) => Ok("Reset timestamps updated successfully".to_string()),
        Err(e) => Err(format!("Failed to update reset timestamps: {}", e)),
    }
}

/// Returns the local SQLite schema version used by the frontend migration check.
#[tauri::command]
pub async fn get_schema_version(db_manager: tauri::State<'_, crate::database::DatabaseManager>) -> Result<i64, String> {
    match DataManager::get_schema_version(&db_manager.pool) {
        Ok(version) => Ok(version as i64),
        Err(e) => Err(format!("Failed to get schema version: {}", e)),
    }
}

/// Runs local SQLite migrations up to the requested schema version.
#[tauri::command]
pub async fn migrate_database(
    target_version: i64,
    db_manager: tauri::State<'_, crate::database::DatabaseManager>,
) -> Result<String, String> {
    let current_version = match DataManager::get_schema_version(&db_manager.pool) {
        Ok(v) => v as i64,
        Err(e) => return Err(format!("Failed to get current schema version: {}", e)),
    };

    match DataManager::migrate_database(&db_manager.pool, current_version as i32, target_version as i32) {
        Ok(_) => Ok(format!(
            "Database migrated from version {} to {}",
            current_version, target_version
        )),
        Err(e) => Err(format!("Failed to migrate database: {}", e)),
    }
}
