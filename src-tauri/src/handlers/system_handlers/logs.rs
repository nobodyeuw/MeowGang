use crate::services::logging_service;

/// Returns the current application log content for the Settings diagnostics view.
#[tauri::command]
pub async fn get_log_content() -> Result<String, String> {
    logging_service::get_log_content().map_err(|e| e.to_string())
}

/// Clears the current application log file from the Settings diagnostics view.
#[tauri::command]
pub async fn clear_log() -> Result<(), String> {
    logging_service::clear_log().map_err(|e| e.to_string())
}

/// Lets frontend code write structured messages into the same user-facing log file.
#[tauri::command]
pub async fn write_frontend_log(level: String, message: String) -> Result<(), String> {
    let trimmed_message = message.trim();
    if trimmed_message.is_empty() {
        return Ok(());
    }

    let log_message = format!("Frontend: {}", trimmed_message);
    match level.trim().to_lowercase().as_str() {
        "error" => logging_service::error(&log_message),
        "warn" | "warning" => logging_service::warn(&log_message),
        "debug" => logging_service::debug(&log_message),
        _ => logging_service::info(&log_message),
    }

    Ok(())
}

/// Writes a timestamped copy of the current log to the temp directory and opens it.
#[tauri::command]
pub async fn send_log_report() -> Result<String, String> {
    let log_content = logging_service::get_log_content().map_err(|e| e.to_string())?;

    let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
    let report_filename = format!("LOA_Tracker_log_report_{}.txt", timestamp);
    let report_path = std::env::temp_dir().join(&report_filename);

    std::fs::write(&report_path, &log_content).map_err(|e| e.to_string())?;

    logging_service::info(&format!("Log report created: {}", report_path.display()));

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &report_path.to_string_lossy()])
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
}
