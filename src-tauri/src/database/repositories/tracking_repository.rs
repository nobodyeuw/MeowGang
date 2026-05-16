use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use crate::models::*;
use crate::database::repositories::gold_repository::GoldRepository;

pub struct TrackingRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl TrackingRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        Self { pool }
    }

    pub fn get_tracking_config_matrix(&self, roster_id: &str) -> Result<crate::models::TodoConfigMatrix> {
        let conn = self.pool.get()?;
        
        // Get characters for this roster, ordered by display_order
        let mut char_stmt = conn.prepare(
            "SELECT char_id, char_name, class_id, item_level, combat_power, display_order
             FROM conf_character 
             WHERE roster_id = ?1
             ORDER BY CAST(display_order AS INTEGER)"
        )?;
        
        let character_iter = char_stmt.query_map([roster_id], |row| {
            Ok(crate::models::CharacterMatrixInfo {
                char_id: row.get::<_, i64>(0)?,
                char_name: row.get::<_, String>(1)?,
                item_level: row.get::<_, f64>(3)?,
                combat_power: row.get::<_, f64>(4)?,
                class_id: row.get::<_, String>(2)?,
                display_order: row.get::<_, String>(5)?.parse().unwrap_or(0),
            })
        })?;
        
        let characters: Vec<crate::models::CharacterMatrixInfo> = character_iter.filter_map(Result::ok).collect();
        
        // Get all conf_tracking entries for characters in this roster (both tasks and raids)
        let mut todo_stmt = conn.prepare(
            "SELECT char_id, content_id, is_tracked FROM conf_tracking WHERE roster_id = ?1"
        )?;
        
        let todo_iter = todo_stmt.query_map([roster_id], |row| {
            Ok((
                row.get::<_, i64>(0)?, // char_id
                row.get::<_, String>(1)?, // content_id
                row.get::<_, i64>(2)? == 1, // is_tracked
            ))
        })?;
        
        let todo_entries: Vec<(i64, String, bool)> = todo_iter.filter_map(Result::ok).collect();
        
        // Get rested values for characters in this roster
        let mut rested_stmt = conn.prepare(
            "SELECT char_id, content_id, current_value 
             FROM rested_values 
             WHERE char_id IN (
                 SELECT char_id FROM conf_character WHERE roster_id = ?1
             )"
        )?;
        
        let rested_iter = rested_stmt.query_map([roster_id], |row| {
            Ok((
                row.get::<_, i64>(0)?, // char_id
                row.get::<_, String>(1)?, // content_id
                row.get::<_, i64>(2)?, // current_value
            ))
        })?;
        
        let rested_entries: Vec<(i64, String, i64)> = rested_iter.filter_map(Result::ok).collect();
        
        // Create character states for each content_id
        let mut character_states = std::collections::HashMap::new();
        for char in &characters {
            for (char_id, content_id, is_tracked) in &todo_entries {
                // Only create state if this entry belongs to the current character
                if *char_id == char.char_id {
                    // Get rested value if exists, default to 0
                    let current_value = rested_entries
                        .iter()
                        .find(|(rested_char_id, rested_content_id, _)| 
                            rested_char_id == char_id && rested_content_id == content_id)
                        .map(|(_, _, value)| *value)
                        .unwrap_or(0);
                    
                    character_states.insert((char.char_id, content_id.clone()), crate::models::CharacterRaidState {
                        char_id: char.char_id,
                        content_id: content_id.clone(),
                        tracked: *is_tracked,
                        current_value: Some(current_value),
                    });
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
            todo_entries: Some(todo_entries),
            rested_entries: Some(rested_entries),
            character_states: Some(character_states.into_iter().map(|((char_id, content_id), state)| state).collect()),
        };
        
        Ok(matrix)
    }

    pub fn update_tracking_config(&self, character_id: i64, content_id: &str, is_tracked: bool) -> Result<()> {
        let conn = self.pool.get()?;
        
        // First get roster_id for this character
        let roster_id: String = conn.query_row(
            "SELECT roster_id FROM conf_character WHERE char_id = ?1",
            params![character_id],
            |row| row.get(0)
        )?;
        
        let tracked_value = if is_tracked { 1 } else { 0 };
        
        conn.execute(
            "INSERT OR REPLACE INTO conf_tracking (roster_id, char_id, content_id, is_tracked) VALUES (?1, ?2, ?3, ?4)",
            params![roster_id, character_id, content_id, tracked_value],
        )?;
        
        Ok(())
    }

    pub fn save_rested_value(&self, character_id: i64, content_id: &str, rested_value: i64) -> Result<()> {
        let conn = self.pool.get()?;
        
        // First get roster_id for this character
        let roster_id: String = conn.query_row(
            "SELECT roster_id FROM conf_character WHERE char_id = ?1",
            params![character_id],
            |row| row.get(0)
        )?;
        
        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?
            .as_millis() as i64;
        
        conn.execute(
            "INSERT OR REPLACE INTO rested_values (char_id, content_id, current_value, roster_id, last_updated) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![character_id, content_id, rested_value, roster_id, timestamp],
        )?;
        Ok(())
    }

    pub fn set_todo_tracked(&self, character_id: i64, content_id: &str, tracked: bool) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE conf_tracking SET is_tracked = ?1 WHERE char_id = ?2 AND content_id = ?3",
            params![if tracked { 1 } else { 0 }, character_id, content_id],
        )?;
        Ok(())
    }

    pub fn batch_update_task_status(&self, character_id: i64, task_updates: Vec<crate::models::TaskStatusStruct>) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        
        for update in task_updates {
            tx.execute(
                "INSERT OR REPLACE INTO conf_tracking 
                 (char_id, content_id, is_tracked) 
                 VALUES (?1, ?2, ?3)",
                params![
                    character_id,
                    update.task_id,
                    (if update.tracked { 1 } else { 0 }),
                ],
            )?;
        }
        
        tx.commit()?;
        Ok(())
    }

    // Methods needed for todo handlers
    pub fn get_tracked_tasks_for_roster(&self, roster_id: &str) -> Result<Vec<crate::models::TaskConfig>> {
        let conn = self.pool.get()?;
        
        let mut stmt = conn.prepare(
            "SELECT DISTINCT ct.content_id, gt.content_name, gt.category, gt.reset_schedule, 
                    gt.logic_type, gt.max_rest_value, gt.min_ilvl
             FROM conf_tracking ct
             JOIN game_tasks gt ON ct.content_id = gt.id
             WHERE ct.roster_id = ?1 AND ct.is_tracked = 1
             ORDER BY gt.category, gt.reset_schedule, gt.content_name"
        )?;
        
        let task_iter = stmt.query_map([roster_id], |row| {
            Ok(crate::models::TaskConfig {
                content_id: row.get(0)?,
                content_name: row.get(1)?,
                category: row.get(2)?,
                reset_schedule: row.get(3)?,
                logic_type: row.get(4)?,
                max_rest_value: row.get(5)?,
                min_ilvl: row.get(6)?,
            })
        })?;
        
        let mut tasks = Vec::new();
        for task in task_iter {
            tasks.push(task?);
        }
        
        Ok(tasks)
    }

    pub fn is_task_tracked(&self, char_id: i64, task_id: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        
        let mut stmt = conn.prepare(
            "SELECT is_tracked FROM conf_tracking WHERE char_id = ?1 AND content_id = ?2"
        )?;
        
        let result: Result<i64, rusqlite::Error> = stmt.query_row(params![char_id, task_id], |row| row.get(0));
        
        match result {
            Ok(tracked) => Ok(tracked == 1),
            Err(_) => Ok(false),
        }
    }

    pub fn get_rested_value(&self, char_id: i64, task_id: &str) -> Result<i64> {
        let conn = self.pool.get()?;
        
        let mut stmt = conn.prepare(
            "SELECT current_value FROM rested_values WHERE char_id = ?1 AND content_id = ?2"
        )?;
        
        let result: i64 = stmt.query_row(params![char_id, task_id], |row| row.get(0))?;
        Ok(result)
    }

    pub fn is_raid_tracked(&self, char_id: i64, raid_id: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        
        let mut stmt = conn.prepare(
            "SELECT COUNT(*) FROM conf_raid WHERE char_id = ?1 AND content_id = ?2"
        )?;
        
        let count: i64 = stmt.query_row(params![char_id, raid_id], |row| row.get(0))?;
        
        Ok(count > 0)
    }
}
