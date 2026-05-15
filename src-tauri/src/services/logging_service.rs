use anyhow::Result;
use chrono::Utc;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::io::Write;
use dirs;
use std::sync::Mutex;

pub struct LoggingService {
    log_file: Mutex<File>,
}

impl LoggingService {
    pub fn new() -> Result<Self> {
        let log_dir = get_log_directory()?;
        std::fs::create_dir_all(&log_dir)?;
        
        let log_file_path = log_dir.join("LOA Tracker.log");
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)?;
        
        Ok(Self {
            log_file: Mutex::new(log_file),
        })
    }

    pub fn log_info(&self, message: &str) {
        self.write_log("INFO", message);
    }

    pub fn log_error(&self, message: &str) {
        self.write_log("ERROR", message);
    }

    pub fn log_warn(&self, message: &str) {
        self.write_log("WARN", message);
    }

    pub fn log_debug(&self, message: &str) {
        self.write_log("DEBUG", message);
    }

    fn write_log(&self, level: &str, message: &str) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let log_entry = format!("[{}] [{}] {}\n", timestamp, level, message);
        
        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.write_all(log_entry.as_bytes());
            let _ = file.flush();
        }
        
        // Also print to console for development
        match level {
            "ERROR" => eprintln!("{}", log_entry.trim()),
            "WARN" => eprintln!("{}", log_entry.trim()),
            _ => println!("{}", log_entry.trim()),
        }
    }

    pub fn get_log_content(&self) -> Result<String> {
        let log_dir = get_log_directory()?;
        let log_file_path = log_dir.join("LOA Tracker.log");
        
        std::fs::read_to_string(log_file_path)
            .map_err(|e| anyhow::anyhow!("Failed to read log file: {}", e))
    }

    pub fn clear_log(&self) -> Result<()> {
        let log_dir = get_log_directory()?;
        let log_file_path = log_dir.join("LOA Tracker.log");
        
        std::fs::write(&log_file_path, "")
            .map_err(|e| anyhow::anyhow!("Failed to clear log file: {}", e))?;
        
        self.log_info("Log file cleared");
        Ok(())
    }
}

fn get_log_directory() -> Result<PathBuf> {
    // Use LocalAppData like AppContext (fallback for before Tauri init)
    #[cfg(target_os = "windows")]
    let app_data = dirs::data_local_dir()
        .or_else(|| dirs::data_dir())
        .ok_or_else(|| anyhow::anyhow!("Failed to get app data directory"))?;
    
    #[cfg(not(target_os = "windows"))]
    let app_data = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get app data directory"))?;
    
    let log_dir = app_data
        .join("LOA Tracker")
        .join("logs");
    
    Ok(log_dir)
}

pub fn init_logging_with_app_handle(app: &tauri::AppHandle) -> Result<()> {
    // Use Tauri's path resolution for consistency
    let log_dir = crate::app::log_dir(app);
    
    std::fs::create_dir_all(&log_dir)?;
    
    let log_file_path = log_dir.join("LOA Tracker.log");
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)?;
    
    let logger = LoggingService {
        log_file: Mutex::new(log_file),
    };
    
    unsafe {
        GLOBAL_LOGGER = Some(logger);
    }
    
    Ok(())
}

// Global logging instance
static mut GLOBAL_LOGGER: Option<LoggingService> = None;
static LOGGER_INIT: std::sync::Once = std::sync::Once::new();

pub fn init_logging() -> Result<()> {
    LOGGER_INIT.call_once(|| {
        match LoggingService::new() {
            Ok(logger) => {
                unsafe {
                    GLOBAL_LOGGER = Some(logger);
                }
                crate::log_info!("Logging system initialized");
            }
            Err(e) => {
                eprintln!("Failed to initialize logging: {}", e);
            }
        }
    });
    Ok(())
}

pub fn get_logger() -> Option<&'static LoggingService> {
    unsafe { 
        // Suppress the warning by using raw pointer pattern
        GLOBAL_LOGGER.as_ref()
    }
}

// Convenience functions
pub fn info(message: &str) {
    if let Some(logger) = get_logger() {
        logger.log_info(message);
    } else {
        println!("[INFO] {}", message);
    }
}

pub fn error(message: &str) {
    if let Some(logger) = get_logger() {
        logger.log_error(message);
    } else {
        eprintln!("[ERROR] {}", message);
    }
}

pub fn warn(message: &str) {
    if let Some(logger) = get_logger() {
        logger.log_warn(message);
    } else {
        eprintln!("[WARN] {}", message);
    }
}

pub fn debug(message: &str) {
    if let Some(logger) = get_logger() {
        logger.log_debug(message);
    } else {
        println!("[DEBUG] {}", message);
    }
}

// Macro for formatted logging
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::services::logging_service::info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::services::logging_service::error(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::services::logging_service::warn(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::services::logging_service::debug(&format!($($arg)*))
    };
}
