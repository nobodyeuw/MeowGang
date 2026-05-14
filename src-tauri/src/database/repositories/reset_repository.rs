use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::collections::HashMap;
use chrono::{Datelike, TimeZone};
use crate::database::data_manager::GameTask;

#[derive(Debug, Clone)]
pub struct ResetRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl ResetRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        Self { pool }
    }

    /// Reset all tasks based on their reset_schedule
    /// - Daily tasks: reset if last_daily_reset is older than the most recent daily reset time (10:00 UTC)
    /// - Weekly tasks: reset if last_weekly_reset is older than the most recent weekly reset time (Wednesday 10:00 UTC)
    /// - Raids: reset with weekly schedule (no reset_schedule entry)
    pub fn reset_tasks_by_schedule(&self, tasks: &HashMap<String, GameTask>) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;

        let now = chrono::Utc::now();

        // Get current reset times from app_state
        let (last_daily, last_weekly): (i64, i64) = tx.query_row(
            "SELECT last_daily_reset, last_weekly_reset FROM app_state LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?))
        ).unwrap_or((0, 0));

        // Calculate current reset times - use UTC consistently
        // Tasks completed before this time should be reset
        let daily_reset_time = {
            // Most recent daily reset at 10:00 UTC
            let today_reset = now.date_naive().and_hms_opt(10, 0, 0).unwrap().and_utc();
            let reset_time = if now >= today_reset {
                today_reset
            } else {
                today_reset - chrono::Duration::days(1)
            };
            reset_time.timestamp_millis()
        };

        let weekly_reset_time = {
            // Most recent Wednesday reset at 10:00 UTC
            let mut reset_date = now.date_naive();
            while reset_date.weekday().num_days_from_monday() != 2 { // Wednesday (0=Monday, 2=Wednesday)
                reset_date = reset_date - chrono::Duration::days(1);
            }
            let reset_time = reset_date.and_hms_opt(10, 0, 0).unwrap().and_utc();
            let final_reset_time = if now < reset_time {
                reset_time - chrono::Duration::weeks(1)
            } else {
                reset_time
            };
            final_reset_time.timestamp_millis()
        };

        // Update rested values for chaos and guardian (runs daily regardless of reset condition)
        println!("Updating rested values - last: {}, current: {}", last_daily, daily_reset_time);
        self.update_rested_values(&tx, "chaos")?;
        self.update_rested_values(&tx, "guardian")?;
        
        // Check if daily reset is needed
        if last_daily < daily_reset_time {
            println!("Daily reset needed - last: {}, current: {}", last_daily, daily_reset_time);
            
            // Reset all daily tasks to false
            self.reset_daily_tasks(&tx, tasks, daily_reset_time)?;
            
            // Update app_state
            tx.execute(
                "UPDATE app_state SET last_daily_reset = ?1",
                params![daily_reset_time],
            )?;
        } else {
            println!("No daily reset needed - last: {}, current: {}", last_daily, daily_reset_time);
        }

        // Check if weekly reset is needed
        if last_weekly < weekly_reset_time { 
            println!("Performing weekly reset - last: {}, new: {}", last_weekly, weekly_reset_time);
            
            // Reset all weekly tasks to false
            self.reset_weekly_tasks(&tx, tasks)?;
            
            // Reset all raid gates to false
            self.reset_raid_gates(&tx)?;
            
            // Update app_state
            tx.execute(
                "UPDATE app_state SET last_weekly_reset = ?1",
                params![weekly_reset_time],
            )?;
            
            println!("Weekly reset completed successfully");
        } else {
            println!("No weekly reset needed - last: {}, threshold: {}", last_weekly, weekly_reset_time);
        }

        tx.commit()?;
        Ok(())
    }

    /// Reset all daily tasks (reset_schedule = 'daily') to false
    fn reset_daily_tasks(&self, tx: &rusqlite::Transaction, tasks: &HashMap<String, GameTask>, daily_reset_time: i64) -> Result<()> {
        let daily_tasks: Vec<String> = tasks
            .iter()
            .filter(|(_, task)| task.reset_schedule == "daily")
            .map(|(id, _): (&String, &GameTask)| id.clone())
            .collect();

        for task_id in daily_tasks {
            // Reset completion status to false only for tasks completed before daily reset time
            tx.execute(
                "UPDATE completion_status SET is_completed = 0 WHERE content_id = ?1 AND timestamp < ?2",
                params![task_id, daily_reset_time],
            )?;
        }

        Ok(())
    }

    /// Reset all weekly tasks (reset_schedule = 'weekly') to false
    fn reset_weekly_tasks(&self, tx: &rusqlite::Transaction, tasks: &HashMap<String, GameTask>) -> Result<()> {
        let weekly_tasks: Vec<String> = tasks
            .iter()
            .filter(|(_, task)| task.reset_schedule == "weekly")
            .map(|(id, _): (&String, &GameTask)| id.clone())
            .collect();

        for task_id in weekly_tasks {
            // Reset completion status to false
            tx.execute(
                "UPDATE completion_status SET is_completed = 0 WHERE content_id = ?1",
                params![task_id],
            )?;
        }

        Ok(())
    }

    /// Reset all raid gates to false (weekly reset)
    fn reset_raid_gates(&self, tx: &rusqlite::Transaction) -> Result<()> {
        // Reset all raid gate completions to false
        tx.execute(
            "UPDATE completion_status SET is_completed = 0 WHERE session_id IS NOT NULL",
            [],
        )?;
        
        Ok(())
    }

    /// Update rested values for chaos/guardian based on completion status.
    /// Idempotent within a single reset cycle: calling multiple times between
    /// two daily resets (10:00 UTC) produces the same result as calling once.
    pub fn update_rested_values(&self, tx: &rusqlite::Transaction, task_id: &str) -> Result<()> {
        let mut stmt = tx.prepare("SELECT char_id FROM conf_character")?;
        let char_iter = stmt.query_map([], |row| {
            Ok(row.get::<_, i64>(0)?)
        })?;

        let now = chrono::Utc::now();

        // Most recent daily reset boundary (10:00 UTC)
        let current_reset = {
            let today_reset = now.date_naive().and_hms_opt(10, 0, 0).unwrap().and_utc();
            if now >= today_reset {
                today_reset
            } else {
                today_reset - chrono::Duration::days(1)
            }
        };
        let current_reset_ms = current_reset.timestamp_millis();
        let previous_reset = current_reset - chrono::Duration::days(1);

        for char_result in char_iter {
            let char_id = char_result?;

            // Get current rested value and last updated timestamp
            let (current_rested, last_updated) = tx.query_row(
                "SELECT current_value, last_updated FROM rested_values WHERE char_id = ?1 AND content_id = ?2",
                (char_id, task_id),
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            ).unwrap_or((0, 0));

            // Already updated during this reset cycle — skip
            if last_updated >= current_reset_ms {
                continue;
            }

            // Check if task was completed during the previous reset cycle
            let was_completed = tx.query_row(
                "SELECT MAX(is_completed) FROM completion_status WHERE char_id = ?1 AND content_id = ?2 AND timestamp >= ?3 AND timestamp < ?4",
                (char_id, task_id,
                 previous_reset.timestamp_millis(),
                 current_reset_ms),
                |row| Ok(row.get::<_, Option<i64>>(0)?.unwrap_or(0) == 1)
            ).unwrap_or(false);

            let roster_id: String = tx.query_row(
                "SELECT roster_id FROM conf_character WHERE char_id = ?1",
                [char_id],
                |row| row.get(0)
            ).unwrap_or_else(|_| "unknown".to_string());

            let new_rested = if was_completed {
                current_rested
            } else {
                // Count how many reset cycles passed since last update
                let cycles_missed = if last_updated > 0 {
                    let last_updated_dt = chrono::Utc.timestamp_millis_opt(last_updated)
                        .single()
                        .unwrap_or(now);
                    let last_reset_at_update = {
                        let reset = last_updated_dt.date_naive().and_hms_opt(10, 0, 0).unwrap().and_utc();
                        if last_updated_dt >= reset { reset } else { reset - chrono::Duration::days(1) }
                    };
                    let days = (current_reset - last_reset_at_update).num_days();
                    std::cmp::max(days, 1) as i64
                } else {
                    1
                };

                let bonus = cycles_missed * 10;
                std::cmp::min(current_rested + bonus, 100)
            };

            // Store last_updated as the current reset boundary (not now()) so
            // subsequent calls within the same cycle are no-ops.
            tx.execute(
                "INSERT OR REPLACE INTO rested_values (roster_id, char_id, content_id, current_value, last_updated) VALUES (?1, ?2, ?3, ?4, ?5)",
                (roster_id, char_id, task_id, new_rested, current_reset_ms),
            )?;
        }

        Ok(())
    }
}
