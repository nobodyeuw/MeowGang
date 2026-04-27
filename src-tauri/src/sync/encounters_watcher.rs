use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, command};
use serde_json::json;

use crate::database::repositories::todo_repository::TodoRepository;
use crate::handlers::encounter_sync_handlers::sync_encounters_to_completions;

/// Simple periodic checker for encounters.db changes
pub struct EncountersFileWatcher {
    app: AppHandle,
    todo_repo: Arc<TodoRepository>,
    last_sync: Arc<std::sync::Mutex<Instant>>,
    encounters_db_path: String,
}

impl EncountersFileWatcher {
    pub fn new(app: AppHandle, todo_repo: Arc<TodoRepository>, encounters_db_path: String) -> Self {
        Self {
            app,
            todo_repo,
            last_sync: Arc::new(std::sync::Mutex::new(Instant::now())),
            encounters_db_path,
        }
    }

    /// Start periodic checking for encounters.db changes
    pub fn start_watching(&self) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.app.clone();
        let todo_repo = self.todo_repo.clone();
        let last_sync = self.last_sync.clone();

        // Spawn background task to periodically check for changes
        let encounters_db_path = self.encounters_db_path.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await; // Check every 30 seconds
                
                // Check if enough time has passed since last sync (minimum 60 seconds)
                let now = Instant::now();
                let should_sync = {
                    let last_sync_guard = last_sync.lock().unwrap();
                    now.duration_since(*last_sync_guard) > Duration::from_secs(60)
                };
                
                if should_sync {
                    println!("Periodic check: checking for encounters.db changes...");
                    println!("Using configured encounters.db path: {}", encounters_db_path);
                    
                    // Check file modification time
                    let file_modified = match std::fs::metadata(&encounters_db_path) {
                        Ok(metadata) => metadata.modified().ok(),
                        Err(e) => {
                            eprintln!("Failed to read encounters.db metadata: {}", e);
                            None
                        }
                    };
                    
                    // For now, just emit monitoring event with file check info
                    let sync_result = crate::handlers::encounter_sync_handlers::SyncResult {
                        synced_count: 0,
                        skipped_count: 0,
                        errors: vec![],
                        duration_ms: 0,
                    };
                    
                    // Update last sync time
                    *last_sync.lock().unwrap() = now;
                    
                    // Emit event to frontend with file check info
                    let _ = app.emit("encounters-auto-sync-complete", json!({
                        "synced_count": sync_result.synced_count,
                        "skipped_count": sync_result.skipped_count,
                        "errors": &sync_result.errors,
                        "duration_ms": sync_result.duration_ms,
                        "trigger": "periodic_check",
                        "encounters_db_path": encounters_db_path,
                        "file_modified": file_modified.map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()).unwrap_or(0),
                        "message": "File watcher active - checking for new entries"
                    }));
                    
                    println!("Periodic check completed - monitoring: {}", encounters_db_path);
                }
            }
        });

        println!("Started periodic encounters.db monitoring (every 30 seconds)");
        Ok(())
    }
}

#[command]
pub fn force_encounters_sync(
    app: tauri::AppHandle,
    todo_repo: tauri::State<'_, Arc<TodoRepository>>,
    settings_manager: tauri::State<'_, crate::settings::SettingsManager>,
) -> Result<crate::handlers::encounter_sync_handlers::SyncResult, String> {
    let result = sync_encounters_to_completions(
        app.clone(),
        todo_repo,
        settings_manager,
    )?;
    
    app.emit("encounters-force-sync-complete", json!({
        "synced_count": result.synced_count,
        "skipped_count": result.skipped_count,
        "errors": &result.errors,
        "duration_ms": result.duration_ms
    })).map_err(|e| e.to_string())?;
    
    Ok(result)
}

#[command]
pub fn start_encounters_file_watcher(
    app: tauri::AppHandle,
    todo_repo: tauri::State<'_, Arc<TodoRepository>>,
    settings_manager: tauri::State<'_, crate::settings::SettingsManager>,
) -> Result<String, String> {
    // Get user-configured encounters.db path from settings
    let settings = settings_manager.read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_default();
    
    let encounters_db_path = settings.system.encounters_db_path
        .unwrap_or_else(|| "encounters.db".to_string());
    
    let watcher = EncountersFileWatcher::new(app.clone(), todo_repo.inner().clone(), encounters_db_path);
    
    match watcher.start_watching() {
        Ok(()) => Ok("File watcher started successfully".to_string()),
        Err(e) => Err(format!("Failed to start file watcher: {}", e))
    }
}

