use rusqlite::params;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeowConnectCharacterSnapshot {
    pub char_id: i64,
    pub char_name: String,
    pub roster_id: String,
    pub roster_name: String,
    pub class_id: String,
    pub item_level: f64,
    pub combat_power: f64,
    pub display_order: i64,
    pub earns_gold: bool,
    pub hide_from_dashboard: bool,
    pub meow_connect_enabled: bool,
    pub has_static_reservation: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeowConnectCompletionSnapshot {
    pub roster_id: String,
    pub char_id: i64,
    pub content_id: String,
    pub gate: Option<String>,
    pub difficulty: Option<String>,
    pub is_completed: bool,
    pub source: String,
    pub session_id: Option<String>,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeowConnectRaidReservationSnapshot {
    pub roster_id: String,
    pub char_id: i64,
    pub content_id: String,
    pub difficulty: String,
    pub reserved_for_static: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeowConnectLocalSnapshot {
    pub generated_at: i64,
    pub weekly_reset_ms: i64,
    pub characters: Vec<MeowConnectCharacterSnapshot>,
    pub completion_snapshots: Vec<MeowConnectCompletionSnapshot>,
    pub raid_reservations: Vec<MeowConnectRaidReservationSnapshot>,
}

#[tauri::command]
pub async fn get_meow_connect_local_snapshot(
    db_manager: State<'_, crate::database::DatabaseManager>,
) -> Result<MeowConnectLocalSnapshot, String> {
    let conn = db_manager
        .pool
        .get()
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let weekly_reset_ms = conn
        .query_row("SELECT last_weekly_reset FROM app_state LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap_or(0);

    let mut character_stmt = conn
        .prepare(
            "SELECT char_id,
                    COALESCE(char_name, ''),
                    roster_id,
                    COALESCE(roster_name, ''),
                    COALESCE(class_id, ''),
                    COALESCE(item_level, 0),
                    COALESCE(combat_power, 0),
                    COALESCE(display_order, '0'),
                    COALESCE(earns_gold, 0),
                    COALESCE(hide_from_dashboard, 0),
                    COALESCE(meow_connect_enabled, 0)
             FROM conf_character
             WHERE COALESCE(meow_connect_enabled, 0) = 1
               AND COALESCE(removed_from_roster, 0) = 0
             ORDER BY roster_display_order, CAST(display_order AS INTEGER), char_name, char_id",
        )
        .map_err(|e| e.to_string())?;

    let character_rows = character_stmt
        .query_map([], |row| {
            let display_order_raw: String = row.get(7)?;
            Ok(MeowConnectCharacterSnapshot {
                char_id: row.get(0)?,
                char_name: row.get(1)?,
                roster_id: row.get(2)?,
                roster_name: row.get(3)?,
                class_id: row.get(4)?,
                item_level: row.get(5)?,
                combat_power: row.get(6)?,
                display_order: display_order_raw.parse::<i64>().unwrap_or(0),
                earns_gold: row.get::<_, i64>(8)? == 1,
                hide_from_dashboard: row.get::<_, i64>(9)? == 1,
                meow_connect_enabled: row.get::<_, i64>(10)? == 1,
                has_static_reservation: false,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut characters = Vec::new();
    for character in character_rows {
        characters.push(character.map_err(|e| e.to_string())?);
    }

    let mut completion_stmt = conn
        .prepare(
            "SELECT roster_id,
                    char_id,
                    content_id,
                    COALESCE(is_completed, 0),
                    COALESCE(completion_source, 'manual'),
                    timestamp,
                    details,
                    session_id
             FROM completion_status
             WHERE (timestamp IS NULL OR timestamp >= ?1)
               AND char_id IN (
                 SELECT char_id FROM conf_character
                 WHERE COALESCE(meow_connect_enabled, 0) = 1
                   AND COALESCE(removed_from_roster, 0) = 0
               )
             ORDER BY timestamp DESC, rowid DESC",
        )
        .map_err(|e| e.to_string())?;

    let completion_rows = completion_stmt
        .query_map(params![weekly_reset_ms], |row| {
            let content_id: String = row.get(2)?;
            let session_id: Option<String> = row.get(7)?;
            Ok(MeowConnectCompletionSnapshot {
                roster_id: row.get(0)?,
                char_id: row.get(1)?,
                content_id: content_id.clone(),
                gate: infer_gate_from_session(&content_id, session_id.as_deref()),
                difficulty: row
                    .get::<_, Option<String>>(6)?
                    .filter(|value| !value.trim().is_empty()),
                is_completed: row.get::<_, i64>(3)? == 1,
                source: row.get(4)?,
                completed_at: row.get(5)?,
                session_id,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut completion_snapshots = Vec::new();
    for completion in completion_rows {
        completion_snapshots.push(completion.map_err(|e| e.to_string())?);
    }

    let mut reservation_stmt = conn
        .prepare(
            "SELECT roster_id,
                    char_id,
                    content_id,
                    difficulty,
                    MAX(COALESCE(reserved_for_static, 0))
             FROM conf_raid
             WHERE char_id IN (
               SELECT char_id FROM conf_character
               WHERE COALESCE(meow_connect_enabled, 0) = 1
                 AND COALESCE(removed_from_roster, 0) = 0
             )
             GROUP BY roster_id, char_id, content_id, difficulty",
        )
        .map_err(|e| e.to_string())?;

    let reservation_rows = reservation_stmt
        .query_map([], |row| {
            Ok(MeowConnectRaidReservationSnapshot {
                roster_id: row.get(0)?,
                char_id: row.get(1)?,
                content_id: row.get(2)?,
                difficulty: row.get(3)?,
                reserved_for_static: row.get::<_, i64>(4)? == 1,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut raid_reservations = Vec::new();
    for reservation in reservation_rows {
        raid_reservations.push(reservation.map_err(|e| e.to_string())?);
    }

    Ok(MeowConnectLocalSnapshot {
        generated_at: chrono::Utc::now().timestamp_millis(),
        weekly_reset_ms,
        characters,
        completion_snapshots,
        raid_reservations,
    })
}

fn infer_gate_from_session(content_id: &str, session_id: Option<&str>) -> Option<String> {
    let session_id = session_id?.trim();
    if session_id.is_empty() {
        return None;
    }

    let normalized = session_id
        .strip_prefix(content_id)
        .unwrap_or(session_id)
        .trim_start_matches('_')
        .trim_start_matches('-')
        .trim();

    if normalized.to_lowercase().starts_with("gate ") {
        return Some(normalized.to_string());
    }

    normalized
        .split('_')
        .find(|part| part.to_lowercase().starts_with("gate "))
        .map(|part| part.trim().to_string())
}
