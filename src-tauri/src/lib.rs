// Main application modules
pub mod app;
pub mod handlers;
pub mod database;
pub mod roster;
pub mod models;
pub mod init;
pub mod sync;
pub mod services;
pub mod state;
pub mod version;
pub mod context;
pub mod settings;
pub mod validation;
pub mod market;

// Re-export commonly used items
pub use database::DatabaseManager;
pub use std::sync::Arc;
pub use roster::HumanizedScraper;

// Tauri application setup
use dirs;
use tauri::Manager;
use database::repositories::{RosterRepository, TrackingRepository, RaidRepository, CharacterRepository, GoldRepository};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    crate::log_info!("Starting LOA Tracker application");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            crate::log_info!("Initializing Tauri application setup");
            
            // Initialize application context like reference project
            let package_info = app.package_info();
            let mut app_context = context::AppContext::new(package_info.version.to_string())
                .expect("Could not create application context");
            
            // Update paths using Tauri's path resolution for consistency
            app_context.update_paths_with_tauri(app.handle())
                .expect("Could not update paths with Tauri");
            
            // Ensure necessary directories exist
            app_context.ensure_directories()
                .expect("Could not ensure directories exist");
            
            // Initialize database setup (synchronous, fast) - AFTER path update
            let db_path = init::ensure_database_setup(&app_context.app_data_dir)?;
            crate::log_info!("Database path established: {:?}", db_path);
            
            // Initialize separate market database for progression planner
            let market_db_path = app_context.app_data_dir.join("market.db");
            let market_db_path_str = market_db_path.to_str()
                .ok_or_else(|| format!("Market DB path contains invalid UTF-8: {:?}", market_db_path))?;
            let market_db = market::MarketDatabase::new(market_db_path_str)
                .map_err(|e| {
                    crate::log_error!("Failed to initialize market database: {}", e);
                    format!("Failed to initialize market database: {}", e)
                })?;
            crate::log_info!("Market database initialized successfully");

            // Seed gem entries (manual-only, no API source)
            market_db.seed_gem_entries()
                .map_err(|e| {
                    crate::log_error!("Failed to seed gem entries: {}", e);
                    format!("Failed to seed gem entries: {}", e)
                })?;

            let db_path_str = db_path.to_str()
                .ok_or_else(|| format!("Database path contains invalid UTF-8: {:?}", db_path))?;
            let db_manager = DatabaseManager::new(db_path_str)
                .map_err(|e| {
                    crate::log_error!("Failed to initialize database: {}", e);
                    format!("Failed to initialize database: {}", e)
                })?;
            crate::log_info!("Database manager initialized successfully");
            
            // Initialize settings manager (AFTER path update)
            let settings_manager = settings::SettingsManager::new(app_context.settings_path.clone())
                .map_err(|e| {
                    crate::log_error!("Failed to create settings manager: {}", e);
                    format!("Failed to create settings manager: {}", e)
                })?;
            
            // Ensure settings file exists with defaults
            let _settings = settings_manager.ensure_exists()
                .map_err(|e| {
                    crate::log_error!("Failed to ensure settings exist: {}", e);
                    format!("Failed to ensure settings exist: {}", e)
                })?;
            crate::log_info!("Settings manager initialized successfully");
            
            // Initialize repositories
            crate::log_debug!("Initializing database repositories");
            let roster_repo = RosterRepository::new(db_manager.pool.clone());
            let tracking_repo = TrackingRepository::new(db_manager.pool.clone());
            let raid_repo = RaidRepository::new(db_manager.pool.clone());
            let character_repo = CharacterRepository::new(db_manager.pool.clone());
            let gold_repo = GoldRepository::new(db_manager.pool.clone());
            crate::log_info!("All repositories initialized successfully");
            
            // Initialize scraper with placeholder values; actual roster name/character
            // are set per-request in the handler layer.
            let scraper = roster::HumanizedScraper::new(String::new(), String::new());
            crate::log_debug!("Roster scraper initialized");
            
            // Initialize raid data state for frontend-driven gold logging
            let raid_data_state = state::RaidDataState::new();
            
            // Initialize logging system with Tauri path resolution
            crate::log_info!("Initializing logging system");
            services::logging_service::init_logging_with_app_handle(app.handle())
                .map_err(|e| {
                    crate::log_error!("Failed to initialize logging: {}", e);
                    format!("Failed to initialize logging: {}", e)
                })?;
            crate::log_info!("Logging system initialized successfully");
            
            // Initialize gold logging service
            let gold_service = services::gold_logging_service::GoldLoggingService::new(Arc::new(db_manager.pool.clone()));
            
            // Clean up any existing duplicate gold log entries on startup
            match gold_service.clean_all_duplicate_gold_logs() {
                Ok(count) if count > 0 => crate::log_info!("Cleaned {} duplicate gold log entries on startup", count),
                Ok(_) => {},
                Err(e) => crate::log_warn!("Failed to clean duplicate gold logs: {}", e),
            }
            
            // Manage the application state
            app.manage(app_context.clone());
            app.manage(settings_manager);
            app.manage(db_manager.clone());
            app.manage(db_manager.pool.clone()); // Manage pool separately for reset handlers
            app.manage(roster_repo);
            app.manage(tracking_repo);
            app.manage(raid_repo);
            app.manage(character_repo);
            app.manage(gold_repo);
            app.manage(scraper);
            app.manage(raid_data_state);
            app.manage(gold_service);
            app.manage(market_db);
            
            // Create and manage TodoRepository
            let todo_repo = database::repositories::TodoRepository::new(Arc::new(db_manager.pool.clone()));
            let todo_repo_arc = Arc::new(todo_repo);
            app.manage(todo_repo_arc.clone());
            
            // Initialize database with default data if needed
            crate::log_debug!("Checking if default data initialization is needed");
            if let Err(e) = database::data_manager::DataManager::initialize_default_data(
                &db_manager.pool,
                std::collections::HashMap::new(),
                vec![],
                std::collections::HashMap::new(),
            ) {
                crate::log_error!("Failed to initialize default database data: {}", e);
            } else {
                crate::log_info!("Default data initialization completed");
            }

            // Initialize schema version check and migration
            let current_version = database::data_manager::DataManager::get_schema_version(&db_manager.pool)
                .unwrap_or(1);
            const TARGET_VERSION: i32 = 3;
            crate::log_info!("Current schema version: {}, target version: {}", current_version, TARGET_VERSION);
            
            if current_version < TARGET_VERSION {
                crate::log_info!("Starting database migration from version {} to {}", current_version, TARGET_VERSION);
                database::data_manager::DataManager::migrate_database(&db_manager.pool, current_version, TARGET_VERSION)?;
                crate::log_info!("Database migration completed successfully");
            }
            
            // Automatically check and update reset timestamps
            crate::log_debug!("Updating reset timestamps");
            database::data_manager::DataManager::update_reset_timestamps(&db_manager.pool)?;
            crate::log_info!("Reset timestamps updated");
            
            // Start scheduled daily reset service
            crate::log_info!("Starting scheduled daily reset service");
            crate::services::reset_service::ResetService::start_scheduled_reset(db_manager.pool.clone());
            crate::log_info!("Daily reset service started");
            
            // Start encounters file watcher for real-time updates
            crate::log_info!("Starting encounters file watcher initialization");
            let todo_repo_for_watcher = todo_repo_arc.clone();
            let app_for_watcher = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                crate::log_debug!("Encounters watcher: waiting for app initialization");
                // Small delay to ensure app is fully initialized
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                
                // Start the file watcher using the existing watcher struct
                // Try to auto-detect encounters.db path first
                let encounters_db_path = if let Some(detected_path) = crate::handlers::system_handlers::detect_encounters_db_path() {
                    crate::log_info!("Auto-detected encounters.db path: {}", detected_path);
                    detected_path
                } else {
                    crate::log_info!("Using default encounters.db path, user will need to configure it");
                    "encounters.db".to_string()
                };
                
                let watcher = crate::sync::encounters_watcher::EncountersFileWatcher::new(
                    app_for_watcher, 
                    todo_repo_for_watcher.clone(),
                    encounters_db_path
                );
                
                if let Err(e) = watcher.start_watching() {
                    crate::log_error!("Failed to start encounters file watcher: {}", e);
                } else {
                    crate::log_info!("Encounters file watcher started successfully");
                }
            });
            
            // Perform daily roster scraping if needed
            crate::log_info!("Initializing daily roster scraping service");
            let todo_repo_for_scraping = todo_repo_arc.clone();
            
            // Run daily scraping in background
            tauri::async_runtime::spawn(async move {
                crate::log_debug!("Starting daily roster scraping task");
                // Call the function directly with the repositories
                use crate::handlers::sync_metadata_handlers::get_rosters_needing_daily_scrape;
                
                if let Ok(rosters_needing_scrape) = get_rosters_needing_daily_scrape(&*todo_repo_for_scraping) {
                    crate::log_info!("Found {} rosters needing daily scrape", rosters_needing_scrape.len());
                    for roster_id in rosters_needing_scrape {
                        // For each roster, create a scraper and update character data
                        let characters = match crate::handlers::sync_metadata_handlers::get_characters_for_roster(&*todo_repo_for_scraping, &roster_id) {
                            Ok(chars) => chars,
                            Err(e) => {
                                crate::log_error!("Failed to get characters for roster {}: {}", roster_id, e);
                                continue;
                            }
                        };
                        
                        if let Some(first_char) = characters.first() {
                            let mut scraper = crate::roster::HumanizedScraper::new(
                                first_char.char_name.clone(),
                                roster_id.clone()
                            );
                            
                            match scraper.scrape_roster().await {
                                Ok(scraper_result) => {
                                    let mut updated_count = 0;
                                    for scraped_char in scraper_result.mapped_for_models.roster.characters {
                                        if let Ok(true) = crate::handlers::sync_metadata_handlers::update_character_stats(
                                            &*todo_repo_for_scraping, 
                                            &scraped_char.char_name, 
                                            scraped_char.item_level, 
                                            scraped_char.combat_power
                                        ) {
                                            updated_count += 1;
                                        }
                                    }
                                    
                                    // Update metadata
                                    if let Err(e) = crate::handlers::sync_metadata_handlers::update_roster_scrape_metadata(
                                        &*todo_repo_for_scraping, 
                                        &roster_id, 
                                        "completed", 
                                        Some(&format!("Updated {} characters", updated_count))
                                    ) {
                                        crate::log_error!("Failed to update metadata for roster {}: {}", roster_id, e);
                                    }
                                    
                                    crate::log_info!("Successfully scraped roster {}: {} characters updated", roster_id, updated_count);
                                },
                                Err(e) => {
                                    crate::log_error!("Failed to scrape roster {}: {}", roster_id, e);
                                    
                                    // Update metadata with failure
                                    if let Err(e2) = crate::handlers::sync_metadata_handlers::update_roster_scrape_metadata(
                                        &*todo_repo_for_scraping, 
                                        &roster_id, 
                                        "failed", 
                                        Some(&e.to_string())
                                    ) {
                                        crate::log_error!("Failed to update failure metadata for roster {}: {}", roster_id, e2);
                                    }
                                }
                            }
                        }
                    }
                }
            });
            
            crate::log_info!("LOA Tracker setup completed successfully");
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // System handlers
            handlers::system_handlers::get_app_version,
            handlers::system_handlers::get_system_settings,
            handlers::system_handlers::set_encounters_db_path,
            handlers::system_handlers::set_lost_ark_exe_path,
            handlers::system_handlers::set_start_with_windows,
            handlers::system_handlers::set_start_with_lost_ark,
            handlers::system_handlers::is_lost_ark_running,
            handlers::system_handlers::check_for_updates,
            handlers::system_handlers::install_update,
            handlers::system_handlers::get_log_content,
            handlers::system_handlers::clear_log,
            handlers::system_handlers::send_log_report,
            handlers::system_handlers::get_changelogs,
            handlers::system_handlers::get_known_bugs,
            handlers::system_handlers::test_database_simple,
            
            // Data Manager handlers
            handlers::data_manager_handlers::initialize_application_data,
            handlers::data_manager_handlers::ensure_character_data_complete,
            handlers::data_manager_handlers::initialize_character_data,
            handlers::data_manager_handlers::update_reset_timestamps,
            handlers::data_manager_handlers::get_schema_version,
            handlers::data_manager_handlers::migrate_database,
            handlers::data_manager_handlers::get_app_bootstrap_snapshot,
            handlers::data_manager_handlers::get_dashboard_snapshot,
            
            // Roster handlers
            handlers::roster_handlers::scrape_roster,
            handlers::roster_handlers::get_rosters,
            handlers::roster_handlers::get_characters,
            handlers::roster_handlers::update_character_order,
            handlers::roster_handlers::update_character_roster_name,
            handlers::roster_handlers::sync_roster_data,
            handlers::roster_handlers::sync_roster_if_needed,
            handlers::roster_handlers::delete_roster,
            
            // Character handlers
            handlers::character_handlers::update_character_settings,
            handlers::character_handlers::update_character_earns_gold,
            handlers::character_handlers::get_character_rested_values,
            handlers::character_handlers::get_character_completion_status,
            handlers::character_handlers::get_character_raid_configs,
            handlers::character_handlers::get_character_tracking_status,
            handlers::character_handlers::get_character_details,
            handlers::character_handlers::get_dashboard_characters,
            handlers::character_handlers::test_character_query,
            
            // Tracking handlers
            handlers::tracking_handlers::get_tracking_config_matrix,
            handlers::tracking_handlers::update_tracking_config,
            handlers::tracking_handlers::save_tracking_config,
            handlers::tracking_handlers::save_rested_value,
            handlers::tracking_handlers::set_todo_tracked,
            
            // Raid handlers
            handlers::raid_handlers::get_game_raids,
            handlers::raid_handlers::get_character_raid_config,
            handlers::raid_handlers::get_raid_gate_matrix,
            handlers::raid_handlers::get_raid_matrix_data,
            handlers::raid_handlers::update_raid_master_config,
            handlers::raid_handlers::update_raid_gate_config,
            
            // Todo handlers
            handlers::todo_handlers::get_todo_matrix,
            handlers::todo_handlers::update_task_status,
            handlers::todo_handlers::update_roster_task_status,
            handlers::todo_handlers::update_raid_gate_status,
            handlers::todo_handlers::get_raid_gate_completed,
            handlers::todo_handlers::get_raid_gate_completions_bulk,

                        
    // Raid Todo handlers
    handlers::raid_todo_handlers::get_raid_configs_for_roster,
    
    // Encounter sync handlers
    handlers::encounter_sync_handlers::sync_encounters_to_completions,
    handlers::encounter_sync_handlers::get_encounters_preview,
    handlers::encounter_sync_handlers::test_boss_mapping,
    
    // Entity sync handlers
    handlers::entity_sync_handlers::sync_entity_data,
    handlers::entity_sync_handlers::sync_all_recent_entities,
    
    // Sync metadata handlers
    handlers::sync_metadata_handlers::perform_daily_roster_scraping,
    handlers::sync_metadata_handlers::get_roster_scrape_history,
    
    // Reset handlers
    handlers::reset_handlers::perform_manual_reset,
    handlers::reset_handlers::check_calendar_task_availability,
    handlers::reset_handlers::get_next_reset_time,
    handlers::reset_handlers::update_rested_values_now,
    handlers::reset_handlers::get_next_daily_reset_time,
    
    // Gold service handlers
    handlers::gold_service_handlers::update_raid_data_state,
    handlers::gold_service_handlers::trigger_gold_processing,
    handlers::gold_service_handlers::process_pending_gold_logs,
    handlers::gold_service_handlers::check_and_process_recent_completions,
    handlers::gold_service_handlers::get_weekly_gold_stats,
    handlers::gold_service_handlers::delete_gold_logs_for_raid,
    handlers::gold_service_handlers::clean_duplicate_gold_logs,
    handlers::gold_service_handlers::clean_all_duplicate_gold_logs,
            
    // Encounters watcher handlers
    sync::encounters_watcher::force_encounters_sync,
    sync::encounters_watcher::start_encounters_file_watcher,
    
    // Market handlers
    handlers::market_handlers::refresh_market_prices,
    handlers::market_handlers::get_all_market_prices,
    handlers::market_handlers::get_market_prices_by_category,
    handlers::market_handlers::get_market_price,
    handlers::market_handlers::set_manual_market_price,
    handlers::market_handlers::remove_manual_market_price,
    handlers::market_handlers::market_needs_refresh,
    handlers::market_handlers::get_gem_prices,
    handlers::market_handlers::get_price_history,
])
.run(tauri::generate_context!())
.unwrap_or_else(|e| {
    eprintln!("Fatal error while running LOA Tracker: {}", e);
    std::panic::panic_any(format!("Failed to start application: {}", e));
});
}
