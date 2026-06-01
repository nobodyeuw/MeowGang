use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Transaction;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sysinfo::System;
use tauri::{AppHandle, Manager, State};

pub(crate) mod logs;
pub(crate) mod resources;
pub(crate) mod updates;

pub use logs::{clear_log, get_log_content, send_log_report, write_frontend_log};
pub use resources::{get_changelogs, get_known_bugs};
pub use updates::{check_for_updates, install_update, UpdateInfo};

const STARTUP_TASK_NAME: &str = "LOA_Tracker_Auto_Start";
const STARTUP_REGISTRY_VALUE_NAME: &str = "LOA Tracker";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettings {
    pub encounters_db_path: Option<String>,
    pub lost_ark_exe_path: Option<String>,
    pub loa_logs_exe_path: Option<String>,
    pub start_with_windows: bool,
    pub start_with_lost_ark: bool,
    pub hide_on_launch: bool,
    pub show_setup_guide_button: bool,
    pub show_auth_welcome: bool,
}

#[tauri::command]
/// Returns the build/package version shown in the UI.
pub async fn get_app_version() -> Result<String, String> {
    Ok(crate::version::APP_VERSION.to_string())
}

#[tauri::command]
/// Loads system settings and lazily auto-detects missing external paths.
pub async fn get_system_settings(
    settings_manager: State<'_, crate::settings::SettingsManager>,
) -> Result<crate::settings::SystemSettings, String> {
    let mut settings = settings_manager
        .read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());

    // Auto-detect paths if not set
    let encounters_db_path = if settings.system.encounters_db_path.is_none() {
        if let Some(path) = detect_encounters_db_path() {
            // Save auto-detected path
            settings.system.encounters_db_path = Some(path.clone());
            let _ = settings_manager.save(&settings);
            Some(path)
        } else {
            None
        }
    } else {
        settings.system.encounters_db_path.clone()
    };

    let lost_ark_exe_path = if settings.system.lost_ark_exe_path.is_none() {
        if let Some(path) = detect_lost_ark_exe_path() {
            // Save auto-detected path
            settings.system.lost_ark_exe_path = Some(path.clone());
            let _ = settings_manager.save(&settings);
            Some(path)
        } else {
            None
        }
    } else {
        settings.system.lost_ark_exe_path.clone()
    };

    let loa_logs_exe_path = if settings.system.loa_logs_exe_path.is_none() {
        if let Some(path) = detect_loa_logs_exe_path() {
            settings.system.loa_logs_exe_path = Some(path.clone());
            let _ = settings_manager.save(&settings);
            Some(path)
        } else {
            None
        }
    } else {
        settings.system.loa_logs_exe_path.clone()
    };

    let start_with_loa_logs = settings.system.start_with_loa_logs;

    Ok(crate::settings::SystemSettings {
        encounters_db_path,
        lost_ark_exe_path,
        loa_logs_exe_path,
        start_with_windows: settings.system.start_with_windows,
        start_with_lost_ark: settings.system.start_with_lost_ark,
        start_with_loa_logs,
        hide_on_launch: settings.system.hide_on_launch,
        show_setup_guide_button: settings.system.show_setup_guide_button,
        show_auth_welcome: settings.system.show_auth_welcome,
        show_haals_hourglass_reminder: settings.system.show_haals_hourglass_reminder,
        extra: settings.system.extra,
    })
}

/// Loads settings, applies one mutation, and saves the result.
fn update_system_settings(
    settings_manager: &crate::settings::SettingsManager,
    update: impl FnOnce(&mut crate::settings::Settings),
) -> Result<(), String> {
    let mut settings = settings_manager
        .read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());

    update(&mut settings);

    settings_manager
        .save(&settings)
        .map_err(|e| format!("Failed to save settings: {}", e))
}

#[tauri::command]
/// Toggles the setup guide shortcut in the app header.
pub async fn set_show_setup_guide_button(
    enabled: bool,
    settings_manager: State<'_, crate::settings::SettingsManager>,
) -> Result<(), String> {
    update_system_settings(&settings_manager, |settings| {
        settings.system.show_setup_guide_button = enabled;
    })?;
    crate::log_info!("Set setup guide header button visibility to: {}", enabled);
    Ok(())
}

#[tauri::command]
/// Toggles whether the auth welcome screen is shown on startup.
pub async fn set_show_auth_welcome(
    enabled: bool,
    settings_manager: State<'_, crate::settings::SettingsManager>,
) -> Result<(), String> {
    update_system_settings(&settings_manager, |settings| {
        settings.system.show_auth_welcome = enabled;
    })?;
    crate::log_info!("Set Discord welcome screen visibility to: {}", enabled);
    Ok(())
}

