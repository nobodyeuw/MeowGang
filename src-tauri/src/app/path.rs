use std::path::PathBuf;
use tauri::Manager;
use dirs;

/// Returns the path where we store application data
/// 
/// Windows: Resolves to %LOCALAPPDATA%\LOA Tracker
pub fn data_dir(_app: &tauri::AppHandle) -> PathBuf {
    #[cfg(target_os = "windows")]
    let path = dirs::data_local_dir()
        .expect("could not get local app data dir")
        .join("LOA Tracker");
    
    #[cfg(not(target_os = "windows"))]
    let path = dirs::data_local_dir()
        .expect("could not get local app data dir")
        .join("LOA Tracker");

    path
}

/// Returns the path where we store application logs
/// Windows: Resolves to %LOCALAPPDATA%\LOA Tracker\logs
pub fn log_dir(_app: &tauri::AppHandle) -> PathBuf {
    #[cfg(target_os = "windows")]
    let path = dirs::data_local_dir()
        .expect("could not get local app data dir")
        .join("LOA Tracker")
        .join("logs");
    
    #[cfg(not(target_os = "windows"))]
    let path = dirs::data_local_dir()
        .expect("could not get local app data dir")
        .join("LOA Tracker")
        .join("logs");

    path
}

/// Returns the path where we store resources
/// Windows: Resolves to %LOCALAPPDATA%\LOA Tracker\resources
pub fn resources_dir(app: &tauri::AppHandle) -> PathBuf {
    data_dir(app).join("resources")
}

/// Returns the path where we store the database
/// Windows: Resolves to %LOCALAPPDATA%\LOA Tracker\userlogs.db
pub fn database_path(app: &tauri::AppHandle) -> PathBuf {
    data_dir(app).join("userlogs.db")
}

/// Returns the path where we store settings
/// Windows: Resolves to %LOCALAPPDATA%\LOA Tracker\settings.json
pub fn settings_path(app: &tauri::AppHandle) -> PathBuf {
    data_dir(app).join("settings.json")
}
