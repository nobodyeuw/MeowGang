// Main application modules
pub mod app;
pub mod context;
pub mod database;
pub mod handlers;
pub mod init;
pub mod market;
pub mod models;
pub mod roster;
pub mod services;
pub mod settings;
pub mod sync;
pub mod validation;
pub mod version;

// Re-export commonly used items
pub use database::DatabaseManager;
pub use roster::HumanizedScraper;
pub use std::sync::Arc;

// Tauri application setup
use database::repositories::{
    CharacterRepository, ProgressionRepository, RaidRepository, RosterRepository, TrackingRepository,
};
use tauri::{
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

struct StartupResources {
    app_context: context::AppContext,
    settings_manager: settings::SettingsManager,
    db_manager: DatabaseManager,
    market_db: market::MarketDatabase,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    crate::log_info!("Starting LOA Tracker application");

    let single_instance_guard = match single_instance::claim() {
        single_instance::SingleInstanceGuard::Primary(guard) => Some(guard),
        single_instance::SingleInstanceGuard::Secondary => {
            crate::log_info!("Existing LOA Tracker instance detected; requesting it to show");
            return;
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(e) = window.hide() {
                    crate::log_error!("Failed to hide LOA Tracker to tray: {}", e);
                } else {
                    crate::log_info!("LOA Tracker hidden to tray");
                }
            }
        })
        .setup(|app| {
            crate::log_info!("Initializing Tauri application setup");
            if let Some(guard) = single_instance_guard {
                single_instance::listen_for_show_requests(app.handle().clone(), guard);
            }
            apply_custom_window_chrome(app.handle());

            let StartupResources {
                app_context,
                settings_manager,
                db_manager,
                market_db,
            } = initialize_startup_resources(app.handle(), app.package_info().version.to_string())?;

            setup_tray_icon(app.handle())?;

            let is_startup_monitor = std::env::args().any(|arg| arg == "--startup-monitor");
            apply_startup_settings(app.handle(), &settings_manager, is_startup_monitor);

            // Initialize logging system with Tauri path resolution
            crate::log_info!("Initializing logging system");
            services::logging_service::init_logging_with_app_handle(app.handle()).map_err(|e| {
                crate::log_error!("Failed to initialize logging: {}", e);
                format!("Failed to initialize logging: {}", e)
            })?;
            crate::log_info!("Logging system initialized successfully");

            let todo_repo_arc = manage_application_state(app, app_context, settings_manager, &db_manager, market_db);

            initialize_local_database_runtime(&db_manager)?;

            start_encounters_watcher(app.handle().clone(), todo_repo_arc.clone());
            start_daily_roster_scraping(app.handle().clone(), todo_repo_arc.clone());

            crate::log_info!("LOA Tracker setup completed successfully");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth handlers
            handlers::auth_handlers::authenticate_discord,
            handlers::auth_handlers::authenticate_supabase_discord,
            handlers::auth_handlers::verify_discord_profile_auth,
            handlers::auth_handlers::verify_stored_discord_auth,
            handlers::auth_handlers::get_discord_whitelist_members,
            // System handlers
            handlers::system_handlers::get_app_version,
            handlers::system_handlers::get_system_settings,
            handlers::system_handlers::set_show_setup_guide_button,
            handlers::system_handlers::set_show_auth_welcome,
            handlers::system_handlers::set_show_haals_hourglass_reminder,
            handlers::system_handlers::set_hide_on_launch,
            handlers::system_handlers::clear_user_data,
            handlers::system_handlers::set_encounters_db_path,
            handlers::system_handlers::set_lost_ark_exe_path,
            handlers::system_handlers::set_start_with_windows,
            handlers::system_handlers::set_start_with_lost_ark,
            handlers::system_handlers::set_loa_logs_exe_path,
            handlers::system_handlers::set_start_with_loa_logs,
            handlers::system_handlers::is_loa_logs_running,
            handlers::system_handlers::is_lost_ark_running,
            handlers::system_handlers::updates::check_for_updates,
            handlers::system_handlers::updates::install_update,
            handlers::system_handlers::logs::get_log_content,
            handlers::system_handlers::logs::clear_log,
            handlers::system_handlers::logs::write_frontend_log,
            handlers::system_handlers::logs::send_log_report,
            handlers::system_handlers::resources::get_changelogs,
            handlers::system_handlers::resources::get_known_bugs,
            // Data Manager handlers
            handlers::data_manager_handlers::initialize_application_data,
            handlers::data_manager_handlers::ensure_character_data_complete,
            handlers::data_manager_handlers::initialize_character_data,
            handlers::data_manager_handlers::update_reset_timestamps,
            handlers::data_manager_handlers::get_schema_version,
            handlers::data_manager_handlers::migrate_database,
            handlers::data_manager_handlers::get_app_bootstrap_snapshot,
            handlers::data_manager_handlers::get_dashboard_snapshot,
            handlers::dashboard_calendar_handlers::get_dashboard_calendar_assignments,
            handlers::dashboard_calendar_handlers::save_dashboard_calendar_assignment,
            handlers::dashboard_calendar_handlers::clear_dashboard_calendar_assignment,
            handlers::dashboard_calendar_handlers::get_dashboard_raid_reservations,
            handlers::dashboard_calendar_handlers::save_dashboard_raid_reservation,
            handlers::dashboard_calendar_handlers::clear_dashboard_raid_reservation,
            handlers::dashboard_calendar_handlers::cleanup_dashboard_raid_reservations,
            // Roster handlers
            handlers::roster_handlers::scrape_roster,
            handlers::roster_handlers::get_rosters,
            handlers::roster_handlers::get_characters,
            handlers::roster_handlers::update_character_order,
            handlers::roster_handlers::update_roster_order,
            handlers::roster_handlers::update_character_roster_name,
            handlers::roster_handlers::update_roster_name,
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
            handlers::character_handlers::scrape_character_details,
            // Tracking handlers
            handlers::tracking_handlers::get_tracking_config_matrix,
            handlers::tracking_handlers::update_tracking_config,
            handlers::tracking_handlers::update_lazy_daily_config,
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
            handlers::todo_handlers::get_roster_event_progress,
            handlers::todo_handlers::update_task_status,
            handlers::todo_handlers::update_roster_event_status,
            handlers::todo_handlers::update_roster_event_weekly_count,
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
            // Temporarily disabled due to Supabase realtime message limits
            // // MeowConnect handlers
            // handlers::meow_connect_handlers::get_meow_connect_local_snapshot,
            // handlers::meow_connect_handlers::apply_meow_connect_clear_hints,
            // handlers::meow_connect_handlers::replace_meow_connect_group_raid_tags,
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
            handlers::market_handlers::reset_manual_market_price_to_estimate,
            handlers::market_handlers::set_market_favorite,
            handlers::market_handlers::market_needs_refresh,
            handlers::market_handlers::get_gem_prices,
            handlers::market_handlers::get_accessory_prices,
            handlers::market_handlers::get_price_history,
            // Progression planner (character detail storage; scraper fills via save_scraped_character_progression)
            handlers::progression_handlers::get_character_progression_snapshot,
            handlers::progression_handlers::save_scraped_character_progression,
            handlers::progression_handlers::upsert_progression_goal,
            handlers::progression_handlers::delete_progression_goal,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("Fatal error while running LOA Tracker: {}", e);
            std::panic::panic_any(format!("Failed to start application: {}", e));
        });
}