#[tauri::command]
/// Toggles the Tuesday reminder for 1730+ Cube characters before weekly reset.
pub async fn set_show_haals_hourglass_reminder(
    enabled: bool,
    settings_manager: State<'_, crate::settings::SettingsManager>,
) -> Result<(), String> {
    update_system_settings(&settings_manager, |settings| {
        settings.system.show_haals_hourglass_reminder = enabled;
    })?;
    crate::log_info!("Set Haal's Hourglass reminder visibility to: {}", enabled);
    Ok(())
}

#[tauri::command]
/// Toggles whether the main window hides to tray on launch.
pub async fn set_hide_on_launch(
    enabled: bool,
    settings_manager: State<'_, crate::settings::SettingsManager>,
) -> Result<(), String> {
    update_system_settings(&settings_manager, |settings| {
        settings.system.hide_on_launch = enabled;
    })?;
    crate::log_info!("Set hide on launch to: {}", enabled);
    Ok(())
}

#[tauri::command]
/// Clears local roster/character/task data while keeping app settings.
pub async fn clear_user_data(pool: State<'_, Pool<SqliteConnectionManager>>) -> Result<String, String> {
    let mut conn = pool.get().map_err(|e| format!("Failed to open user database: {}", e))?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start clear user data transaction: {}", e))?;

    let deleted_characters = clear_character_data(&tx).map_err(|e| format!("Failed to clear character data: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit clear user data transaction: {}", e))?;

    crate::log_info!("Cleared user data for {} characters", deleted_characters);
    Ok(format!("Cleared {} characters and related data", deleted_characters))
}

/// Deletes character-owned rows in one transaction for the clear-data command.
fn clear_character_data(tx: &Transaction<'_>) -> rusqlite::Result<usize> {
    let deleted_characters: usize = tx.query_row("SELECT COUNT(*) FROM conf_character", [], |row| row.get(0))?;

    for table in ["conf_tracking", "conf_raid", "completion_status", "rested_values"] {
        tx.execute(&format!("DELETE FROM {}", table), [])?;
    }

    for table in [
        "character_engravings",
        "character_equipment",
        "character_gems",
        "progression_goals",
    ] {
        tx.execute(&format!("DELETE FROM {}", table), [])?;
    }

    tx.execute("DELETE FROM sync_metadata WHERE table_name = 'conf_character'", [])?;
    tx.execute("DELETE FROM conf_character", [])?;

    Ok(deleted_characters)
}

/// Attempts to locate LOA Logs' encounters.db from common install/runtime paths.
pub fn detect_encounters_db_path() -> Option<String> {
    // Check LOA Logs directory first (used by LOA Logs app)
    if let Some(local_data_dir) = dirs::data_local_dir() {
        let loa_logs_path = local_data_dir.join("LOA Logs").join("encounters.db");
        if loa_logs_path.exists() {
            if let Some(path_str) = loa_logs_path.to_str() {
                crate::log_info!("Auto-detected encounters.db in LOA Logs: {}", path_str);
                return Some(path_str.to_string());
            }
        }
    }

    // Use current directory like reference project
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent_dir) = current_exe.parent() {
            let encounters_path = parent_dir.join("encounters.db");

            if encounters_path.exists() {
                if let Some(path_str) = encounters_path.to_str() {
                    crate::log_info!("Auto-detected encounters.db at: {}", path_str);
                    return Some(path_str.to_string());
                }
            }
        }
    }

    // Fallback to current working directory
    if let Ok(current_dir) = std::env::current_dir() {
        let fallback_path = current_dir.join("encounters.db");
        if fallback_path.exists() {
            if let Some(path_str) = fallback_path.to_str() {
                crate::log_info!("Auto-detected encounters.db at: {}", path_str);
                return Some(path_str.to_string());
            }
        }
    }

    None
}

