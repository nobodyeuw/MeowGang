use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

pub struct TrackingRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl TrackingRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        Self { pool }
    }

    /// Returns true for tasks whose tracking state is controlled once per roster.
    fn is_roster_wide_task_id(content_id: &str) -> bool {
        matches!(content_id, "gate" | "boss" | "event_argeos_winter" | "ship_shop")
    }

    /// Loads the Settings > Tracking matrix state.
    ///
    /// `conf_tracking` owns visibility/tracking state for tasks and raid rows.
    /// Raid details such as difficulty, take-gold, box, and static reservation
    /// remain owned by `conf_raid`.
    pub fn get_tracking_config_matrix(&self, roster_id: &str) -> Result<crate::models::TodoConfigMatrix> {
        let conn = self.pool.get()?;

        // Get characters for this roster, ordered by display_order
        let mut char_stmt = conn.prepare(
            "SELECT char_id, char_name, class_id, item_level, combat_power, earns_gold, display_order
             FROM conf_character 
             WHERE roster_id = ?1 AND COALESCE(removed_from_roster, 0) = 0
             ORDER BY CAST(display_order AS INTEGER)",
        )?;

        let character_iter = char_stmt.query_map([roster_id], |row| {
            Ok(crate::models::CharacterMatrixInfo {
                char_id: row.get::<_, i64>(0)?,
                char_name: row.get::<_, String>(1)?,
                item_level: row.get::<_, f64>(3)?,
                combat_power: row.get::<_, f64>(4)?,
                class_id: row.get::<_, String>(2)?,
                earns_gold: row.get::<_, bool>(5)?,
                display_order: row.get::<_, String>(6)?.parse().unwrap_or(0),
            })
        })?;

        let characters: Vec<crate::models::CharacterMatrixInfo> = character_iter.filter_map(Result::ok).collect();

        // Get all conf_tracking entries for characters in this roster (both tasks and raids)
        let mut todo_stmt = conn.prepare(
            "SELECT char_id, content_id, is_tracked, COALESCE(lazy_daily, 0) FROM conf_tracking WHERE roster_id = ?1",
        )?;

        let todo_iter = todo_stmt.query_map([roster_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,      // char_id
                row.get::<_, String>(1)?,   // content_id
                row.get::<_, i64>(2)? == 1, // is_tracked
                row.get::<_, i64>(3)? == 1, // lazy_daily
            ))
        })?;

        let todo_entries: Vec<(i64, String, bool, bool)> = todo_iter.filter_map(Result::ok).collect();

        // Get rested values for characters in this roster
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

        // Create character states for each content_id
        let mut character_states = std::collections::HashMap::new();
        for char in &characters {
            for (char_id, content_id, is_tracked, lazy_daily) in &todo_entries {
                // Only create state if this entry belongs to the current character
                if *char_id == char.char_id {
                    // Get rested value if exists, default to 0
                    let current_value = rested_entries
                        .iter()
                        .find(|(rested_char_id, rested_content_id, _)| {
                            rested_char_id == char_id && rested_content_id == content_id
                        })
                        .map(|(_, _, value)| *value)
                        .unwrap_or(0);

                    character_states.insert(
                        (char.char_id, content_id.clone()),
                        crate::models::CharacterRaidState {
                            char_id: char.char_id,
                            content_id: content_id.clone(),
                            tracked: *is_tracked,
                            current_value: Some(current_value),
                            lazy_daily: Some(*lazy_daily),
                        },
                    );
                }
            }
        }

        // Create empty matrices - frontend will populate them with actual data
        let matrix = crate::models::TodoConfigMatrix {
            characters,
            daily_tasks: Vec::new(),
            roster_tasks: Vec::new(),
            weekly_tasks: Vec::new(),
            raids: Vec::new(),
            todo_entries: Some(
                todo_entries
                    .iter()
                    .map(|(char_id, content_id, is_tracked, _)| (*char_id, content_id.clone(), *is_tracked))
                    .collect(),
            ),
            rested_entries: Some(rested_entries),
            character_states: Some(character_states.into_iter().map(|(_, state)| state).collect()),
        };

        Ok(matrix)
    }

    /// Updates one tracking toggle in `conf_tracking`.
    ///
    /// Roster-wide tasks are expanded across all active characters so every
    /// matrix cell has a row, while completion state remains roster-level.
    pub fn update_tracking_config(&self, character_id: i64, content_id: &str, is_tracked: bool) -> Result<()> {
        let mut conn = self.pool.get()?;

        // First get roster_id for this character
        let roster_id: String = conn.query_row(
            "SELECT roster_id FROM conf_character WHERE char_id = ?1",
            params![character_id],
            |row| row.get(0),
        )?;

        let tracked_value = if is_tracked { 1 } else { 0 };

        if Self::is_roster_wide_task_id(content_id) {
            let tx = conn.transaction()?;
            tx.execute(
                "INSERT INTO conf_tracking (roster_id, char_id, content_id, is_tracked, lazy_daily)
                 SELECT roster_id, char_id, ?2, ?3, 0
                 FROM conf_character
                 WHERE roster_id = ?1
                 ON CONFLICT(char_id, content_id) DO UPDATE SET
                   roster_id = excluded.roster_id,
                   is_tracked = excluded.is_tracked",
                params![roster_id, content_id, tracked_value],
            )?;
            tx.commit()?;
            return Ok(());
        }

        conn.execute(
            "INSERT INTO conf_tracking (roster_id, char_id, content_id, is_tracked, lazy_daily)
             VALUES (?1, ?2, ?3, ?4, 0)
             ON CONFLICT(char_id, content_id) DO UPDATE SET
               roster_id = excluded.roster_id,
               is_tracked = excluded.is_tracked",
            params![roster_id, character_id, content_id, tracked_value],
        )?;

        Ok(())
    }

    /// Updates the "lazy daily" preference for a character task.
    pub fn update_lazy_daily_config(&self, character_id: i64, content_id: &str, lazy_daily: bool) -> Result<()> {
        let conn = self.pool.get()?;

        let roster_id: String = conn.query_row(
            "SELECT roster_id FROM conf_character WHERE char_id = ?1",
            params![character_id],
            |row| row.get(0),
        )?;

        conn.execute(
            "INSERT INTO conf_tracking (roster_id, char_id, content_id, is_tracked, lazy_daily)
             VALUES (?1, ?2, ?3, 1, ?4)
             ON CONFLICT(char_id, content_id) DO UPDATE SET
               roster_id = excluded.roster_id,
               lazy_daily = excluded.lazy_daily",
            params![roster_id, character_id, content_id, if lazy_daily { 1 } else { 0 }],
        )?;

        Ok(())
    }

    /// Stores the manual rested value shown in Settings > Tracking.
    pub fn save_rested_value(&self, character_id: i64, content_id: &str, rested_value: i64) -> Result<()> {
        let conn = self.pool.get()?;

        // First get roster_id for this character
        let roster_id: String = conn.query_row(
            "SELECT roster_id FROM conf_character WHERE char_id = ?1",
            params![character_id],
            |row| row.get(0),
        )?;

        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?
            .as_millis() as i64;

        conn.execute(
            "INSERT INTO rested_values (char_id, content_id, current_value, roster_id, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(char_id, content_id) DO UPDATE SET
               current_value = excluded.current_value,
               roster_id = excluded.roster_id,
               last_updated = excluded.last_updated",
            params![character_id, content_id, rested_value, roster_id, timestamp],
        )?;
        Ok(())
    }

    /// Compatibility command for older To Do tracking toggles.
    pub fn set_todo_tracked(&self, character_id: i64, content_id: &str, tracked: bool) -> Result<()> {
        let conn = self.pool.get()?;
        let roster_id: String = conn.query_row(
            "SELECT roster_id FROM conf_character WHERE char_id = ?1",
            params![character_id],
            |row| row.get(0),
        )?;

        conn.execute(
            "INSERT INTO conf_tracking (roster_id, char_id, content_id, is_tracked, lazy_daily)
             VALUES (?1, ?2, ?3, ?4, 0)
             ON CONFLICT(char_id, content_id) DO UPDATE SET
               roster_id = excluded.roster_id,
               is_tracked = excluded.is_tracked",
            params![roster_id, character_id, content_id, if tracked { 1 } else { 0 }],
        )?;
        Ok(())
    }

    /// Compatibility bulk save for older Tracking UI payloads.
    pub fn batch_update_task_status(
        &self,
        character_id: i64,
        task_updates: Vec<crate::models::TaskStatusStruct>,
    ) -> Result<()> {
        let mut conn = self.pool.get()?;
        let roster_id: String = conn.query_row(
            "SELECT roster_id FROM conf_character WHERE char_id = ?1",
            params![character_id],
            |row| row.get(0),
        )?;
        let tx = conn.transaction()?;

        for update in task_updates {
            tx.execute(
                "INSERT INTO conf_tracking 
                 (roster_id, char_id, content_id, is_tracked, lazy_daily)
                 VALUES (?1, ?2, ?3, ?4, 0)
                 ON CONFLICT(char_id, content_id) DO UPDATE SET
                   roster_id = excluded.roster_id,
                   is_tracked = excluded.is_tracked",
                params![
                    &roster_id,
                    character_id,
                    update.task_id,
                    (if update.tracked { 1 } else { 0 }),
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }
}
