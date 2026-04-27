use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::collections::HashMap;
use chrono::Datelike;
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
    /// - Daily tasks: reset if last_daily_reset is older than current daily reset time (11:00 UTC)
    /// - Weekly tasks: reset if last_weekly_reset is older than current weekly reset time (Wednesday 11:00 UTC)
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
            // Today's daily reset at 10:00 UTC
            let today_reset = now.date_naive().and_hms_opt(10, 0, 0).unwrap().and_utc();
            if now >= today_reset {
                today_reset.timestamp()
            } else {
                // If before 10:00 UTC today, use yesterday's reset time
                (today_reset - chrono::Duration::days(1)).timestamp()
            }
        };

        let weekly_reset_time = {
            // Find next Wednesday at 10:00 UTC
            let mut reset_date = now.date_naive();
            while reset_date.weekday().num_days_from_sunday() != 3 { // Wednesday (0=Sunday, 3=Wednesday)
                reset_date = reset_date + chrono::Duration::days(1);
            }
            // Set to 10:00 UTC
            let reset_time = reset_date.and_hms_opt(10, 0, 0).unwrap().and_utc();
            let final_reset_time = if now >= reset_time {
                // If we're past Wednesday's reset, go to next week
                reset_time + chrono::Duration::weeks(1)
            } else {
                reset_time
            };
            final_reset_time.timestamp()
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

    /// Update rested values for chaos/guardian based on completion status
    pub fn update_rested_values(&self, tx: &rusqlite::Transaction, task_id: &str) -> Result<()> {
        // Get all characters
        let mut stmt = tx.prepare("SELECT char_id FROM conf_character")?;
        let char_iter = stmt.query_map([], |row| {
            Ok(row.get::<_, i64>(0)?)
        })?;

        for char_result in char_iter {
            let char_id = char_result?;
            
            // Check if task was completed yesterday
            let now = chrono::Utc::now();
            let yesterday_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc() - chrono::Duration::days(1);
            let yesterday_end = now.date_naive().and_hms_opt(23, 59, 59).unwrap().and_utc() - chrono::Duration::days(1);
            
            let was_completed = tx.query_row(
                "SELECT is_completed FROM completion_status WHERE char_id = ?1 AND content_id = ?2 AND timestamp >= ?3 AND timestamp <= ?4",
                (char_id, task_id, 
                 yesterday_start.timestamp_millis(),
                 yesterday_end.timestamp_millis()),
                |row| Ok(row.get::<_, i64>(0)? == 1)
            ).unwrap_or(false);

            // Get roster_id for this character
            let roster_id: String = tx.query_row(
                "SELECT roster_id FROM conf_character WHERE char_id = ?1",
                [char_id],
                |row| row.get(0)
            ).unwrap_or_else(|_| "unknown".to_string());
            
            // Get current rested value and last updated timestamp
            let (current_rested, last_updated) = tx.query_row(
                "SELECT current_value, last_updated FROM rested_values WHERE char_id = ?1 AND content_id = ?2",
                (char_id, task_id),
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            ).unwrap_or((0, 0));

            // Calculate new rested value based on days since last update
            let new_rested = if was_completed {
                // Completed yesterday: no bonus (+0)
                current_rested
            } else {
                // Not completed yesterday: calculate days since last update
                let now = chrono::Utc::now();
                let days_since_update = if last_updated > 0 {
                    // Calculate full days since last update
                    let last_updated_date = chrono::DateTime::from_timestamp(last_updated, 0).unwrap_or(now);
                    let days = (now - last_updated_date).num_days();
                    // Only count full days that have passed (not including today)
                    if days > 0 { days as i32 } else { 0 }
                } else {
                    // No last_updated timestamp, assume maximum accumulation
                    10 // Default to 1 day if no history
                };
                
                // Add +10 for each day not completed (max 100)
                let bonus = (days_since_update * 10) as i64;
                std::cmp::min(current_rested + bonus, 100)
            };

            // Update rested value
            tx.execute(
                "INSERT OR REPLACE INTO rested_values (roster_id, char_id, content_id, current_value, last_updated) VALUES (?1, ?2, ?3, ?4, ?5)",
                (roster_id, char_id, task_id, new_rested, chrono::Utc::now().timestamp_millis()),
            )?;
        }

        Ok(())
    }
}