#[cfg(target_os = "windows")]
mod single_instance {
    use std::ptr::null_mut;
    use std::sync::OnceLock;
    use tauri::AppHandle;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
    use windows_sys::Win32::System::Threading::{
        CreateEventW, CreateMutexW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE, INFINITE,
    };

    const MUTEX_NAME: &str = "Local\\LOA_Tracker_Single_Instance";
    const SHOW_EVENT_NAME: &str = "Local\\LOA_Tracker_Show_Main_Window";

    static INSTANCE_MUTEX: OnceLock<usize> = OnceLock::new();

    pub enum SingleInstanceGuard {
        Primary(PrimaryInstanceGuard),
        Secondary,
    }

    pub struct PrimaryInstanceGuard {
        show_event: usize,
    }

    pub fn claim() -> SingleInstanceGuard {
        let mutex_name = wide_null(MUTEX_NAME);
        let mutex = unsafe { CreateMutexW(null_mut(), 1, mutex_name.as_ptr()) };
        if mutex.is_null() {
            crate::log_warn!("Failed to create single-instance mutex");
            return SingleInstanceGuard::Primary(create_guard());
        }

        let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        if already_exists {
            unsafe {
                CloseHandle(mutex);
            }
            signal_primary_instance();
            SingleInstanceGuard::Secondary
        } else {
            let _ = INSTANCE_MUTEX.set(mutex as usize);
            SingleInstanceGuard::Primary(create_guard())
        }
    }

