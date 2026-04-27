use std::path::PathBuf;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AppContext {
    pub version: String,
    pub app_path: PathBuf,
    pub current_dir: PathBuf,
    pub app_data_dir: PathBuf,
    pub database_path: PathBuf,
    pub settings_path: PathBuf,
    pub logs_path: PathBuf,
    pub resources_path: PathBuf,
}

impl AppContext {
    pub fn new(version: String) -> Result<Self> {
        let app_path = std::env::current_exe()?;
        let current_dir = app_path.parent()
            .ok_or_else(|| anyhow::anyhow!("Could not determine parent directory of executable"))?
            .to_path_buf();

        // Use default app data directory (Roaming on Windows)
        #[cfg(target_os = "windows")]
        let app_data_dir = dirs::data_dir()
            .unwrap_or_else(|| current_dir.clone())
            .join("LOA Tracker");
        
        #[cfg(not(target_os = "windows"))]
        let app_data_dir = dirs::data_dir()
            .unwrap_or_else(|| current_dir.clone())
            .join("LOA Tracker");

        let database_path = app_data_dir.join("userlogs.db");
        let settings_path = app_data_dir.join("settings.json");
        let logs_path = app_data_dir.join("logs");
        let resources_path = app_data_dir.join("resources");

        Ok(Self {
            version,
            app_path,
            current_dir,
            app_data_dir,
            database_path,
            settings_path,
            logs_path,
            resources_path,
        })
    }

    pub fn ensure_directories(&self) -> Result<()> {
        if !self.app_data_dir.exists() {
            std::fs::create_dir_all(&self.app_data_dir)?;
        }
        if !self.logs_path.exists() {
            std::fs::create_dir_all(&self.logs_path)?;
        }
        if !self.resources_path.exists() {
            std::fs::create_dir_all(&self.resources_path)?;
        }
        // Skip resource file copying to avoid file locking issues
        Ok(())
    }

    pub fn update_paths_with_tauri(&mut self, app: &tauri::AppHandle) -> Result<()> {
        // Update paths using Tauri's path resolution for consistency
        let roaming_data_dir = crate::app::data_dir(app);
        
        // Keep app_data_dir as Roaming for window state plugin only
        self.app_data_dir = roaming_data_dir.clone();
        
        // But use LocalAppData for all other data
        let local_data_dir = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not get local app data directory"))?
            .join("LOA Tracker");
        
        self.database_path = local_data_dir.join("userlogs.db");
        self.settings_path = local_data_dir.join("settings.json");
        self.logs_path = local_data_dir.join("logs");
        self.resources_path = local_data_dir.join("resources");
        
        Ok(())
    }

    fn copy_resource_files(&self) -> Result<()> {
        // Skip resource file copying to avoid file locking issues
        // Resources will be loaded directly from the app bundle
        Ok(())
    }
}
