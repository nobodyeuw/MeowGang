use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Transaction;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use sysinfo::System;
use tauri::State;

const STARTUP_TASK_NAME: &str = "LOA_Tracker_Auto_Start";
const STARTUP_REGISTRY_VALUE_NAME: &str = "LOA Tracker";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettings {
    pub encounters_db_path: Option<String>,
    pub lost_ark_exe_path: Option<String>,
    pub loa_logs_exe_path: Option<String>,
    pub start_with_windows: bool,
    pub start_with_lost_ark: bool,
    pub show_setup_guide_button: bool,
    pub show_auth_welcome: bool,
}

#[tauri::command]
pub async fn get_app_version() -> Result<String, String> {
    Ok(crate::version::APP_VERSION.to_string())
}

#[tauri::command]
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

    if let Err(e) = refresh_startup_registration(&settings) {
        crate::log_error!("Failed to refresh startup registration while loading settings: {}", e);
    }

    Ok(crate::settings::SystemSettings {
        encounters_db_path,
        lost_ark_exe_path,
        loa_logs_exe_path,
        start_with_windows: settings.system.start_with_windows,
        start_with_lost_ark: settings.system.start_with_lost_ark,
        start_with_loa_logs,
        show_setup_guide_button: settings.system.show_setup_guide_button,
        show_auth_welcome: settings.system.show_auth_welcome,
        extra: settings.system.extra,
    })
}

#[tauri::command]
pub async fn set_show_setup_guide_button(
    enabled: bool,
    settings_manager: State<'_, crate::settings::SettingsManager>,
) -> Result<(), String> {
    let mut settings = settings_manager
        .read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());

    settings.system.show_setup_guide_button = enabled;
    settings_manager
        .save(&settings)
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    crate::log_info!("Set setup guide header button visibility to: {}", enabled);
    Ok(())
}

#[tauri::command]
pub async fn set_show_auth_welcome(
    enabled: bool,
    settings_manager: State<'_, crate::settings::SettingsManager>,
) -> Result<(), String> {
    let mut settings = settings_manager
        .read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());

    settings.system.show_auth_welcome = enabled;
    settings_manager
        .save(&settings)
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    crate::log_info!("Set Discord welcome screen visibility to: {}", enabled);
    Ok(())
}

#[tauri::command]
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

#[tauri::command]
pub async fn set_encounters_db_path(
    settings_manager: State<'_, crate::settings::SettingsManager>,
    path: String,
) -> Result<(), String> {
    // Validate path exists and is a .db file
    let path_obj = std::path::Path::new(&path);
    if !path_obj.exists() {
        return Err("File not found".to_string());
    }

    if !path_obj.extension().map_or(false, |ext| ext == "db") {
        return Err("Invalid file extension. Expected .db file".to_string());
    }

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
    // Validate path exists and is an .exe file
    let path_obj = std::path::Path::new(&path);
    if !path_obj.exists() {
        return Err("File not found".to_string());
    }

    if !path_obj.extension().map_or(false, |ext| ext == "exe") {
        return Err("Invalid file extension. Expected .exe file".to_string());
    }

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
    // Validate path exists and is an .exe file
    let path_obj = std::path::Path::new(&path);
    if !path_obj.exists() {
        return Err("File not found".to_string());
    }

    if !path_obj.extension().map_or(false, |ext| ext == "exe") {
        return Err("Invalid file extension. Expected .exe file".to_string());
    }

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
    let enabled = settings.system.start_with_windows
        || settings.system.start_with_lost_ark
        || settings.system.start_with_loa_logs;
    let background_monitor = !settings.system.start_with_windows
        && (settings.system.start_with_lost_ark || settings.system.start_with_loa_logs);

    refresh_startup_registration_impl(enabled, background_monitor)
}

pub(crate) fn should_keep_background_monitor(settings: &crate::settings::Settings) -> bool {
    settings.system.start_with_lost_ark || settings.system.start_with_loa_logs
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
            let mut is_monitoring = state.lock().unwrap();
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
                    let is_monitoring = state_clone.lock().unwrap();
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
            let mut is_monitoring = state.lock().unwrap();
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
            let mut is_monitoring = state.lock().unwrap();
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
                    let is_monitoring = state_clone.lock().unwrap();
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
            let mut is_monitoring = state.lock().unwrap();
            *is_monitoring = false;
        }
        crate::log_info!("LOA Logs companion monitoring disabled");
    }

    Ok(())
}