    pub fn listen_for_show_requests(app: AppHandle, guard: PrimaryInstanceGuard) {
        if guard.show_event == 0 {
            return;
        }

        std::thread::spawn(move || loop {
            let result = unsafe { WaitForSingleObject(guard.show_event as HANDLE, INFINITE) };
            if result == 0 {
                crate::handlers::system_handlers::reveal_main_window(&app);
            } else {
                crate::log_warn!("Single-instance show listener stopped unexpectedly: {}", result);
                break;
            }
        });
    }

    fn create_guard() -> PrimaryInstanceGuard {
        let event_name = wide_null(SHOW_EVENT_NAME);
        let show_event = unsafe { CreateEventW(null_mut(), 0, 0, event_name.as_ptr()) };
        if show_event.is_null() {
            crate::log_warn!("Failed to create single-instance show event");
        }
        PrimaryInstanceGuard {
            show_event: show_event as usize,
        }
    }

    fn signal_primary_instance() {
        let event_name = wide_null(SHOW_EVENT_NAME);
        let show_event = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, event_name.as_ptr()) };
        if show_event.is_null() {
            crate::log_warn!("Existing instance was detected, but show event could not be opened");
            return;
        }

        unsafe {
            SetEvent(show_event);
            CloseHandle(show_event);
        }
    }

    fn wide_null(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(not(target_os = "windows"))]
mod single_instance {
    use tauri::AppHandle;

    pub enum SingleInstanceGuard {
        Primary(PrimaryInstanceGuard),
        Secondary,
    }

    pub struct PrimaryInstanceGuard;

    pub fn claim() -> SingleInstanceGuard {
        SingleInstanceGuard::Primary(PrimaryInstanceGuard)
    }

    pub fn listen_for_show_requests(_app: AppHandle, _guard: PrimaryInstanceGuard) {}
}