fn detect_lost_ark_exe_path() -> Option<String> {
    // Try common Steam installation paths
    let steam_paths = vec![
        // D:\steam\steamapps\common\Lost Ark\Binaries\Win64
        Some(PathBuf::from(
            "D:\\steam\\steamapps\\common\\Lost Ark\\Binaries\\Win64\\LOSTARK.exe",
        )),
        // Program Files (x86)
        std::env::var("PROGRAMFILES(X86)").ok().map(|p| {
            PathBuf::from(&p)
                .join("Steam")
                .join("steamapps")
                .join("common")
                .join("Lost Ark")
                .join("Binaries")
                .join("Win64")
                .join("LOSTARK.exe")
        }),
        // Program Files
        std::env::var("PROGRAMFILES").ok().map(|p| {
            PathBuf::from(&p)
                .join("Steam")
                .join("steamapps")
                .join("common")
                .join("Lost Ark")
                .join("Binaries")
                .join("Win64")
                .join("LOSTARK.exe")
        }),
    ];

    for path_option in steam_paths {
        if let Some(path) = path_option {
            if path.exists() {
                if let Some(path_str) = path.to_str() {
                    crate::log_info!("Auto-detected LostArk.exe at: {}", path_str);
                    return Some(path_str.to_string());
                }
            }
        }
    }

    // Try user's specific Steam path from environment
    if let Ok(steam_path) = std::env::var("STEAM_PATH") {
        let lostark_path = PathBuf::from(&steam_path)
            .join("steamapps")
            .join("common")
            .join("Lost Ark")
            .join("Binaries")
            .join("Win64")
            .join("LOSTARK.exe");

        if lostark_path.exists() {
            if let Some(path_str) = lostark_path.to_str() {
                crate::log_info!("Auto-detected LostArk.exe from STEAM_PATH: {}", path_str);
                return Some(path_str.to_string());
            }
        }
    }

    None
}

pub(crate) fn detect_loa_logs_exe_path() -> Option<String> {
    // Common LOA Logs locations: Local AppData LOA Logs folder, Program Files
    if let Some(local_data_dir) = dirs::data_local_dir() {
        let candidates = [
            local_data_dir.join("Programs").join("LOA Logs").join("LOA Logs.exe"),
            local_data_dir.join("LOA Logs").join("LOA Logs.exe"),
        ];

        for candidate in candidates {
            if candidate.exists() {
                if let Some(path_str) = candidate.to_str() {
                    crate::log_info!("Auto-detected LOA Logs exe in LocalAppData: {}", path_str);
                    return Some(path_str.to_string());
                }
            }
        }
    }

    // Check Program Files locations
    if let Ok(program_files) = std::env::var("PROGRAMFILES") {
        let candidate = PathBuf::from(&program_files).join("LOA Logs").join("LOA Logs.exe");
        if candidate.exists() {
            if let Some(path_str) = candidate.to_str() {
                crate::log_info!("Auto-detected LOA Logs exe in ProgramFiles: {}", path_str);
                return Some(path_str.to_string());
            }
        }
    }

    if let Ok(program_files_x86) = std::env::var("PROGRAMFILES(X86)") {
        let candidate = PathBuf::from(&program_files_x86).join("LOA Logs").join("LOA Logs.exe");
        if candidate.exists() {
            if let Some(path_str) = candidate.to_str() {
                crate::log_info!("Auto-detected LOA Logs exe in ProgramFiles(x86): {}", path_str);
                return Some(path_str.to_string());
            }
        }
    }

    None
}

/// Validates a user-selected file path before saving it into settings.
fn validate_existing_file_extension(path: &str, expected_extension: &str) -> Result<(), String> {
    let path_obj = std::path::Path::new(path);
    if !path_obj.exists() {
        return Err("File not found".to_string());
    }

    if !path_obj
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected_extension))
    {
        return Err(format!("Invalid file extension. Expected .{} file", expected_extension));
    }

    Ok(())
}

#[tauri::command]
pub async fn set_encounters_db_path(
    settings_manager: State<'_, crate::settings::SettingsManager>,
    path: String,
) -> Result<(), String> {
    validate_existing_file_extension(&path, "db")?;

    let mut settings = settings_manager
        .read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());

    settings.system.encounters_db_path = Some(path);
    settings_manager
        .save(&settings)
        .map_err(|e| format!("Failed to save settings: {}", e))
}

#[tauri::command]
pub async fn set_lost_ark_exe_path(
    settings_manager: State<'_, crate::settings::SettingsManager>,
    path: String,
) -> Result<(), String> {
    validate_existing_file_extension(&path, "exe")?;

    let mut settings = settings_manager
        .read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());

    settings.system.lost_ark_exe_path = Some(path);
    settings_manager
        .save(&settings)
        .map_err(|e| format!("Failed to save settings: {}", e))
}