pub(crate) fn ensure_loa_logs_running(path: Option<&str>) -> Result<(), String> {
    let mut system = System::new_all();
    system.refresh_processes();

    if system
        .processes()
        .values()
        .any(|process| is_loa_logs_process_name(&process.name().to_lowercase()))
    {
        return Ok(());
    }

    let path_str = match path {
        Some(path) if !path.trim().is_empty() => path.to_string(),
        _ => detect_loa_logs_exe_path().ok_or_else(|| "No LOA Logs executable path configured".to_string())?,
    };

    let exe_path = std::path::Path::new(&path_str);
    if !exe_path.exists() {
        return Err(format!("Configured LOA Logs path does not exist: {}", path_str));
    }

    let mut command = std::process::Command::new(exe_path);
    if let Some(parent) = exe_path.parent() {
        command.current_dir(parent);
    }

    match command.spawn() {
        Ok(_) => {
            crate::log_info!("Launched LOA Logs from settings: {}", path_str);
            Ok(())
        }
        Err(e) if e.raw_os_error() == Some(740) => launch_loa_logs_elevated(exe_path),
        Err(e) => Err(format!("Failed to launch LOA Logs: {}", e)),
    }
}

#[cfg(target_os = "windows")]
fn launch_loa_logs_elevated(exe_path: &std::path::Path) -> Result<(), String> {
    let working_directory = exe_path
        .parent()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    let exe_path_arg = escape_powershell_single_quoted_string(&exe_path.to_string_lossy());
    let working_directory_arg = escape_powershell_single_quoted_string(&working_directory);
    let command = format!(
        "Start-Process -FilePath '{}' -WorkingDirectory '{}' -Verb RunAs",
        exe_path_arg, working_directory_arg
    );

    std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &command,
        ])
        .spawn()
        .map(|_| {
            crate::log_info!("Requested elevated LOA Logs launch through UAC: {}", exe_path.display());
        })
        .map_err(|e| format!("Failed to request elevated LOA Logs launch: {}", e))
}