fn setup_tray_icon(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let menu = MenuBuilder::new(app)
        .text("show", "Show LOA Tracker")
        .separator()
        .text("quit", "Quit")
        .build()?;

    let mut tray_builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("LOA Tracker")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().0.as_str() {
            "show" => reveal_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                reveal_main_window(&tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    tray_builder.build(app)?;
    Ok(())
}

/// Enforces the custom Svelte titlebar even when dev runs reuse stale window state.
fn apply_custom_window_chrome(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        crate::log_warn!("Main window was not found while applying custom window chrome");
        return;
    };

    if let Err(e) = window.set_decorations(false) {
        crate::log_warn!("Failed to disable native window decorations: {}", e);
    }
}

/// Creates the filesystem, database, settings, and market resources needed by setup.
fn initialize_startup_resources(
    app: &tauri::AppHandle,
    version: String,
) -> Result<StartupResources, Box<dyn std::error::Error>> {
    let mut app_context = context::AppContext::new(version)?;
    app_context.update_paths_with_tauri(app)?;
    app_context.ensure_directories()?;

    handlers::auth_handlers::migrate_legacy_roaming_files(app);

    let db_path = init::ensure_database_setup(&app_context.app_data_dir)?;
    crate::log_info!("Database path established: {:?}", db_path);

    let market_db_path = app_context.app_data_dir.join("market.db");
    let market_db_path_str = market_db_path.to_str().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Market DB path contains invalid UTF-8: {:?}", market_db_path),
        )
    })?;
    let market_db = market::MarketDatabase::new(market_db_path_str).map_err(|e| {
        crate::log_error!("Failed to initialize market database: {}", e);
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to initialize market database: {}", e),
        )
    })?;

    // Manual-only estimate rows support progression planning inputs that the market API cannot fetch.
    market_db.seed_gem_entries().map_err(|e| {
        crate::log_error!("Failed to seed gem entries: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to seed gem entries: {}", e))
    })?;
    market_db.seed_accessory_entries().map_err(|e| {
        crate::log_error!("Failed to seed accessory entries: {}", e);
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to seed accessory entries: {}", e),
        )
    })?;

    let db_path_str = db_path.to_str().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Database path contains invalid UTF-8: {:?}", db_path),
        )
    })?;
    let db_manager = DatabaseManager::new(db_path_str).map_err(|e| {
        crate::log_error!("Failed to initialize database: {}", e);
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to initialize database: {}", e),
        )
    })?;

    let settings_manager = settings::SettingsManager::new(app_context.settings_path.clone()).map_err(|e| {
        crate::log_error!("Failed to create settings manager: {}", e);
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create settings manager: {}", e),
        )
    })?;

    settings_manager.ensure_exists().map_err(|e| {
        crate::log_error!("Failed to ensure settings exist: {}", e);
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to ensure settings exist: {}", e),
        )
    })?;
    crate::log_info!("Settings manager initialized successfully");

    Ok(StartupResources {
        app_context,
        settings_manager,
        db_manager,
        market_db,
    })
}

/// Applies persisted launch behavior once the tray and settings manager exist.
///
/// This keeps startup side effects together: autostart registration, optional
/// hidden launch, and companion process monitoring.
fn apply_startup_settings(
    app: &tauri::AppHandle,
    settings_manager: &settings::SettingsManager,
    is_startup_monitor: bool,
) {
    let Ok(Some(settings)) = settings_manager.read() else {
        return;
    };

    if let Err(e) = crate::handlers::system_handlers::refresh_startup_registration(&settings) {
        crate::log_error!("Failed to refresh startup registration: {}", e);
    }

    if settings.system.hide_on_launch || (is_startup_monitor && !settings.system.start_with_windows) {
        if let Some(window) = app.get_webview_window("main") {
            if let Err(e) = window.hide() {
                crate::log_error!("Failed to hide launch window: {}", e);
            } else {
                crate::log_info!("LOA Tracker started hidden in tray");
            }
        }
    }

    if settings.system.start_with_lost_ark {
        match crate::handlers::system_handlers::set_lost_ark_monitoring(true, app.clone()) {
            Ok(_) => crate::log_info!("Lost Ark monitoring started on startup"),
            Err(e) => crate::log_error!("Failed to start Lost Ark monitoring on startup: {}", e),
        }
    }

    if settings.system.start_with_loa_logs {
        let mut startup_settings = settings.clone();
        if startup_settings.system.loa_logs_exe_path.is_none() {
            if let Some(path) = crate::handlers::system_handlers::detect_loa_logs_exe_path() {
                startup_settings.system.loa_logs_exe_path = Some(path);
                if let Err(e) = settings_manager.save(&startup_settings) {
                    crate::log_warn!("Failed to save auto-detected LOA Logs path: {}", e);
                }
            }
        }

        crate::log_warn!(
            "LOA Logs auto-launch is temporarily disabled; LOA Tracker will only monitor for LOA Logs startup"
        );

        match crate::handlers::system_handlers::set_loa_logs_monitoring(true, app.clone()) {
            Ok(_) => crate::log_info!("LOA Logs companion monitoring started on startup"),
            Err(e) => crate::log_error!("Failed to start LOA Logs companion monitoring on startup: {}", e),
        }
    }
}