#[tauri::command]
pub async fn set_loa_logs_exe_path(
    settings_manager: State<'_, crate::settings::SettingsManager>,
    path: String,
) -> Result<(), String> {
    validate_existing_file_extension(&path, "exe")?;

    let mut settings = settings_manager
        .read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());

    settings.system.loa_logs_exe_path = Some(path);
    settings_manager
        .save(&settings)
        .map_err(|e| format!("Failed to save settings: {}", e))
}

#[tauri::command]
pub async fn set_start_with_loa_logs(
    settings_manager: State<'_, crate::settings::SettingsManager>,
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    let mut settings = settings_manager
        .read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());

    settings.system.start_with_loa_logs = enabled;
    settings_manager
        .save(&settings)
        .map_err(|e| format!("Failed to save settings: {}", e))?;
    refresh_startup_registration(&settings).map_err(|e| format!("Failed to update startup registration: {}", e))?;

    set_loa_logs_monitoring(enabled, app)?;

    if enabled {
        crate::log_warn!(
            "LOA Logs auto-launch is temporarily disabled; LOA Tracker will only monitor for LOA Logs startup"
        );
    }

    crate::log_info!("Successfully set LOA Logs monitoring to: {}", enabled);
    Ok(())
}

#[tauri::command]
pub async fn is_loa_logs_running() -> Result<bool, String> {
    let mut system = System::new_all();
    system.refresh_processes();

    for process in system.processes().values() {
        let name = process.name().to_lowercase();
        if name == "loa logs.exe" || name == "loa_logs.exe" || name.contains("loa") && name.contains("logs") {
            return Ok(true);
        }
    }

    Ok(false)
}

#[tauri::command]
pub async fn set_start_with_windows(
    settings_manager: State<'_, crate::settings::SettingsManager>,
    enabled: bool,
) -> Result<(), String> {
    let mut settings = settings_manager
        .read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());

    settings.system.start_with_windows = enabled;
    settings_manager
        .save(&settings)
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    match refresh_startup_registration(&settings) {
        Ok(_) => {
            crate::log_info!("Successfully set start with Windows to: {}", enabled);
        }
        Err(e) => {
            crate::log_error!("Failed to set Task Scheduler autostart: {}", e);
            return Err(format!("Failed to set Windows autostart: {}", e));
        }
    }

    Ok(())
}

pub(crate) fn refresh_startup_registration(
    settings: &crate::settings::Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    let enabled = settings.system.start_with_windows || settings.system.start_with_lost_ark;
    let background_monitor = !settings.system.start_with_windows && settings.system.start_with_lost_ark;

    refresh_startup_registration_impl(enabled, background_monitor)
}

#[cfg(target_os = "windows")]
fn refresh_startup_registration_impl(
    enabled: bool,
    background_monitor: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    std::thread::spawn(move || match set_autostart_task(enabled, background_monitor) {
        Ok(_) => {
            if enabled {
                if let Err(e) = set_autostart_registry(false, background_monitor) {
                    crate::log_debug!("Failed to remove fallback registry autostart: {}", e);
                }
            } else if let Err(e) = set_autostart_registry(false, background_monitor) {
                crate::log_debug!("Failed to remove fallback registry autostart: {}", e);
            }
            Ok(())
        }
        Err(task_error) if enabled => {
            crate::log_warn!(
                "Task Scheduler autostart failed, falling back to current-user registry startup: {}",
                task_error
            );
            set_autostart_registry(true, background_monitor).map_err(|registry_error| {
                format!(
                    "Failed to update Task Scheduler startup task: {}; fallback registry startup also failed: {}",
                    task_error, registry_error
                )
            })
        }
        Err(task_error) => {
            crate::log_debug!("Task Scheduler autostart removal failed: {}", task_error);
            set_autostart_registry(false, background_monitor).map_err(|registry_error| {
                format!(
                    "Failed to remove Task Scheduler startup task: {}; fallback registry startup also failed: {}",
                    task_error, registry_error
                )
            })
        }
    })
    .join()
    .map_err(|_| "Task Scheduler startup registration thread panicked".to_string())??;

    Ok(())
}