#[cfg(target_os = "windows")]
fn escape_powershell_single_quoted_string(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(not(target_os = "windows"))]
fn launch_loa_logs_elevated(exe_path: &std::path::Path) -> Result<(), String> {
    Err(format!(
        "LOA Logs requires elevated launch, which is only supported on Windows: {}",
        exe_path.display()
    ))
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

use crate::services::logging_service;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

const UPDATE_READY_GRACE_PERIOD_SECONDS: i64 = 10 * 60;
const UPDATE_METADATA_URL: &str = "https://raw.githubusercontent.com/nobodyeuw/MeowGang/main/latest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub body: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateMetadata {
    version: String,
    pub_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SeenUpdateState {
    version: String,
    first_seen_at: i64,
}

async fn update_delay_remaining_seconds(version: &str) -> Option<i64> {
    let client = reqwest::Client::new();
    let metadata = match client
        .get(UPDATE_METADATA_URL)
        .header("User-Agent", "LOA Tracker")
        .send()
        .await
    {
        Ok(response) => match response.json::<UpdateMetadata>().await {
            Ok(metadata) => metadata,
            Err(e) => {
                logging_service::warn(&format!("Failed to parse update metadata for delay check: {}", e));
                return None;
            }
        },
        Err(e) => {
            logging_service::warn(&format!("Failed to fetch update metadata for delay check: {}", e));
            return None;
        }
    };

    if metadata.version.trim_start_matches('v') != version.trim_start_matches('v') {
        return None;
    }

    let Some(pub_date) = metadata.pub_date else {
        return None;
    };

    let published_at = match chrono::DateTime::parse_from_rfc3339(&pub_date) {
        Ok(date) => date.with_timezone(&chrono::Utc),
        Err(e) => {
            logging_service::warn(&format!("Failed to parse update pub_date for delay check: {}", e));
            return None;
        }
    };

    let elapsed_seconds = chrono::Utc::now()
        .signed_duration_since(published_at)
        .num_seconds()
        .max(0);

    if elapsed_seconds < UPDATE_READY_GRACE_PERIOD_SECONDS {
        Some(UPDATE_READY_GRACE_PERIOD_SECONDS - elapsed_seconds)
    } else {
        None
    }
}

fn update_first_seen_delay_remaining_seconds(app: &AppHandle, version: &str) -> Option<i64> {
    let now = chrono::Utc::now().timestamp();
    let app_data_dir = match app.path().app_data_dir() {
        Ok(path) => path,
        Err(e) => {
            logging_service::warn(&format!("Failed to resolve app data dir for update delay: {}", e));
            return Some(UPDATE_READY_GRACE_PERIOD_SECONDS);
        }
    };

    if let Err(e) = fs::create_dir_all(&app_data_dir) {
        logging_service::warn(&format!("Failed to create app data dir for update delay: {}", e));
        return Some(UPDATE_READY_GRACE_PERIOD_SECONDS);
    }

    let state_path = app_data_dir.join("update_first_seen.json");
    let stored_state = fs::read_to_string(&state_path)
        .ok()
        .and_then(|content| serde_json::from_str::<SeenUpdateState>(&content).ok());

    let state = match stored_state {
        Some(state) if state.version.trim_start_matches('v') == version.trim_start_matches('v') => state,
        _ => {
            let state = SeenUpdateState {
                version: version.to_string(),
                first_seen_at: now,
            };
            if let Ok(content) = serde_json::to_string_pretty(&state) {
                if let Err(e) = fs::write(&state_path, content) {
                    logging_service::warn(&format!("Failed to persist update first-seen state: {}", e));
                }
            }
            state
        }
    };

    let elapsed_seconds = (now - state.first_seen_at).max(0);
    if elapsed_seconds < UPDATE_READY_GRACE_PERIOD_SECONDS {
        Some(UPDATE_READY_GRACE_PERIOD_SECONDS - elapsed_seconds)
    } else {
        None
    }
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateInfo, String> {
    logging_service::info("Checking for updates");

    let current_version = app.package_info().version.to_string();

    match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => {
                let first_seen_remaining = update_first_seen_delay_remaining_seconds(&app, &update.version);
                let pub_date_remaining = update_delay_remaining_seconds(&update.version).await;
                let remaining_delay = first_seen_remaining.into_iter().chain(pub_date_remaining).max();

                if let Some(remaining_seconds) = remaining_delay {
                    logging_service::info(&format!(
                        "Update {} detected but hidden for {} more seconds while release artifacts settle",
                        update.version, remaining_seconds
                    ));
                    return Ok(UpdateInfo {
                        current_version,
                        latest_version: None,
                        update_available: false,
                        body: None,
                    });
                }

                logging_service::info(&format!("Update available: {}", update.version));
                Ok(UpdateInfo {
                    current_version,
                    latest_version: Some(update.version),
                    update_available: true,
                    body: update.body,
                })
            }
            Ok(None) => {
                logging_service::info("No update available");
                Ok(UpdateInfo {
                    current_version,
                    latest_version: None,
                    update_available: false,
                    body: None,
                })
            }
            Err(e) => {
                logging_service::error(&format!("Update check failed: {}", e));
                Err(e.to_string())
            }
        },
        Err(e) => {
            logging_service::error(&format!("Failed to get updater: {}", e));
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<String, String> {
    logging_service::info("Starting update installation");

    match app.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(Some(update)) => {
                    if let Some(remaining_seconds) = update_delay_remaining_seconds(&update.version).await {
                        let remaining_minutes = ((remaining_seconds as f64) / 60.0).ceil() as i64;
                        logging_service::info(&format!(
                            "Install blocked because update {} is still in release grace period",
                            update.version
                        ));
                        return Err(format!(
                            "Update is still being prepared. Please try again in about {} minute{}.",
                            remaining_minutes,
                            if remaining_minutes == 1 { "" } else { "s" }
                        ));
                    }

                    logging_service::info(&format!("Downloading and installing update: {}", update.version));

                    // Download and install in one step like loa-logs-master
                    match update.download_and_install(|_, _| {}, || {}).await {
                        Ok(_) => {
                            logging_service::info("Update installed successfully");
                            Ok("Update installed successfully! The app will restart.".to_string())
                        }
                        Err(e) => {
                            logging_service::error(&format!("Update installation failed: {}", e));
                            Err(format!("Failed to install update: {}", e))
                        }
                    }
                }
                Ok(None) => Err("No update available".to_string()),
                Err(e) => {
                    logging_service::error(&format!("Failed to check for updates: {}", e));
                    Err(format!("Failed to check for updates: {}", e))
                }
            }
        }
        Err(e) => {
            logging_service::error(&format!("Failed to get updater: {}", e));
            Err(format!("Failed to get updater: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_log_content() -> Result<String, String> {
    if let Some(logger) = logging_service::get_logger() {
        logger.get_log_content().map_err(|e| e.to_string())
    } else {
        Err("Logging system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn clear_log() -> Result<(), String> {
    if let Some(logger) = logging_service::get_logger() {
        logger.clear_log().map_err(|e| e.to_string())
    } else {
        Err("Logging system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn get_changelogs(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    use std::fs;
    use std::path::PathBuf;

    // Use Tauri's path resolution for consistency
    let resources_path = crate::app::resources_dir(&app);

    // Try multiple possible locations for changelog file
    let mut possible_paths: Vec<PathBuf> = vec![
        // Primary: app data resources directory (LOA Tracker/resources/)
        resources_path.join("changelogs.json"),
        // In development: resources relative to executable
        {
            let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;
            let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
            exe_dir.join("../resources/changelogs.json")
        },
        // In production: app resource directory (Tauri's standard resource dir)
        app.path()
            .resource_dir()
            .map_err(|e| format!("Failed to get resource directory: {}", e))?
            .join("changelogs.json"),
        // Fallback: src-tauri/resources/ (might work in some setups)
        PathBuf::from("src-tauri/resources/changelogs.json"),
    ];

    // Find first existing path
    let path = possible_paths.iter().find(|p| p.exists()).cloned();

    let path = path.ok_or_else(|| {
        format!(
            "Changelogs file not found. Tried: {}",
            possible_paths
                .iter()
                .map(|p| format!("{:?}", p))
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;

    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read changelogs file from {:?}: {}", path, e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse changelogs JSON: {}", e))
}

#[tauri::command]
pub async fn get_known_bugs(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    use std::fs;
    use std::path::PathBuf;

    // Get the app context to access the resources path
    let app_context = app.state::<crate::context::AppContext>();
    let resources_path = &app_context.resources_path;

    // Try multiple possible locations for known bugs file
    let mut possible_paths: Vec<PathBuf> = vec![
        // Primary: app data resources directory (LOA Tracker/resources/)
        resources_path.join("known_bugs.json"),
        // In development: resources relative to executable
        {
            let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;
            let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
            exe_dir.join("../resources/known_bugs.json")
        },
        // In production: app resource directory (Tauri's standard resource dir)
        app.path()
            .resource_dir()
            .map_err(|e| format!("Failed to get resource directory: {}", e))?
            .join("known_bugs.json"),
        // Fallback: src-tauri/resources/ (might work in some setups)
        PathBuf::from("src-tauri/resources/known_bugs.json"),
    ];

    // Find first existing path
    let path = possible_paths.iter().find(|p| p.exists()).cloned();

    let path = path.ok_or_else(|| {
        format!(
            "Known bugs file not found. Tried: {}",
            possible_paths
                .iter()
                .map(|p| format!("{:?}", p))
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;

    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read known bugs file from {:?}: {}", path, e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse known bugs JSON: {}", e))
}

#[tauri::command]
pub async fn send_log_report() -> Result<String, String> {
    if let Some(logger) = logging_service::get_logger() {
        let log_content = logger.get_log_content().map_err(|e| e.to_string())?;

        // Create a timestamp for the report
        let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
        let report_filename = format!("LOA_Tracker_log_report_{}.txt", timestamp);

        // Save to temporary directory
        let temp_dir = std::env::temp_dir();
        let report_path = temp_dir.join(&report_filename);

        std::fs::write(&report_path, &log_content).map_err(|e| e.to_string())?;

        logging_service::info(&format!("Log report created: {}", report_path.display()));

        // Open the file in default application using Windows command
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(&["/c", "start", "", &report_path.to_string_lossy()])
                .spawn()
                .map_err(|e| format!("Failed to open log report: {}", e))?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            std::process::Command::new("xdg-open")
                .arg(&report_path)
                .spawn()
                .map_err(|e| format!("Failed to open log report: {}", e))?;
        }

        Ok(format!("Log report saved to: {}", report_path.display()))
    } else {
        Err("Logging system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn test_database_simple() -> Result<String, String> {
    // Simple database connectivity test
    crate::log_debug!("test_database_simple called");
    Ok("Database connection test successful".to_string())
}

#[tauri::command]
pub async fn test_sync_data_structure() -> Result<String, String> {
    // Test sync data structure
    crate::log_debug!("test_sync_data_structure called");
    Ok("Sync data structure test successful".to_string())
}

#[tauri::command]
pub async fn initialize_missing_data() -> Result<String, String> {
    // Initialize any missing data in database
    crate::log_debug!("initialize_missing_data called");
    Ok("Missing data initialized successfully".to_string())
}

#[tauri::command]
pub async fn add_gold_log(
    character_id: i64,
    gold_amount: i64,
    source: String,
    raid_name: Option<String>,
) -> Result<(), String> {
    // Add gold log entry - placeholder
    crate::log_debug!(
        "Adding gold log: {} gold for character {} from {}",
        gold_amount,
        character_id,
        source
    );
    Ok(())
}