/// Registers long-lived Tauri state used by command handlers.
///
/// Database migrations still run in setup after this returns; the managed
/// `DatabaseManager` is a clone around the same pool, matching the previous
/// inline setup behavior.
fn manage_application_state(
    app: &mut tauri::App,
    app_context: context::AppContext,
    settings_manager: settings::SettingsManager,
    db_manager: &DatabaseManager,
    market_db: market::MarketDatabase,
) -> Arc<database::repositories::TodoRepository> {
    crate::log_debug!("Initializing database repositories");
    let roster_repo = RosterRepository::new(db_manager.pool.clone());
    let tracking_repo = TrackingRepository::new(db_manager.pool.clone());
    let raid_repo = RaidRepository::new(db_manager.pool.clone());
    let character_repo = CharacterRepository::new(db_manager.pool.clone());
    let progression_repo = ProgressionRepository::new(db_manager.pool.clone());
    crate::log_info!("All repositories initialized successfully");

    // The managed scraper exists for compatibility with older command
    // signatures. Real roster/character values are set per request.
    let scraper = roster::HumanizedScraper::new(String::new(), String::new());
    crate::log_debug!("Roster scraper initialized");

    let todo_repo = database::repositories::TodoRepository::new(Arc::new(db_manager.pool.clone()));
    let todo_repo_arc = Arc::new(todo_repo);

    app.manage(app_context);
    app.manage(settings_manager);
    app.manage(db_manager.clone());
    app.manage(db_manager.pool.clone());
    app.manage(roster_repo);
    app.manage(tracking_repo);
    app.manage(raid_repo);
    app.manage(character_repo);
    app.manage(progression_repo);
    app.manage(scraper);
    app.manage(market_db);
    app.manage(todo_repo_arc.clone());

    todo_repo_arc
}

/// Runs local database bootstrap that must happen before background services.
///
/// Frontend-provided game data is loaded later through commands; the empty
/// default-data call keeps legacy local tables present for reset/migration
/// paths during startup.
fn initialize_local_database_runtime(db_manager: &DatabaseManager) -> Result<(), Box<dyn std::error::Error>> {
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

    let current_version = database::data_manager::DataManager::get_schema_version(&db_manager.pool).unwrap_or(1);
    const TARGET_VERSION: i32 = 21;
    crate::log_info!(
        "Current schema version: {}, target version: {}",
        current_version,
        TARGET_VERSION
    );

    if current_version < TARGET_VERSION {
        crate::log_info!(
            "Starting database migration from version {} to {}",
            current_version,
            TARGET_VERSION
        );
        database::data_manager::DataManager::migrate_database(&db_manager.pool, current_version, TARGET_VERSION)?;
        crate::log_info!("Database migration completed successfully");
    }

    crate::log_debug!("Updating reset timestamps");
    database::data_manager::DataManager::update_reset_timestamps(&db_manager.pool)?;
    crate::log_info!("Reset timestamps updated");

    crate::log_info!("Starting scheduled daily reset service");
    crate::services::reset_service::ResetService::start_scheduled_reset(db_manager.pool.clone());
    crate::log_info!("Daily reset service started");

    Ok(())
}

