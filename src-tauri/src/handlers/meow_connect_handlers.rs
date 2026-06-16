use rusqlite::params;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeowConnectGroupRaidTagInput {
    pub char_id: i64,
    pub content_id: String,
    pub group_id: String,
    pub group_tag: String,
    pub group_name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeowConnectClearHintInput {
    pub char_id: i64,
    pub content_id: String,
    pub gate: String,
    pub difficulty: Option<String>,
    pub completed_at: i64,
    pub source_owner_name: Option<String>,
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

// Temporarily disabled due to Supabase realtime message limits
// #[tauri::command]
// pub async fn replace_meow_connect_group_raid_tags(
//     assignments: Vec<MeowConnectGroupRaidTagInput>,
//     db_manager: State<'_, crate::database::DatabaseManager>,
// ) -> Result<(), String> {
//     let mut conn = db_manager
//         .pool
//         .get()
//         .map_err(|e| format!("Database connection failed: {}", e))?;
//     let tx = conn.transaction().map_err(|e| e.to_string())?;
//     let now = chrono::Utc::now().timestamp_millis();
//
//     tx.execute("DELETE FROM meow_group_raid_tags", [])
//         .map_err(|e| e.to_string())?;
//
//     for assignment in assignments {
//         let group_tag = assignment.group_tag.trim().to_uppercase();
//         if group_tag.is_empty() {
//             continue;
//         }
//
//         tx.execute(
//             "INSERT OR REPLACE INTO meow_group_raid_tags
//                 (char_id, content_id, group_id, group_tag, group_name, updated_at)
//              VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
//             params![
//                 assignment.char_id,
//                 assignment.content_id,
//                 assignment.group_id,
//                 group_tag,
//                 assignment.group_name.trim(),
//                 now
//             ],
//         )
//         .map_err(|e| e.to_string())?;
//     }
//
//     tx.commit().map_err(|e| e.to_string())?;
//     Ok(())
// }

#[tauri::command]
pub async fn apply_meow_connect_clear_hints(
    hints: Vec<MeowConnectClearHintInput>,
    db_manager: State<'_, crate::database::DatabaseManager>,
) -> Result<i64, String> {
    let conn = db_manager
        .pool
        .get()
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let weekly_reset_ms = conn
        .query_row("SELECT last_weekly_reset FROM app_state LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap_or(0);

    let mut applied = 0i64;
    for hint in hints {
        if hint.char_id <= 0 || hint.content_id.trim().is_empty() || hint.gate.trim().is_empty() {
            continue;
        }
        if hint.completed_at > 0 && hint.completed_at < weekly_reset_ms {
            continue;
        }

        let roster_id = match conn.query_row(
            "SELECT roster_id
             FROM conf_character
             WHERE char_id = ?1
               AND COALESCE(meow_connect_enabled, 0) = 1
               AND COALESCE(removed_from_roster, 0) = 0",
            params![hint.char_id],
            |row| row.get::<_, String>(0),
        ) {
            Ok(roster_id) => roster_id,
            Err(_) => continue,
        };

        let content_id = hint.content_id.trim().to_string();
        let gate = normalize_gate_label(&hint.gate);
        let session_id = format!("{}_{}", content_id, gate);
        let timestamp = if hint.completed_at > 0 {
            hint.completed_at
        } else {
            chrono::Utc::now().timestamp_millis()
        };
        let details = hint
            .difficulty
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("meow_connect")
            .to_string();

        let existing = conn
            .query_row(
                "SELECT rowid, COALESCE(is_completed, 0), COALESCE(completion_source, 'manual'), COALESCE(timestamp, 0)
                 FROM completion_status
                 WHERE char_id = ?1 AND content_id = ?2 AND session_id = ?3
                 ORDER BY timestamp DESC, rowid DESC
                 LIMIT 1",
                params![hint.char_id, &content_id, &session_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .ok();

        if let Some((rowid, is_completed, _source, _existing_timestamp)) = existing {
            if is_completed == 1 {
                continue;
            }

            let changed = conn
                .execute(
                    "UPDATE completion_status
                     SET is_completed = 1,
                         timestamp = ?1,
                         completion_source = 'meow_connect',
                         details = ?2
                     WHERE rowid = ?3",
                    params![timestamp, details, rowid],
                )
                .map_err(|e| e.to_string())?;
            if changed > 0 {
                applied += 1;
            }
            continue;
        }

        let changed = conn
            .execute(
                "INSERT INTO completion_status
                    (roster_id, char_id, content_id, is_completed, timestamp, session_id, completion_source, details)
                 VALUES (?1, ?2, ?3, 1, ?4, ?5, 'meow_connect', ?6)",
                params![roster_id, hint.char_id, content_id, timestamp, session_id, details],
            )
            .map_err(|e| e.to_string())?;
        if changed > 0 {
            applied += 1;
            if let Some(owner_name) = hint.source_owner_name.as_deref() {
                crate::log_info!(
                    "Applied MeowConnect clear hint from {} for char_id={}, content_id={}, gate={}",
                    owner_name,
                    hint.char_id,
                    hint.content_id,
                    gate
                );
            }
        }
    }

    Ok(applied)
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

fn normalize_gate_label(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "Gate 1".to_string();
    }

    let lower = trimmed.to_lowercase();
    if lower.starts_with("gate ") {
        return format!("Gate {}", trimmed[5..].trim());
    }
    if let Some(number) = lower.strip_prefix('g') {
        if number.trim().chars().all(|ch| ch.is_ascii_digit()) {
            return format!("Gate {}", number.trim());
        }
    }
    if trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        return format!("Gate {}", trimmed);
    }

    trimmed.to_string()
}
