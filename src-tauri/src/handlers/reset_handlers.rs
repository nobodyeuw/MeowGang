use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tauri::State;

/// Direct reset function that works without State wrapper (deprecated - use ResetService instead)
pub async fn reset_tasks_if_needed_direct(pool: Pool<SqliteConnectionManager>) -> Result<String, String> {
    let service = crate::services::reset_service::ResetService::new(pool);
    service.perform_reset().await.map_err(|e| e.to_string())
}

/// Tauri command for manual reset
#[tauri::command]
pub async fn perform_manual_reset<'r>(pool: State<'r, Pool<SqliteConnectionManager>>) -> Result<String, String> {
    let service = crate::services::reset_service::ResetService::new(pool.inner().clone());
    service.perform_reset().await.map_err(|e| e.to_string())
}

/// Tauri command to check if calendar task is available
#[tauri::command]
pub async fn check_calendar_task_availability<'r>(task_id: String, pool: State<'r, Pool<SqliteConnectionManager>>) -> Result<bool, String> {
    let service = crate::services::reset_service::ResetService::new(pool.inner().clone());
    service.is_calendar_task_available(&task_id).map_err(|e| e.to_string())
}

/// Tauri command to get next reset time for a task
#[tauri::command]
pub async fn get_next_reset_time<'r>(task_id: String, pool: State<'r, Pool<SqliteConnectionManager>>) -> Result<String, String> {
    let service = crate::services::reset_service::ResetService::new(pool.inner().clone());
    let next_reset = service.get_next_reset_time(&task_id).map_err(|e| e.to_string())?;
    Ok(next_reset.format("%Y-%m-%d %H:%M:%S UTC").to_string())
}

/// Tauri command to update rested values immediately (called on app start)
#[tauri::command]
pub async fn update_rested_values_now<'r>(pool: State<'r, Pool<SqliteConnectionManager>>) -> Result<String, String> {
    let service = crate::services::reset_service::ResetService::new(pool.inner().clone());
    service.update_rested_values_only().await.map_err(|e| e.to_string())
}

/// Tauri command to get next daily reset time for countdown
#[tauri::command]
pub async fn get_next_daily_reset_time<'r>(pool: State<'r, Pool<SqliteConnectionManager>>) -> Result<String, String> {
    let service = crate::services::reset_service::ResetService::new(pool.inner().clone());
    let next_reset = service.get_next_reset_time("chaos").map_err(|e| e.to_string())?;
    Ok(next_reset.format("%Y-%m-%d %H:%M:%S UTC").to_string())
}
