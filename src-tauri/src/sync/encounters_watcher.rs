use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, command};
use serde_json::json;
use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};

use crate::database::repositories::todo_repository::TodoRepository;
use crate::handlers::encounter_sync_handlers::{sync_encounters_to_completions, sync_encounters_to_completions_internal};

/// File watcher for encounters.db changes that triggers live sync
pub struct EncountersFileWatcher {
    app: AppHandle,
    todo_repo: Arc<TodoRepository>,
    last_sync: Arc<Mutex<Instant>>,
    encounters_db_path: String,
}

impl EncountersFileWatcher {
    pub fn new(app: AppHandle, todo_repo: Arc<TodoRepository>, encounters_db_path: String) -> Self {
        Self {
            app,
            todo_repo,
            last_sync: Arc::new(Mutex::new(Instant::now())),
            encounters_db_path,
        }
    }

    /// Start watching encounters.db for file changes and sync new entries immediately
    pub fn start_watching(&self) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.app.clone();
        let todo_repo = self.todo_repo.clone();
        let last_sync = self.last_sync.clone();
        let encounters_db_path = self.encounters_db_path.clone();
        let watch_path = PathBuf::from(&encounters_db_path);
        let print_watch_path = watch_path.clone();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<notify::Result<notify::Event>>();

        let mut watcher = recommended_watcher(move |res| {
            let _ = tx.send(res);
        })?;

        watcher.watch(&watch_path, RecursiveMode::NonRecursive)?;

        tauri::async_runtime::spawn(async move {
            let _watcher = watcher;
            while let Some(event_res) = rx.recv().await {
                match event_res {
                    Ok(event) => {
                        let is_relevant = matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_))
                            && event.paths.iter().any(|path| path == &watch_path || path.ends_with(&watch_path));

                        if !is_relevant {
                            continue;
                        }

                        let now = Instant::now();
                        let should_sync = {
                            let last_sync_guard = last_sync.lock().unwrap();
                            now.duration_since(*last_sync_guard) > Duration::from_secs(10)
                        };

                        if !should_sync {
                            continue;
                        }

                        crate::log_info!("Detected encounters.db change, starting auto sync: {}", &encounters_db_path);
                        // Give the external writer a moment to finish updating the file.
                        tokio::time::sleep(Duration::from_secs(1)).await;

                        let settings_manager = app.state::<crate::settings::SettingsManager>();
                        let sync_result = sync_encounters_to_completions_internal(
                            app.clone(),
                            todo_repo.clone(),
                            settings_manager.inner(),
                        );

                        let sync_payload = match sync_result {
                            Ok(result) => {
                                *last_sync.lock().unwrap() = now;
                                json!({
                                    "synced_count": result.synced_count,
                                    "skipped_count": result.skipped_count,
                                    "errors": result.errors,
                                    "duration_ms": result.duration_ms,
                                    "trigger": "file_change",
                                    "encounters_db_path": encounters_db_path,
                                    "message": "Auto synced new encounters from encounters.db"
                                })
                            }
                            Err(error_message) => {
                                crate::log_error!("Encounter auto-sync failed: {}", error_message);
                                json!({
                                    "synced_count": 0,
                                    "skipped_count": 0,
                                    "errors": [error_message],
                                    "duration_ms": 0,
                                    "trigger": "file_change",
                                    "encounters_db_path": encounters_db_path,
                                    "message": "Encounter auto-sync failed"
                                })
                            }
                        };

                        let _ = app.emit("encounters-auto-sync-complete", sync_payload);
                    }
                    Err(err) => {
                        crate::log_error!("Encounter watcher error: {}", err);
                    }
                }
            }
        });

        crate::log_info!("Started encounters.db watcher for path {:?}", print_watch_path);
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

