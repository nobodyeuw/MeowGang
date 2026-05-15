use tauri::State;
use crate::services::gold_logging_service::GoldLoggingService;
use crate::state::RaidDataState;
use crate::database::repositories::gold_repository::WeeklyGoldSummary;

#[tauri::command]
pub async fn update_raid_data_state(
    raids_data: Vec<crate::database::data_manager::Raid>,
    raid_state: State<'_, RaidDataState>
) -> Result<(), String> {
    match raid_state.update_raids(raids_data) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to update raid data state: {}", e))
    }
}

#[tauri::command]
pub async fn trigger_gold_processing(
    gold_service: State<'_, GoldLoggingService>,
    raid_state: State<'_, RaidDataState>
) -> Result<String, String> {
    match gold_service.process_pending_gold_logs(&*raid_state) {
        Ok(count) => Ok(format!("Successfully processed {} entries", count)),
        Err(e) => Err(e.to_string())
    }
}

#[tauri::command]
pub async fn process_pending_gold_logs(
    gold_service: State<'_, GoldLoggingService>,
    raid_state: State<'_, RaidDataState>
) -> Result<String, String> {
    match gold_service.process_pending_gold_logs(&*raid_state) {
        Ok(count) => Ok(format!("Processed {} gold log entries", count)),
        Err(e) => Err(format!("Failed to process gold logs: {}", e))
    }
}

#[tauri::command]
pub async fn get_weekly_gold_stats(
    _roster_id: Option<String>,
    gold_repo: State<'_, crate::database::repositories::GoldRepository>
) -> Result<WeeklyGoldSummary, String> {
    let conn = gold_repo.get_connection().map_err(|e| e.to_string())?;

    let week_start: i64 = conn.query_row(
        "SELECT last_weekly_reset FROM app_state LIMIT 1",
        [],
        |row| row.get(0)
    ).unwrap_or(0);

    let summary = gold_repo
        .get_weekly_gold_stats_all(week_start)
        .map_err(|e| e.to_string())?;

    crate::log_debug!("get_weekly_gold_stats: week_start={}, summary={:?}", week_start, summary);

    Ok(summary)
}


#[tauri::command]
pub async fn check_and_process_recent_completions(
    gold_service: State<'_, GoldLoggingService>,
    raid_state: State<'_, RaidDataState>
) -> Result<String, String> {
    match gold_service.check_and_process_recent_completions(&*raid_state) {
        Ok(count) => Ok(format!("Processed {} recent completions", count)),
        Err(e) => Err(format!("Failed to process recent completions: {}", e))
    }
}

#[tauri::command]
pub async fn delete_gold_logs_for_raid(
    gold_service: State<'_, GoldLoggingService>,
    char_id: i64,
    content_id: String,
    difficulty: String,
    session_id: String,
) -> Result<String, String> {
    match gold_service.delete_gold_logs_for_raid_completion(
        char_id,
        &content_id,
        &difficulty,
        &session_id
    ) {
        Ok(count) => Ok(format!("Deleted {} gold log entries and completion status for raid {}", count, content_id)),
        Err(e) => Err(format!("Failed to delete gold logs: {}", e))
    }
}

#[tauri::command]
pub async fn clean_duplicate_gold_logs(
    gold_service: State<'_, GoldLoggingService>,
    char_id: i64,
) -> Result<String, String> {
    match gold_service.clean_duplicate_gold_logs(char_id) {
        Ok(count) => Ok(format!("Cleaned up {} duplicate gold log entries for character {}", count, char_id)),
        Err(e) => Err(format!("Failed to clean duplicate gold logs: {}", e))
    }
}

#[tauri::command]
pub async fn clean_all_duplicate_gold_logs(
    gold_service: State<'_, GoldLoggingService>,
) -> Result<String, String> {
    match gold_service.clean_all_duplicate_gold_logs() {
        Ok(count) => Ok(format!("Cleaned up {} duplicate gold log entries total", count)),
        Err(e) => Err(format!("Failed to clean all duplicate gold logs: {}", e))
    }
}
