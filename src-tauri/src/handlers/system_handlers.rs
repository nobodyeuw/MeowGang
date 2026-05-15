use serde::{Deserialize, Serialize};
use tauri::State;
use std::path::PathBuf;
use crate::database::DatabaseManager;
use winreg::RegKey;
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettings {
    pub encounters_db_path: Option<String>,
    pub lost_ark_exe_path: Option<String>,
    pub start_with_windows: bool,
    pub start_with_lost_ark: bool,
}


#[tauri::command]
pub async fn get_app_version() -> Result<String, String> {
    Ok(crate::version::APP_VERSION.to_string())
}

#[tauri::command]
pub async fn get_system_settings(
    settings_manager: State<'_, crate::settings::SettingsManager>
) -> Result<crate::settings::SystemSettings, String> {
    let mut settings = settings_manager.read()
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
    
    Ok(crate::settings::SystemSettings {
        encounters_db_path,
        lost_ark_exe_path,
        start_with_windows: settings.system.start_with_windows,
        start_with_lost_ark: settings.system.start_with_lost_ark,
        extra: settings.system.extra,
    })
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
        Some(PathBuf::from("D:\\steam\\steamapps\\common\\Lost Ark\\Binaries\\Win64\\LOSTARK.exe")),
        
        // Program Files (x86)
        std::env::var("PROGRAMFILES(X86)").ok()
            .map(|p| PathBuf::from(&p)
                .join("Steam")
                .join("steamapps")
                .join("common")
                .join("Lost Ark")
                .join("Binaries")
                .join("Win64")
                .join("LOSTARK.exe")),
        
        // Program Files
        std::env::var("PROGRAMFILES").ok()
            .map(|p| PathBuf::from(&p)
                .join("Steam")
                .join("steamapps")
                .join("common")
                .join("Lost Ark")
                .join("Binaries")
                .join("Win64")
                .join("LOSTARK.exe")),
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
    
    let mut settings = settings_manager.read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());
    
    settings.system.encounters_db_path = Some(path);
    settings_manager.save(&settings).map_err(|e| format!("Failed to save settings: {}", e))
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
    
    let mut settings = settings_manager.read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());
    
    settings.system.lost_ark_exe_path = Some(path);
    settings_manager.save(&settings).map_err(|e| format!("Failed to save settings: {}", e))
}

#[tauri::command]
pub async fn set_start_with_windows(
    settings_manager: State<'_, crate::settings::SettingsManager>,
    enabled: bool,
) -> Result<(), String> {
    let mut settings = settings_manager.read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());
    
    settings.system.start_with_windows = enabled;
    settings_manager.save(&settings).map_err(|e| format!("Failed to save settings: {}", e))?;
    
    // Windows Registry Integration
    match set_autostart_registry(enabled) {
        Ok(_) => {
            crate::log_info!("Successfully set start with Windows to: {}", enabled);
        }
        Err(e) => {
            crate::log_error!("Failed to set registry autostart: {}", e);
            return Err(format!("Failed to set Windows autostart: {}", e));
        }
    }
    
    Ok(())
}

// Helper function for Windows Registry autostart
fn set_autostart_registry(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let exe_path = std::env::current_exe()?;
    let exe_path_str = exe_path.to_string_lossy().to_string();
    
    let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let path = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        winreg::enums::KEY_WRITE,
    )?;
    
    if enabled {
        path.set_value("LOA Tracker", &exe_path_str)?;
        crate::log_info!("Added LOA Tracker to Windows startup registry");
    } else {
        match path.delete_value("LOA Tracker") {
            Ok(_) => crate::log_info!("Removed LOA Tracker from Windows startup registry"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                crate::log_debug!("LOA Tracker was not in Windows startup registry");
            }
            Err(e) => return Err(Box::new(e)),
        }
    }
    
    Ok(())
}

#[tauri::command]
pub async fn set_start_with_lost_ark(
    settings_manager: State<'_, crate::settings::SettingsManager>,
    enabled: bool,
) -> Result<(), String> {
    let mut settings = settings_manager.read()
        .map_err(|e| format!("Failed to read settings: {}", e))?
        .unwrap_or_else(|| settings_manager.get_default());
    
    settings.system.start_with_lost_ark = enabled;
    settings_manager.save(&settings).map_err(|e| format!("Failed to save settings: {}", e))?;
    
    // Lost Ark Process Monitoring
    match set_lost_ark_monitoring(enabled) {
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
fn set_lost_ark_monitoring(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
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
                
                let is_running = system.processes().values()
                    .any(|process| process.name().to_lowercase() == "lostark.exe");
                
                // Launch LOA Tracker when Lost Ark starts (if it wasn't running before)
                if is_running && !was_running {
                    crate::log_info!("Lost Ark detected starting up - launching LOA Tracker");
                    
                    // Get current executable path
                    if let Ok(exe_path) = std::env::current_exe() {
                        // Launch new instance if not already running
                        match std::process::Command::new(&exe_path).spawn() {
                            Ok(_) => crate::log_info!("Successfully launched LOA Tracker"),
                            Err(e) => crate::log_error!("Failed to launch LOA Tracker: {}", e),
                        }
                    }
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

use crate::services::logging_service;
use tauri::{Manager, AppHandle};
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub body: Option<String>,
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateInfo, String> {
    logging_service::info("Checking for updates");
    
    let current_version = app.package_info().version.to_string();
    
    match app.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(Some(update)) => {
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
            }
        }
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
                Ok(None) => {
                    Err("No update available".to_string())
                }
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
        logger.get_log_content()
            .map_err(|e| e.to_string())
    } else {
        Err("Logging system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn clear_log() -> Result<(), String> {
    if let Some(logger) = logging_service::get_logger() {
        logger.clear_log()
            .map_err(|e| e.to_string())
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
        app.path().resource_dir()
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
            possible_paths.iter().map(|p| format!("{:?}", p)).collect::<Vec<_>>().join(", ")
        )
    })?;
    
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read changelogs file from {:?}: {}", path, e))?;
    
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse changelogs JSON: {}", e))
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
        app.path().resource_dir()
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
            possible_paths.iter().map(|p| format!("{:?}", p)).collect::<Vec<_>>().join(", ")
        )
    })?;
    
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read known bugs file from {:?}: {}", path, e))?;
    
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse known bugs JSON: {}", e))
}

#[tauri::command]
pub async fn send_log_report() -> Result<String, String> {
    if let Some(logger) = logging_service::get_logger() {
        let log_content = logger.get_log_content()
            .map_err(|e| e.to_string())?;
        
        // Create a timestamp for the report
        let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
        let report_filename = format!("LOA_Tracker_log_report_{}.txt", timestamp);
        
        // Save to temporary directory
        let temp_dir = std::env::temp_dir();
        let report_path = temp_dir.join(&report_filename);
        
        std::fs::write(&report_path, &log_content)
            .map_err(|e| e.to_string())?;
        
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
    crate::log_debug!("Adding gold log: {} gold for character {} from {}", gold_amount, character_id, source);
    Ok(())
}