fn reveal_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(e) = window.show() {
            crate::log_error!("Failed to show LOA Tracker window from tray: {}", e);
        }
        if let Err(e) = window.unminimize() {
            crate::log_debug!("Failed to unminimize LOA Tracker window from tray: {}", e);
        }
        if let Err(e) = window.set_focus() {
            crate::log_debug!("Failed to focus LOA Tracker window from tray: {}", e);
        }
    } else {
        crate::log_warn!("Main LOA Tracker window was not found from tray");
    }
}

/// Starts the delayed encounters.db watcher after the Tauri app has finished setup.
///
/// The delay avoids racing the app window/state registration during startup.
fn start_encounters_watcher(app: tauri::AppHandle, todo_repo: Arc<database::repositories::TodoRepository>) {
    crate::log_info!("Starting encounters file watcher initialization");

    tauri::async_runtime::spawn(async move {
        crate::log_debug!("Encounters watcher: waiting for app initialization");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let encounters_db_path =
            if let Some(detected_path) = crate::handlers::system_handlers::detect_encounters_db_path() {
                crate::log_info!("Auto-detected encounters.db path: {}", detected_path);
                detected_path
            } else {
                crate::log_info!("Using default encounters.db path, user will need to configure it");
                "encounters.db".to_string()
            };

        let watcher =
            crate::sync::encounters_watcher::EncountersFileWatcher::new(app, todo_repo.clone(), encounters_db_path);

        if let Err(e) = watcher.start_watching() {
            crate::log_error!("Failed to start encounters file watcher: {}", e);
        } else {
            crate::log_info!("Encounters file watcher started successfully");
        }
    });
}

/// Runs the startup daily roster scrape job in the background.
///
/// This shares the same scrape/upsert helper as the manual command path so
/// newly discovered roster characters and refreshed stats behave consistently.
fn start_daily_roster_scraping(app: tauri::AppHandle, todo_repo: Arc<database::repositories::TodoRepository>) {
    crate::log_info!("Initializing daily roster scraping service");

    tauri::async_runtime::spawn(async move {
        crate::log_debug!("Starting daily roster scraping task");

        use crate::handlers::sync_metadata_handlers::{
            get_rosters_needing_daily_scrape, scrape_roster_for_updates, update_roster_scrape_metadata,
        };

        if let Ok(rosters_needing_scrape) = get_rosters_needing_daily_scrape(&*todo_repo) {
            crate::log_info!("Found {} rosters needing daily scrape", rosters_needing_scrape.len());
            let mut successful_scrapes = 0;
            let mut updated_characters = 0;

            for roster_id in rosters_needing_scrape {
                if let Err(e) = update_roster_scrape_metadata(&*todo_repo, &roster_id, "started", None) {
                    crate::log_error!("Failed to mark roster {} scrape as started: {}", roster_id, e);
                }

                match scrape_roster_for_updates(&*todo_repo, &roster_id).await {
                    Ok(updated_count) => {
                        if let Err(e) = update_roster_scrape_metadata(
                            &*todo_repo,
                            &roster_id,
                            "completed",
                            Some(&format!("Updated {} characters", updated_count)),
                        ) {
                            crate::log_error!("Failed to update metadata for roster {}: {}", roster_id, e);
                        }

                        crate::log_info!(
                            "Successfully scraped roster {}: {} characters updated",
                            roster_id,
                            updated_count
                        );
                        successful_scrapes += 1;
                        updated_characters += updated_count;
                    }
                    Err(e) => {
                        crate::log_error!("Failed to scrape roster {}: {}", roster_id, e);

                        if let Err(e2) =
                            update_roster_scrape_metadata(&*todo_repo, &roster_id, "failed", Some(&e.to_string()))
                        {
                            crate::log_error!("Failed to update failure metadata for roster {}: {}", roster_id, e2);
                        }
                    }
                }
            }

            if successful_scrapes > 0 {
                let _ = app.emit(
                    "meow-connect-roster-scrape-complete",
                    serde_json::json!({
                        "successful_scrapes": successful_scrapes,
                        "updated_characters": updated_characters
                    }),
                );
            }
        }
    });
}
