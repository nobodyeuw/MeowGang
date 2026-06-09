use anyhow::Result;
use chrono::{Datelike, Utc, Weekday};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Todo-specific data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoMatrixResponse {
    pub characters: Vec<TodoCharacter>,
    pub daily_tasks: Vec<TodoTask>,
    pub roster_tasks: Vec<TodoTask>,
    pub weekly_tasks: Vec<TodoTask>,
    pub raids: Vec<TodoRaid>,
    pub character_states: Option<std::collections::HashMap<String, CharacterTaskState>>,
    pub rested_entries: Option<Vec<(i64, String, i64)>>,
    pub todo_entries: Option<Vec<(i64, String, bool)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoCharacter {
    pub id: i64,
    pub name: String,
    pub class: String,
    pub ilvl: f64,
    pub combat_power: f64,
    pub earns_gold: bool,
    pub display_order: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoTask {
    pub id: String,
    pub name: String,
    pub category: String,
    pub reset_schedule: String,
    pub logic_type: String,
    pub max_rest_value: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterTaskState {
    pub tracked: bool,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosterEventProgress {
    pub task_id: String,
    pub completed_this_week: i64,
    pub weekly_limit: i64,
    pub completed_today: bool,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoRaid {
    pub id: String,
    pub name: String,
    pub difficulty: String,
    pub gates: Vec<TodoRaidGate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoRaidGate {
    pub gate: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRaidState {
    pub raid_id: String,
    pub gates: Vec<RaidGateState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidGateState {
    pub gate: String,
    pub cleared: bool,
    pub clear_time: Option<String>,
}

pub struct RaidGateCompletionDetails {
    pub completed: bool,
    pub actual_difficulty: Option<String>,
}

pub struct TodoRepository {
    pub(crate) pool: Arc<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>,
}

impl TodoRepository {
    pub fn new(pool: Arc<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>) -> Self {
        Self { pool }
    }

    /// Tasks whose completion is represented once for the whole roster.
    fn is_roster_wide_task_id(task_id: &str) -> bool {
        matches!(task_id, "gate" | "boss" | "event_argeos_winter" | "ship_shop")
    }

    /// Roster-wide tasks that can be completed multiple times per weekly window.
    fn is_roster_event_task_id(task_id: &str) -> bool {
        matches!(task_id, "gate" | "boss" | "event_argeos_winter")
    }

    /// Weekly cap for roster event progress counters.
    fn roster_event_weekly_limit(task_id: &str) -> i64 {
        match task_id {
            "event_argeos_winter" => 3,
            _ => 1,
        }
    }

    /// Whether a roster event should currently be available for manual completion.
    fn roster_event_available_now(task_id: &str) -> bool {
        match task_id {
            "gate" | "boss" => {
                let now = Utc::now();
                let mut game_day_start = now.date_naive().and_hms_opt(10, 0, 0).unwrap().and_utc();
                if now < game_day_start {
                    game_day_start -= chrono::Duration::days(1);
                }

                if now >= game_day_start + chrono::Duration::hours(17) {
                    return false;
                }

                match task_id {
                    "gate" => matches!(
                        game_day_start.weekday(),
                        Weekday::Mon | Weekday::Thu | Weekday::Sat | Weekday::Sun
                    ),
                    "boss" => matches!(game_day_start.weekday(), Weekday::Tue | Weekday::Fri | Weekday::Sun),
                    _ => false,
                }
            }
            _ => true,
        }
    }

    /// Most recent daily reset boundary in milliseconds.
    fn current_daily_reset_ms() -> i64 {
        let now = chrono::Utc::now();
        let today_reset = now.date_naive().and_hms_opt(10, 0, 0).unwrap().and_utc();
        if now >= today_reset {
            today_reset.timestamp_millis()
        } else {
            (today_reset - chrono::Duration::days(1)).timestamp_millis()
        }
    }

    /// Most recent weekly reset boundary in milliseconds.
    fn current_weekly_reset_ms() -> i64 {
        use chrono::Datelike;

        let now = chrono::Utc::now();
        let mut reset_date = now.date_naive();
        while reset_date.weekday().num_days_from_monday() != 2 {
            reset_date = reset_date - chrono::Duration::days(1);
        }

        let reset_time = reset_date.and_hms_opt(10, 0, 0).unwrap().and_utc();
        if now < reset_time {
            (reset_time - chrono::Duration::weeks(1)).timestamp_millis()
        } else {
            reset_time.timestamp_millis()
        }
    }

    /// Finds the existing row used to store a simple roster-wide completion.
    ///
    /// Older rows may point at a character that has since been removed from
    /// the roster, so roster-wide tasks must be located by roster/content
    /// instead of by the current first visible character.
    fn get_roster_task_completion_rowid(
        conn: &rusqlite::Connection,
        roster_id: &str,
        task_id: &str,
    ) -> Result<Option<i64>> {
        let rowid = conn
            .query_row(
                "SELECT rowid
                 FROM completion_status
                 WHERE roster_id = ?1
                   AND content_id = ?2
                   AND session_id IS NULL
                 ORDER BY timestamp DESC, rowid DESC
                 LIMIT 1",
                params![roster_id, task_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        Ok(rowid)
    }

    /// Reads current completion state for a roster-wide task.
    fn get_roster_task_completed_with_conn(
        conn: &rusqlite::Connection,
        roster_id: &str,
        task_id: &str,
    ) -> Result<bool> {
        let completed = conn
            .query_row(
                "SELECT is_completed
                 FROM completion_status
                 WHERE roster_id = ?1
                   AND content_id = ?2
                   AND session_id IS NULL
                 ORDER BY timestamp DESC, rowid DESC
                 LIMIT 1",
                params![roster_id, task_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);

        Ok(completed == 1)
    }

    /// Loads To Do data owned by the backend.
    ///
    /// Task/raid definitions are intentionally sparse here; the frontend fills
    /// labels, order, rewards, and ilvl rules from `src/lib/data`.
    pub fn get_todo_matrix(&self, roster_id: &str) -> Result<TodoMatrixResponse> {
        let conn = self.pool.get()?;

        // Get characters for this roster
        let mut stmt = conn.prepare(
            "SELECT char_id, char_name, class_id, item_level, combat_power, earns_gold, display_order 
             FROM conf_character
             WHERE roster_id = ?1 AND COALESCE(removed_from_roster, 0) = 0
             ORDER BY CAST(display_order AS INTEGER), char_name",
        )?;

        let character_iter = stmt.query_map([roster_id], |row| {
            Ok(TodoCharacter {
                id: row.get(0)?,
                name: row.get(1)?,
                class: row.get(2)?,
                ilvl: row.get(3)?,
                combat_power: row.get(4)?,
                earns_gold: row.get(5)?,
                display_order: row.get(6)?,
            })
        })?;

        let mut characters = Vec::new();
        for character in character_iter {
            characters.push(character?);
        }

        // Load all tracked tasks from conf_tracking. Roster-wide tasks use
        // `char_id = NULL`; character rows and raid rows keep a concrete id.
        let mut todo_stmt =
            conn.prepare("SELECT char_id, content_id, is_tracked FROM conf_tracking WHERE roster_id = ?1")?;

        let todo_iter = todo_stmt.query_map([roster_id], |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?, // char_id
                row.get::<_, String>(1)?,      // content_id
                row.get::<_, i64>(2)? == 1,    // is_tracked
            ))
        })?;

        let todo_entries: Vec<(Option<i64>, String, bool)> = todo_iter.filter_map(Result::ok).collect();

        // Load rested values
        let mut rested_stmt = conn.prepare(
            "SELECT char_id, content_id, current_value 
             FROM rested_values 
             WHERE char_id IN (
                 SELECT char_id FROM conf_character
                 WHERE roster_id = ?1 AND COALESCE(removed_from_roster, 0) = 0
             )",
        )?;

        let rested_iter = rested_stmt.query_map([roster_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,    // char_id
                row.get::<_, String>(1)?, // content_id
                row.get::<_, i64>(2)?,    // current_value
            ))
        })?;

        let rested_entries: Vec<(i64, String, i64)> = rested_iter.filter_map(Result::ok).collect();

        // Load raid ids/gates from conf_raid. Display names and rewards remain
        // frontend-owned source-of-truth data.
        let mut raid_map = std::collections::HashMap::new();
        for character in &characters {
            let mut raid_stmt = conn.prepare("SELECT DISTINCT content_id FROM conf_raid WHERE char_id = ?1")?;

            let raid_iter = raid_stmt.query_map([character.id], |row| {
                Ok(row.get::<_, String>(0)?) // content_id
            })?;

            for raid_result in raid_iter {
                let content_id = raid_result?;
                if !raid_map.contains_key(&content_id) {
                    // Load gates for this raid from conf_raid
                    let mut gate_stmt =
                        conn.prepare("SELECT gate FROM conf_raid WHERE char_id = ?1 AND content_id = ?2")?;

                    let gate_iter = gate_stmt.query_map(params![character.id, content_id.clone()], |row| {
                        Ok(row.get::<_, String>(0)?) // gate
                    })?;

                    let mut gates = Vec::new();
                    for gate_result in gate_iter {
                        gates.push(gate_result?);
                    }

                    // Use content_id as name since content_name doesn't exist
                    raid_map.insert(
                        content_id.clone(),
                        TodoRaid {
                            id: content_id.clone(),
                            name: content_id,                 // Use content_id as name
                            difficulty: "normal".to_string(), // Default difficulty
                            gates: gates
                                .into_iter()
                                .map(|gate| TodoRaidGate {
                                    gate: gate.clone(),
                                    name: gate,
                                })
                                .collect(),
                        },
                    );
                }
            }
        }

        let raids: Vec<TodoRaid> = raid_map.into_values().collect();

        // Create character states using todo_entries and rested_entries (like tracking_repository)
        let mut character_states = std::collections::HashMap::new();

        for character in &characters {
            for (char_id, content_id, is_tracked) in &todo_entries {
                if Self::is_roster_wide_task_id(content_id) {
                    continue;
                }

                // Create state for character-specific tasks
                if char_id == &Some(character.id) {
                    let completed = self.get_task_completed(character.id, content_id).unwrap_or(false);

                    let state = CharacterTaskState {
                        tracked: *is_tracked,
                        completed,
                    };

                    character_states.insert(format!("{}_{}", character.id, content_id), state);
                }
            }
        }

        for task_id in ["gate", "boss"] {
            let tracked = todo_entries
                .iter()
                .any(|(char_id, content_id, is_tracked)| {
                    content_id == task_id && char_id.is_none() && *is_tracked
                });

            let progress = self.get_roster_event_progress(roster_id, task_id)?;
            let completed = progress.completed_today || progress.completed_this_week >= progress.weekly_limit;

            for roster_char in &characters {
                character_states.insert(
                    format!("{}_{}", roster_char.id, task_id),
                    CharacterTaskState { tracked, completed },
                );
            }
        }

        for task_id in ["event_argeos_winter"] {
            let tracked = todo_entries
                .iter()
                .any(|(char_id, content_id, is_tracked)| {
                    content_id == task_id && char_id.is_none() && *is_tracked
                });

            let progress = self.get_roster_event_progress(roster_id, task_id)?;
            for roster_char in &characters {
                character_states.insert(
                    format!("{}_{}", roster_char.id, task_id),
                    CharacterTaskState {
                        tracked,
                        completed: progress.completed_this_week >= progress.weekly_limit,
                    },
                );
            }
        }

        for task_id in ["ship_shop"] {
            let tracked = todo_entries
                .iter()
                .any(|(char_id, content_id, is_tracked)| {
                    content_id == task_id && char_id.is_none() && *is_tracked
                });

            let completed = Self::get_roster_task_completed_with_conn(&conn, roster_id, task_id)?;

            for roster_char in &characters {
                character_states.insert(
                    format!("{}_{}", roster_char.id, task_id),
                    CharacterTaskState { tracked, completed },
                );
            }
        }

        let expanded_todo_entries: Vec<(i64, String, bool)> = todo_entries
            .iter()
            .flat_map(|(char_id, content_id, is_tracked)| {
                if let Some(char_id) = char_id {
                    vec![(*char_id, content_id.clone(), *is_tracked)]
                } else {
                    characters
                        .iter()
                        .map(|character| (character.id, content_id.clone(), *is_tracked))
                        .collect()
                }
            })
            .collect();

        // Create matrix with basic data - frontend will handle task definitions
        let matrix = TodoMatrixResponse {
            characters,
            daily_tasks: vec![],  // Frontend will populate from GAME_TASKS
            roster_tasks: vec![], // Frontend will populate from GAME_TASKS
            weekly_tasks: vec![], // Frontend will populate from GAME_TASKS
            raids,
            character_states: Some(character_states),
            rested_entries: Some(rested_entries),
            todo_entries: Some(expanded_todo_entries),
        };

        Ok(matrix)
    }

    /// Reads task completion with reset-window awareness for daily rested tasks.
    pub fn get_task_completed(&self, char_id: i64, task_id: &str) -> Result<bool> {
        let conn = self.pool.get()?;

        if task_id == "chaos" || task_id == "guardian" {
            let daily_reset_ms = Self::current_daily_reset_ms();
            let completed = conn
                .query_row(
                    "SELECT COALESCE(MAX(is_completed), 0)
                     FROM completion_status
                     WHERE char_id = ?1
                       AND content_id = ?2
                       AND timestamp >= ?3",
                    params![char_id, task_id, daily_reset_ms],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0);

            return Ok(completed == 1);
        }

        // First try to find regular task entry
        let mut stmt = conn.prepare(
            "SELECT is_completed
             FROM completion_status
             WHERE char_id = ?1
               AND content_id = ?2
               AND session_id IS NOT NULL
             ORDER BY timestamp DESC, rowid DESC
             LIMIT 1",
        )?;

        match stmt.query_row(params![char_id, task_id], |row| row.get::<_, i64>(0)) {
            Ok(result) => Ok(result == 1),
            Err(_) => {
                // If not found, try roster task entry (session_id IS NULL)
                let mut stmt = conn.prepare(
                    "SELECT is_completed
                     FROM completion_status
                     WHERE char_id = ?1
                       AND content_id = ?2
                       AND session_id IS NULL
                     ORDER BY timestamp DESC, rowid DESC
                     LIMIT 1",
                )?;

                match stmt.query_row(params![char_id, task_id], |row| row.get::<_, i64>(0)) {
                    Ok(result) => Ok(result == 1),
                    Err(_) => Ok(false), // Not found anywhere
                }
            }
        }
    }

    /// Gets the configured difficulty for a raid gate from Settings > Raids.
    pub fn get_raid_gate_difficulty(&self, char_id: i64, raid_id: &str, gate_id: &str) -> Result<Option<String>> {
        let conn = self.pool.get()?;

        let mut stmt =
            conn.prepare("SELECT difficulty FROM conf_raid WHERE char_id = ?1 AND content_id = ?2 AND gate = ?3")?;

        let result = stmt.query_row(params![char_id, raid_id, gate_id], |row| {
            Ok(Some(row.get::<_, String>(0)?))
        });

        match result {
            Ok(difficulty) => Ok(difficulty),
            Err(_) => Ok(None), // Return None if no configuration found
        }
    }

    /// Reads whether a specific raid gate has been cleared this weekly cycle.
    pub fn get_raid_gate_completed(
        &self,
        char_id: i64,
        raid_id: &str,
        gate_id: &str,
        _difficulty: &str,
    ) -> Result<Option<bool>> {
        let conn = self.pool.get()?;

        // Exact session_id match only - no LIKE fallback that would mark all
        // gates done when just one gate of the raid is completed.
        let base_session_id = format!("{}_{}", raid_id, gate_id);

        let mut stmt = conn.prepare(
            "SELECT is_completed FROM completion_status \
            WHERE char_id = ?1 AND content_id = ?2 AND session_id = ?3 LIMIT 1",
        )?;

        match stmt.query_row(params![char_id, raid_id, &base_session_id], |row| {
            Ok(row.get::<_, i64>(0)?)
        }) {
            Ok(completed) => Ok(Some(completed == 1)),
            Err(_) => Ok(Some(false)),
        }
    }

    /// Reads one raid gate clear state and the stored clear difficulty.
    pub fn get_raid_gate_completion_details(
        &self,
        char_id: i64,
        raid_id: &str,
        gate_id: &str,
    ) -> Result<RaidGateCompletionDetails> {
        let conn = self.pool.get()?;
        let base_session_id = format!("{}_{}", raid_id, gate_id);

        let mut stmt = conn.prepare(
            "SELECT is_completed, details FROM completion_status \
             WHERE char_id = ?1 AND content_id = ?2 AND session_id = ?3 LIMIT 1",
        )?;

        match stmt.query_row(params![char_id, raid_id, &base_session_id], |row| {
            Ok(RaidGateCompletionDetails {
                completed: row.get::<_, i64>(0)? == 1,
                actual_difficulty: row.get::<_, Option<String>>(1)?,
            })
        }) {
            Ok(details) => Ok(details),
            Err(_) => Ok(RaidGateCompletionDetails {
                completed: false,
                actual_difficulty: None,
            }),
        }
    }

    /// Writes a character task completion row and handles roster-wide delegation.
    pub fn set_task_completed(&self, char_id: i64, task_id: &str, completed: bool) -> Result<()> {
        let conn = self.pool.get()?;

        // Get roster_id for this character
        let mut stmt = conn.prepare("SELECT roster_id FROM conf_character WHERE char_id = ?1")?;

        let roster_id: String = stmt.query_row([char_id], |row| row.get(0))?;

        if Self::is_roster_wide_task_id(task_id) {
            drop(stmt);
            drop(conn);
            return self.set_roster_task_completed(&roster_id, task_id, completed);
        }

        let completed_value = if completed { 1i64 } else { 0i64 };
        let timestamp = chrono::Utc::now().timestamp_millis();

        // Check if entry already exists
        let mut stmt =
            conn.prepare("SELECT is_completed FROM completion_status WHERE char_id = ?1 AND content_id = ?2")?;

        match stmt.query_row(params![char_id, task_id], |row| Ok(row.get::<_, i64>(0)?)) {
            Ok(_existing_completed) => {
                // Update existing entry
                conn.execute(
                    "UPDATE completion_status SET is_completed = ?1, timestamp = ?2 WHERE char_id = ?3 AND content_id = ?4",
                    params![completed_value, timestamp, char_id, task_id]
                )?;
            }
            Err(_) => {
                // Insert new entry
                let session_id = format!("{}_{:x}", chrono::Utc::now().timestamp_millis(), rand::random::<u32>());

                conn.execute(
                    "INSERT INTO completion_status (roster_id, char_id, content_id, is_completed, timestamp, session_id) 
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![roster_id, char_id, task_id, completed_value, timestamp, session_id]
                )?;
            }
        }

        // Handle rested value consumption for chaos/guardian on completion
        if completed && (task_id == "chaos" || task_id == "guardian") {
            self.consume_rested_on_completion(char_id, task_id)?;
        }

        Ok(())
    }

    /// Writes an externally synced character task completion at the source timestamp.
    pub fn set_character_task_completed_at(
        &self,
        char_id: i64,
        task_id: &str,
        completed: bool,
        timestamp: i64,
        source: &str,
        details: Option<&str>,
    ) -> Result<bool> {
        let conn = self.pool.get()?;

        let roster_id: String = conn.query_row(
            "SELECT roster_id FROM conf_character WHERE char_id = ?1",
            [char_id],
            |row| row.get(0),
        )?;

        let daily_reset_ms = Self::current_daily_reset_ms();

        let already_completed = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM completion_status
                 WHERE char_id = ?1
                   AND content_id = ?2
                   AND is_completed = 1
                   AND timestamp >= ?3",
                params![char_id, task_id, daily_reset_ms],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        if completed && already_completed {
            return Ok(false);
        }

        let completed_value = if completed { 1i64 } else { 0i64 };
        let session_id = format!("{}_{}", task_id, daily_reset_ms);

        let changed = conn.execute(
            "INSERT INTO completion_status
                (roster_id, char_id, content_id, is_completed, completion_source, timestamp, details, session_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                roster_id,
                char_id,
                task_id,
                completed_value,
                source,
                timestamp,
                details,
                session_id
            ],
        )? > 0;

        if completed && changed && (task_id == "chaos" || task_id == "guardian") {
            self.consume_rested_on_completion(char_id, task_id)?;
        }

        Ok(changed)
    }

    /// Consumes 20 rested points when chaos/guardian is completed, if available.
    fn consume_rested_on_completion(&self, char_id: i64, task_id: &str) -> Result<()> {
        let conn = self.pool.get()?;

        // Get roster_id for this character
        let roster_id: String = conn.query_row(
            "SELECT roster_id FROM conf_character WHERE char_id = ?1",
            [char_id],
            |row| row.get(0),
        )?;

        // Get current rested value
        let current_rested = conn
            .query_row(
                "SELECT current_value FROM rested_values WHERE char_id = ?1 AND content_id = ?2",
                params![char_id, task_id],
                |row| Ok(row.get::<_, i64>(0)?),
            )
            .unwrap_or(0);

        // Only consume if we have at least 20 points
        if current_rested >= 20 {
            let new_rested = current_rested - 20;
            let last_updated = chrono::Utc::now().timestamp_millis();

            conn.execute(
                "INSERT OR REPLACE INTO rested_values (roster_id, char_id, content_id, current_value, last_updated) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![roster_id, char_id, task_id, new_rested, last_updated],
            )?;

            crate::log_debug!(
                "Consumed 20 rested points for {} (character {}): {} -> {}",
                task_id,
                char_id,
                current_rested,
                new_rested
            );
        }

        Ok(())
    }

    /// Writes a single raid gate clear into completion_status.
    pub fn set_raid_gate_completed(
        &self,
        char_id: i64,
        task_id: &str,
        completed: bool,
        difficulty: &str,
        gate_id: &str,
    ) -> Result<()> {
        let conn = self.pool.get()?;

        // Get roster_id for this character
        let mut stmt = conn.prepare("SELECT roster_id FROM conf_character WHERE char_id = ?1")?;

        let roster_id: String = stmt.query_row([char_id], |row| row.get(0))?;

        let completed_value = if completed { 1i64 } else { 0i64 };
        let timestamp = chrono::Utc::now().timestamp_millis();

        // Create base session_id without difficulty for consistent identification
        let base_session_id = format!("{}_{}", task_id, gate_id);

        // Check if entry already exists for this specific gate
        let mut stmt = conn.prepare(
            "SELECT is_completed FROM completion_status WHERE char_id = ?1 AND content_id = ?2 AND session_id = ?3",
        )?;

        match stmt.query_row(params![char_id, task_id, &base_session_id], |row| {
            Ok(row.get::<_, i64>(0)?)
        }) {
            Ok(_existing_completed) => {
                // Update existing entry
                conn.execute(
                    "UPDATE completion_status SET is_completed = ?1, timestamp = ?2, details = ?3 WHERE char_id = ?4 AND content_id = ?5 AND session_id = ?6",
                    params![completed_value, timestamp, difficulty, char_id, task_id, &base_session_id]
                )?;
            }
            Err(_) => {
                // Insert new entry
                conn.execute(
                    "INSERT INTO completion_status (roster_id, char_id, content_id, is_completed, timestamp, session_id, completion_source, details) 
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![roster_id, char_id, task_id, completed_value, timestamp, &base_session_id, "manual", difficulty]
                )?;
            }
        }

        Ok(())
    }

    /// Writes completion state for a roster-wide task.
    pub fn set_roster_task_completed(&self, roster_id: &str, task_id: &str, completed: bool) -> Result<()> {
        if Self::is_roster_event_task_id(task_id) {
            self.set_roster_event_completed(roster_id, task_id, completed)?;
            return Ok(());
        }

        let mut conn = self.pool.get()?;

        let completed_value = if completed { 1i64 } else { 0i64 };
        let timestamp = chrono::Utc::now().timestamp_millis();

        let tx = conn.transaction()?;
        if let Some(rowid) = Self::get_roster_task_completion_rowid(&tx, roster_id, task_id)? {
            tx.execute(
                "UPDATE completion_status
                 SET is_completed = ?1, timestamp = ?2, completion_source = 'manual'
                 WHERE rowid = ?3",
                params![completed_value, timestamp, rowid],
            )?;
        } else {
            tx.execute(
                "INSERT INTO completion_status
                    (roster_id, char_id, content_id, is_completed, timestamp, completion_source, session_id)
                 VALUES (?1, NULL, ?2, ?3, ?4, 'manual', NULL)",
                params![roster_id, task_id, completed_value, timestamp],
            )?;
        }

        tx.commit()?;

        Ok(())
    }

    /// Returns weekly/daily progress for roster event tasks.
    pub fn get_roster_event_progress(&self, roster_id: &str, task_id: &str) -> Result<RosterEventProgress> {
        let conn = self.pool.get()?;
        let weekly_reset_ms = Self::current_weekly_reset_ms();
        let daily_reset_ms = Self::current_daily_reset_ms();
        let weekly_limit = Self::roster_event_weekly_limit(task_id);

        let completed_this_week = conn
            .query_row(
                "SELECT COUNT(DISTINCT session_id)
                 FROM completion_status
                 WHERE roster_id = ?1
                   AND content_id = ?2
                   AND is_completed = 1
                   AND timestamp >= ?3",
                params![roster_id, task_id, weekly_reset_ms],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);

        let completed_today = conn
            .query_row(
                "SELECT COUNT(DISTINCT session_id)
                 FROM completion_status
                 WHERE roster_id = ?1
                   AND content_id = ?2
                   AND is_completed = 1
                   AND timestamp >= ?3",
                params![roster_id, task_id, daily_reset_ms],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        Ok(RosterEventProgress {
            task_id: task_id.to_string(),
            completed_this_week,
            weekly_limit,
            completed_today,
            available: completed_this_week < weekly_limit && !completed_today && Self::roster_event_available_now(task_id),
        })
    }

    /// Adds or toggles today's completion row for a roster event task.
    pub fn set_roster_event_completed(&self, roster_id: &str, task_id: &str, completed: bool) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        let daily_reset_ms = Self::current_daily_reset_ms();
        let weekly_reset_ms = Self::current_weekly_reset_ms();
        let timestamp = chrono::Utc::now().timestamp_millis();
        let session_id = format!("{}_{}", task_id, daily_reset_ms);

        let today_rowids = {
            let mut stmt = tx.prepare(
                "SELECT rowid
                 FROM completion_status
                 WHERE roster_id = ?1
                   AND content_id = ?2
                   AND session_id = ?3
                   AND timestamp >= ?4
                 ORDER BY rowid ASC",
            )?;
            let rowids = stmt
                .query_map(params![roster_id, task_id, session_id, weekly_reset_ms], |row| {
                    row.get::<_, i64>(0)
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rowids
        };

        let existing_today_rowid = today_rowids.first().copied();
        for duplicate_rowid in today_rowids.iter().skip(1) {
            tx.execute(
                "DELETE FROM completion_status WHERE rowid = ?1",
                params![duplicate_rowid],
            )?;
        }

        if completed {
            let completed_this_week = tx
                .query_row(
                    "SELECT COUNT(DISTINCT session_id)
                     FROM completion_status
                     WHERE roster_id = ?1
                       AND content_id = ?2
                       AND is_completed = 1
                       AND timestamp >= ?3",
                    params![roster_id, task_id, weekly_reset_ms],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0);
            if existing_today_rowid.is_none() && completed_this_week >= Self::roster_event_weekly_limit(task_id) {
                tx.commit()?;
                return Ok(());
            }

            if let Some(rowid) = existing_today_rowid {
                tx.execute(
                    "UPDATE completion_status
                     SET is_completed = 1,
                         completion_source = 'manual',
                         timestamp = ?1,
                         details = 'manual event completion'
                     WHERE rowid = ?2",
                    params![timestamp, rowid],
                )?;
            } else {
                tx.execute(
                    "INSERT INTO completion_status
                        (roster_id, char_id, content_id, is_completed, completion_source, timestamp, details, session_id)
                     VALUES (?1, NULL, ?2, 1, 'manual', ?3, 'manual event completion', ?4)",
                    params![roster_id, task_id, timestamp, session_id],
                )?;
            }
        } else {
            if let Some(rowid) = existing_today_rowid {
                tx.execute(
                    "UPDATE completion_status
                     SET is_completed = 0,
                         completion_source = 'manual',
                         timestamp = ?1,
                         details = 'manual event completion'
                     WHERE rowid = ?2",
                    params![timestamp, rowid],
                )?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Rebuilds this week's manual event completions to match a requested count.
    pub fn set_roster_event_weekly_count(&self, roster_id: &str, task_id: &str, completed_count: i64) -> Result<()> {
        let weekly_limit = Self::roster_event_weekly_limit(task_id);
        if completed_count < 0 || completed_count > weekly_limit {
            anyhow::bail!("Roster event completion count must be between 0 and {}", weekly_limit);
        }

        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        let weekly_reset_ms = Self::current_weekly_reset_ms();
        let daily_reset_ms = Self::current_daily_reset_ms();
        let timestamp_base = if daily_reset_ms > weekly_reset_ms {
            daily_reset_ms - 1
        } else {
            daily_reset_ms
        };

        tx.execute(
            "DELETE FROM completion_status
             WHERE roster_id = ?1
               AND content_id = ?2
               AND timestamp >= ?3",
            params![roster_id, task_id, weekly_reset_ms],
        )?;

        for index in 0..completed_count {
            let event_timestamp = (timestamp_base - ((completed_count - index - 1) * 60_000)).max(weekly_reset_ms);
            let session_id = format!("{}_manual_{}_{}", task_id, weekly_reset_ms, index);

            tx.execute(
                "INSERT INTO completion_status
                    (roster_id, char_id, content_id, is_completed, completion_source, timestamp, details, session_id)
                 VALUES (?1, NULL, ?2, 1, 'manual', ?3, 'manual weekly event count', ?4)",
                params![roster_id, task_id, event_timestamp, session_id],
            )?;
        }

        tx.commit()?;
        Ok(())
    }
}
