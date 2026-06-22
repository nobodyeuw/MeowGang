use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::database::DatabaseManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardCalendarAssignment {
    pub event_key: String,
    pub sheet_id: String,
    pub event_id: String,
    pub section_code: Option<String>,
    pub char_id: i64,
    pub char_name: String,
    pub raid_content_id: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardCalendarAssignmentInput {
    pub event_key: String,
    pub sheet_id: String,
    pub event_id: String,
    pub section_code: Option<String>,
    pub char_id: i64,
    pub char_name: String,
    pub raid_content_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardRaidReservation {
    pub id: String,
    pub char_id: i64,
    pub content_id: String,
    pub difficulty: String,
    pub label: String,
    pub reserved_at: i64,
    pub scheduled_at: Option<i64>,
    pub recurring_weekly: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardRaidReservationInput {
    pub char_id: i64,
    pub content_id: String,
    pub difficulty: String,
    pub label: String,
    pub scheduled_at: Option<i64>,
    pub recurring_weekly: bool,
}

#[tauri::command]
pub fn get_dashboard_calendar_assignments(
    db_manager: State<'_, DatabaseManager>,
) -> Result<Vec<DashboardCalendarAssignment>, String> {
    let conn = db_manager
        .get_connection()
        .map_err(|e| format!("Failed to open database: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT event_key, sheet_id, event_id, section_code, char_id, char_name, raid_content_id, updated_at
             FROM dashboard_calendar_assignments
             ORDER BY updated_at DESC",
        )
        .map_err(|e| format!("Failed to prepare assignment query: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(DashboardCalendarAssignment {
                event_key: row.get(0)?,
                sheet_id: row.get(1)?,
                event_id: row.get(2)?,
                section_code: row.get(3)?,
                char_id: row.get(4)?,
                char_name: row.get(5)?,
                raid_content_id: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| format!("Failed to read assignments: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map assignments: {}", e))
}

#[tauri::command]
pub fn save_dashboard_calendar_assignment(
    input: DashboardCalendarAssignmentInput,
    db_manager: State<'_, DatabaseManager>,
) -> Result<DashboardCalendarAssignment, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let conn = db_manager
        .get_connection()
        .map_err(|e| format!("Failed to open database: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO dashboard_calendar_assignments
         (event_key, sheet_id, event_id, section_code, char_id, char_name, raid_content_id, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            input.event_key,
            input.sheet_id,
            input.event_id,
            input.section_code,
            input.char_id,
            input.char_name,
            input.raid_content_id,
            now
        ],
    )
    .map_err(|e| format!("Failed to save assignment: {}", e))?;

    Ok(DashboardCalendarAssignment {
        event_key: input.event_key,
        sheet_id: input.sheet_id,
        event_id: input.event_id,
        section_code: input.section_code,
        char_id: input.char_id,
        char_name: input.char_name,
        raid_content_id: input.raid_content_id,
        updated_at: now,
    })
}

#[tauri::command]
pub fn clear_dashboard_calendar_assignment(
    event_key: String,
    db_manager: State<'_, DatabaseManager>,
) -> Result<(), String> {
    let conn = db_manager
        .get_connection()
        .map_err(|e| format!("Failed to open database: {}", e))?;
    conn.execute(
        "DELETE FROM dashboard_calendar_assignments WHERE event_key = ?1",
        params![event_key],
    )
    .map_err(|e| format!("Failed to clear assignment: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_dashboard_raid_reservations(
    db_manager: State<'_, DatabaseManager>,
) -> Result<Vec<DashboardRaidReservation>, String> {
    let conn = db_manager
        .get_connection()
        .map_err(|e| format!("Failed to open database: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, char_id, content_id, difficulty, label, reserved_at, scheduled_at, recurring_weekly
             FROM dashboard_raid_reservations
             ORDER BY COALESCE(scheduled_at, reserved_at) ASC",
        )
        .map_err(|e| format!("Failed to prepare reservation query: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(DashboardRaidReservation {
                id: row.get(0)?,
                char_id: row.get(1)?,
                content_id: row.get(2)?,
                difficulty: row.get(3)?,
                label: row.get(4)?,
                reserved_at: row.get(5)?,
                scheduled_at: row.get(6)?,
                recurring_weekly: row.get::<_, i64>(7)? == 1,
            })
        })
        .map_err(|e| format!("Failed to read reservations: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map reservations: {}", e))
}

#[tauri::command]
pub fn save_dashboard_raid_reservation(
    input: DashboardRaidReservationInput,
    db_manager: State<'_, DatabaseManager>,
) -> Result<DashboardRaidReservation, String> {
    let id = format!("{}:{}:{}", input.char_id, input.content_id, input.difficulty);
    let now = chrono::Utc::now().timestamp_millis();
    let conn = db_manager
        .get_connection()
        .map_err(|e| format!("Failed to open database: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO dashboard_raid_reservations
         (id, char_id, content_id, difficulty, label, reserved_at, scheduled_at, recurring_weekly)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            input.char_id,
            input.content_id,
            input.difficulty,
            input.label,
            now,
            input.scheduled_at,
            if input.recurring_weekly { 1 } else { 0 }
        ],
    )
    .map_err(|e| format!("Failed to save reservation: {}", e))?;

    Ok(DashboardRaidReservation {
        id,
        char_id: input.char_id,
        content_id: input.content_id,
        difficulty: input.difficulty,
        label: input.label,
        reserved_at: now,
        scheduled_at: input.scheduled_at,
        recurring_weekly: input.recurring_weekly,
    })
}

#[tauri::command]
pub fn clear_dashboard_raid_reservation(
    char_id: i64,
    content_id: String,
    difficulty: String,
    db_manager: State<'_, DatabaseManager>,
) -> Result<(), String> {
    let id = format!("{}:{}:{}", char_id, content_id, difficulty);
    let conn = db_manager
        .get_connection()
        .map_err(|e| format!("Failed to open database: {}", e))?;
    conn.execute("DELETE FROM dashboard_raid_reservations WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to clear reservation: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn cleanup_dashboard_raid_reservations(
    now: i64,
    db_manager: State<'_, DatabaseManager>,
) -> Result<(), String> {
    let conn = db_manager
        .get_connection()
        .map_err(|e| format!("Failed to open database: {}", e))?;
    conn.execute(
        "DELETE FROM dashboard_raid_reservations
         WHERE recurring_weekly = 0 AND scheduled_at IS NOT NULL AND scheduled_at < ?1",
        params![now],
    )
    .map_err(|e| format!("Failed to cleanup reservations: {}", e))?;
    Ok(())
}
