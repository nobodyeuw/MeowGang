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
    _roster_id: Option<String>, // Parameter kept for compatibility but ignored - we want all rosters
    gold_repo: State<'_, crate::database::repositories::GoldRepository>
) -> Result<WeeklyGoldSummary, String> {
    // 1. get database connection
    let conn = gold_repo.get_connection().map_err(|e| e.to_string())?;
    
    // 2. Weekly Reset Time determine
    let last_reset: i64 = conn.query_row(
        "SELECT last_weekly_reset FROM app_state LIMIT 1",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
    let week_start = last_reset;
    
    // 3. Get ALL character IDs with earns_gold=1 from ALL rosters
    let character_ids: Vec<i64> = conn.prepare("SELECT char_id FROM conf_character WHERE earns_gold = 1")
        .map_err(|e| e.to_string())?
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<i64>, _>>()
        .map_err(|e| e.to_string())?;
    
    // 4. Sum all values from gold_logs for characters that earn gold
    let mut stats = WeeklyGoldSummary {
        tradable_gold: 0,
        bound_gold: 0,
        total_gold: 0,
        total_entries: 0,
        extra_income_gold: 0,
        box_purchase_cost: 0,
    };
    
    // If no characters earn gold, return empty stats
    if character_ids.is_empty() {
        return Ok(stats);
    }
    
    // Build IN clause placeholders dynamically
    let placeholders: Vec<String> = character_ids.iter().map(|_| "?".to_string()).collect();
    let in_clause = placeholders.join(",");
    
    // Direct query to get all gold logs for gold-earning characters since weekly reset
    let sql = format!(
        "SELECT 
            COALESCE(SUM(gold_tradable), 0) as tradable,
            COALESCE(SUM(gold_bound), 0) as bound,
            COALESCE(SUM(gold_value_total), 0) as total,
            COUNT(*) as entries
         FROM gold_logs 
         WHERE char_id IN ({})
         AND timestamp >= ?",
        in_clause
    );
    
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    
    // Build parameters: character IDs + week_start
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
    for char_id in &character_ids {
        params.push(char_id);
    }
    params.push(&week_start);
    
    let summary = stmt.query_row(&*params, |row| {
        Ok(WeeklyGoldSummary {
            tradable_gold: row.get(0)?,
            bound_gold: row.get(1)?,
            total_gold: row.get(2)?,
            total_entries: row.get(3)?,
            extra_income_gold: 0, // Will be calculated separately
            box_purchase_cost: 0, // Will be calculated separately
        })
    }).map_err(|e| e.to_string())?;
    
    crate::log_debug!("get_weekly_gold_stats: week_start={}, character_ids.len()={}, summary={:?}", week_start, character_ids.len(), summary);
    
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