#[cfg(target_os = "windows")]
fn set_autostart_registry(enabled: bool, background_monitor: bool) -> Result<(), Box<dyn std::error::Error>> {
    use winreg::RegKey;

    let exe_path = std::env::current_exe()?;
    let exe_path_str = exe_path.to_string_lossy();
    let startup_command = if background_monitor {
        format!("\"{}\" --startup-monitor", exe_path_str)
    } else {
        format!("\"{}\"", exe_path_str)
    };

    let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let run_key = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        winreg::enums::KEY_WRITE,
    )?;

    if enabled {
        run_key.set_value(STARTUP_REGISTRY_VALUE_NAME, &startup_command)?;
        crate::log_info!(
            "Registered LOA Tracker current-user registry autostart fallback: {}",
            startup_command
        );
    } else {
        match run_key.delete_value(STARTUP_REGISTRY_VALUE_NAME) {
            Ok(_) => crate::log_info!("Removed LOA Tracker current-user registry autostart fallback"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                crate::log_debug!("LOA Tracker registry autostart fallback was not present");
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn set_autostart_task(enabled: bool, background_monitor: bool) -> Result<(), Box<dyn std::error::Error>> {
    use winsafe::{self as w, co, prelude::*};

    let _scope = w::CoInitializeEx(co::COINIT::MULTITHREADED)?;
    let service = w::CoCreateInstance::<w::ITaskService>(
        &co::CLSID::TaskScheduler,
        None::<&w::IUnknown>,
        co::CLSCTX::INPROC_SERVER,
    )?;
    service.Connect(None, None, None, None)?;
    let folder = service.GetFolder(r"\")?;

    if !enabled {
        match folder.DeleteTask(STARTUP_TASK_NAME) {
            Ok(_) => crate::log_info!("Removed LOA Tracker Task Scheduler autostart task"),
            Err(e) => crate::log_debug!("LOA Tracker autostart task was not removed: {}", e),
        }
        return Ok(());
    }

    let exe_path = std::env::current_exe()?;
    let arguments = if background_monitor { "--startup-monitor" } else { "" };

    let task = service.NewTask()?;
    task.get_Triggers()?.Create(co::TASK_TRIGGER_TYPE2::LOGON)?;
    task.get_Principal()?.put_RunLevel(co::TASK_RUNLEVEL::LUA)?;

    let action = task
        .get_Actions()?
        .Create(co::TASK_ACTION_TYPE::EXEC)?
        .QueryInterface::<w::IExecAction>()?;
    action.put_Path(&exe_path.display().to_string())?;
    if !arguments.is_empty() {
        action.put_Arguments(arguments)?;
    }
    if let Some(working_directory) = exe_path.parent() {
        action.put_WorkingDirectory(&working_directory.display().to_string())?;
    }

    folder.RegisterTaskDefinition(
        Some(STARTUP_TASK_NAME),
        &task,
        co::TASK_CREATION::CREATE_OR_UPDATE,
        None,
        None,
        co::TASK_LOGON::INTERACTIVE_TOKEN,
        None,
    )?;

    crate::log_info!(
        "Registered LOA Tracker Task Scheduler autostart task: {} {}",
        exe_path.display(),
        arguments
    );

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn refresh_startup_registration_impl(
    _enabled: bool,
    _background_monitor: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[tauri::command]
pub async fn set_start_with_lost_ark(
    settings_manager: State<'_, crate::settings::SettingsManager>,
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    let mut settings = settings_manager
        .read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());

    settings.system.start_with_lost_ark = enabled;
    settings_manager
        .save(&settings)
        .map_err(|e| format!("Failed to save settings: {}", e))?;
    refresh_startup_registration(&settings).map_err(|e| format!("Failed to update startup registration: {}", e))?;

    // Lost Ark Process Monitoring
    match set_lost_ark_monitoring(enabled, app) {
        Ok(_) => {
            crate::log_info!("Successfully set start with Lost Ark to: {}", enabled);
        }
        Err(e) => {
            crate::log_error!("Failed to set Lost Ark monitoring: {}", e);
            return Err(format!("Failed to set Lost Ark monitoring: {}", e));
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn is_lost_ark_running() -> Result<bool, String> {
    let mut system = System::new_all();
    system.refresh_processes();

    for process in system.processes().values() {
        let name = process.name();
        if name.to_lowercase() == "lostark.exe" {
            return Ok(true);
        }
    }

    Ok(false)
}

// Helper function for Lost Ark monitoring setup
pub(crate) fn set_lost_ark_monitoring(enabled: bool, app: AppHandle) -> Result<(), String> {
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    static MONITOR_STATE: std::sync::OnceLock<Arc<Mutex<bool>>> = std::sync::OnceLock::new();

    let state = MONITOR_STATE.get_or_init(|| Arc::new(Mutex::new(false)));

    if enabled {
        // Start monitoring thread if not already running
        {
            let mut is_monitoring = state
                .lock()
                .map_err(|_| "Lost Ark monitor state lock was poisoned".to_string())?;
            if *is_monitoring {
                crate::log_debug!("Lost Ark monitoring is already running");
                return Ok(());
            }
            *is_monitoring = true;
        }

        let state_clone = Arc::clone(state);
        let app_clone = app.clone();
        thread::spawn(move || {
            crate::log_info!("Started Lost Ark process monitoring thread");

            let mut system = System::new_all();
            let mut was_running = false;

            loop {
                // Check if monitoring should stop
                {
                    let Ok(is_monitoring) = state_clone.lock() else {
                        crate::log_error!("Lost Ark monitor state lock was poisoned; stopping monitor thread");
                        break;
                    };
                    if !*is_monitoring {
                        crate::log_debug!("Lost Ark monitoring thread stopping");
                        break;
                    }
                }

                // Refresh process list
                system.refresh_processes();

                let is_running = system
                    .processes()
                    .values()
                    .any(|process| is_lost_ark_process_name(&process.name().to_lowercase()));

                if is_running && !was_running {
                    crate::log_info!("Lost Ark detected starting up - showing LOA Tracker");
                    reveal_main_window(&app_clone);
                }

                was_running = is_running;

                // Check every 5 seconds
                thread::sleep(Duration::from_secs(5));
            }
        });

        crate::log_info!("Lost Ark process monitoring enabled");
    } else {
        // Stop monitoring
        {
            let mut is_monitoring = state
                .lock()
                .map_err(|_| "Lost Ark monitor state lock was poisoned".to_string())?;
            *is_monitoring = false;
        }
        crate::log_info!("Lost Ark process monitoring disabled");
    }

    Ok(())
}

pub(crate) fn set_loa_logs_monitoring(enabled: bool, app: AppHandle) -> Result<(), String> {
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    static MONITOR_STATE: std::sync::OnceLock<Arc<Mutex<bool>>> = std::sync::OnceLock::new();

    let state = MONITOR_STATE.get_or_init(|| Arc::new(Mutex::new(false)));

    if enabled {
        {
            let mut is_monitoring = state
                .lock()
                .map_err(|_| "LOA Logs monitor state lock was poisoned".to_string())?;
            if *is_monitoring {
                crate::log_debug!("LOA Logs companion monitoring is already running");
                return Ok(());
            }
            *is_monitoring = true;
        }

        let state_clone = Arc::clone(state);
        let app_clone = app.clone();
        thread::spawn(move || {
            crate::log_info!("Started LOA Logs companion monitoring thread");

            let mut system = System::new_all();
            let mut loa_logs_was_running = false;

            loop {
                {
                    let Ok(is_monitoring) = state_clone.lock() else {
                        crate::log_error!("LOA Logs monitor state lock was poisoned; stopping monitor thread");
                        break;
                    };
                    if !*is_monitoring {
                        crate::log_debug!("LOA Logs companion monitoring thread stopping");
                        break;
                    }
                }

                system.refresh_processes();

                let loa_logs_is_running = system
                    .processes()
                    .values()
                    .any(|process| is_loa_logs_process_name(&process.name().to_lowercase()));

                if loa_logs_is_running && !loa_logs_was_running {
                    crate::log_info!("LOA Logs detected starting up - showing LOA Tracker");
                    reveal_main_window(&app_clone);
                }

                loa_logs_was_running = loa_logs_is_running;
                thread::sleep(Duration::from_secs(5));
            }
        });

        crate::log_info!("LOA Logs companion monitoring enabled");
    } else {
        {
            let mut is_monitoring = state
                .lock()
                .map_err(|_| "LOA Logs monitor state lock was poisoned".to_string())?;
            *is_monitoring = false;
        }
        crate::log_info!("LOA Logs companion monitoring disabled");
    }

    Ok(())
}

pub(crate) fn reveal_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(e) = window.show() {
            crate::log_error!("Failed to show LOA Tracker window: {}", e);
        }
        if let Err(e) = window.unminimize() {
            crate::log_debug!("Failed to unminimize LOA Tracker window: {}", e);
        }
        if let Err(e) = window.set_focus() {
            crate::log_debug!("Failed to focus LOA Tracker window: {}", e);
        }
    } else {
        crate::log_warn!("Main LOA Tracker window was not found");
    }
}

fn is_lost_ark_process_name(name: &str) -> bool {
    name == "lostark.exe" || name.contains("lostark")
}

fn is_loa_logs_process_name(name: &str) -> bool {
    name == "loa logs.exe" || name == "loa_logs.exe" || (name.contains("loa") && name.contains("logs"))
}
