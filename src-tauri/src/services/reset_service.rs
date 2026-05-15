use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::time::{sleep, Duration};
use chrono::{Utc, Datelike, Weekday, Timelike};
use std::collections::HashMap;

pub struct ResetService {
    pool: Pool<SqliteConnectionManager>,
}

impl ResetService {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        Self { pool }
    }

    /// Start the scheduled daily reset service
    pub fn start_scheduled_reset(pool: Pool<SqliteConnectionManager>) {
        tauri::async_runtime::spawn(async move {
            loop {
                // Calculate time until next daily reset (10:00 UTC = 12:00 German time)
                let now = Utc::now();
                let mut next_reset = now.date_naive().and_hms_opt(10, 0, 0).unwrap().and_utc();
                
                // If today's reset time has passed, schedule for tomorrow
                if now >= next_reset {
                    next_reset = next_reset + chrono::Duration::days(1);
                }
                
                let sleep_duration = (next_reset - now).to_std().unwrap_or(Duration::from_secs(86400));
                
                crate::log_info!("Next daily reset scheduled for: {}", next_reset.format("%Y-%m-%d %H:%M:%S UTC"));
                
                // Perform a startup / recovery reset check immediately.
                let service = ResetService::new(pool.clone());
                match service.perform_reset().await {
                    Ok(message) => {
                        crate::log_info!("Startup reset check completed: {}", message);
                    },
                    Err(e) => {
                        crate::log_error!("Startup reset check failed: {} ({:?})", e, e);
                    }
                }

                // Sleep until next reset time
                sleep(sleep_duration).await;
                
                // Perform the reset with enhanced error handling
                let service = ResetService::new(pool.clone());
                match service.perform_reset().await {
                    Ok(message) => {
                        crate::log_info!("Scheduled daily reset completed: {}", message);
                    },
                    Err(e) => {
                        crate::log_error!("Scheduled daily reset failed: {} ({:?})", e, e);
                        // Continue the loop even if reset fails
                    }
                }
            }
        });
    }

    /// Perform the reset using the repository pattern
    pub async fn perform_reset(&self) -> Result<String> {
        let reset_repo = crate::database::repositories::ResetRepository::new(self.pool.clone());
        let gold_repo = crate::database::repositories::GoldRepository::new(self.pool.clone());
        
        // Get game tasks from data manager
        let tasks = crate::database::data_manager::DataManager::get_game_tasks()?;
        
        // Check if weekly reset is happening and clean old gold logs
        let conn = self.pool.get()?;
        let (last_daily, last_weekly): (i64, i64) = conn.query_row(
            "SELECT last_daily_reset, last_weekly_reset FROM app_state LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?))
        ).unwrap_or((0, 0));
        
        let now = chrono::Utc::now();
        
        // Calculate weekly reset time (Wednesday 10:00 UTC)
        let weekly_reset_time = {
            let mut reset_date = now.date_naive();
            while reset_date.weekday().num_days_from_sunday() != 3 { // Wednesday
                reset_date = reset_date - chrono::Duration::days(1);
            }
            let mut reset_time = reset_date.and_hms_opt(10, 0, 0).unwrap().and_utc();
            if now < reset_time {
                reset_time = reset_time - chrono::Duration::weeks(1);
            }
            reset_time.timestamp_millis()
        };
        
        // If weekly reset is happening, delete old gold logs
        if last_weekly < weekly_reset_time {
            crate::log_info!("Weekly reset detected, cleaning old gold logs");
            match gold_repo.delete_old_gold_logs(weekly_reset_time) {
                Ok(count) => crate::log_info!("Deleted {} old gold log entries", count),
                Err(e) => crate::log_error!("Failed to delete old gold logs: {}", e),
            }
        }
        
        // Perform reset using repository
        reset_repo.reset_tasks_by_schedule(&tasks)?;
        
        Ok("Reset completed successfully".to_string())
    }

    /// Force an immediate reset (for testing/debugging)
    pub async fn force_reset(&self) -> Result<String> {
        crate::log_info!("Force reset triggered - bypassing schedule checks");
        
        let reset_repo = crate::database::repositories::ResetRepository::new(self.pool.clone());
        let tasks = crate::database::data_manager::DataManager::get_game_tasks()?;
        
        // Force reset by updating timestamps first
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        
        let now = chrono::Utc::now();
        let yesterday = now - chrono::Duration::days(1);
        
        // Set last reset times to yesterday to force reset
        tx.execute(
            "UPDATE app_state SET last_daily_reset = ?1, last_weekly_reset = ?2",
            [yesterday.timestamp_millis(), yesterday.timestamp_millis()],
        )?;
        
        tx.commit()?;
        
        // Now perform the reset
        reset_repo.reset_tasks_by_schedule(&tasks)?;
        
        Ok("Force reset completed successfully".to_string())
    }

    /// Update rested values only (without performing full reset)
    pub async fn update_rested_values_only(&self) -> Result<String> {
        crate::log_info!("Updating rested values immediately");
        
        let reset_repo = crate::database::repositories::ResetRepository::new(self.pool.clone());
        
        // Update rested values for chaos and guardian
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        
        reset_repo.update_rested_values(&tx, "chaos")?;
        reset_repo.update_rested_values(&tx, "guardian")?;
        
        tx.commit()?;
        
        Ok("Rested values updated successfully".to_string())
    }

    /// Check if a calendar-based task is currently available
    pub fn is_calendar_task_available(&self, task_id: &str) -> Result<bool> {
        let now = Utc::now();
        
        match task_id {
            "gate" | "boss" => {
                // Calendar tasks: Monday 10:00 to Tuesday 05:00, Tuesday 10:00 to Wednesday 05:00, etc.
                let weekday = now.weekday();
                let hour = now.hour();
                
                match weekday {
                    Weekday::Mon => Ok(hour >= 10), // Monday 10:00 onwards
                    Weekday::Tue => Ok(hour < 5 || hour >= 10), // Tuesday before 05:00 OR after 10:00
                    Weekday::Wed => Ok(hour < 5 || hour >= 10), // Wednesday before 05:00 OR after 10:00
                    Weekday::Thu => Ok(hour < 5 || hour >= 10), // Thursday before 05:00 OR after 10:00
                    Weekday::Fri => Ok(hour < 5 || hour >= 10), // Friday before 05:00 OR after 10:00
                    Weekday::Sat => Ok(hour < 5 || hour >= 10), // Saturday before 05:00 OR after 10:00
                    Weekday::Sun => Ok(hour < 5), // Sunday before 05:00 only
                }
            },
            _ => Ok(true), // Other tasks are always available
        }
    }

    /// Get next reset time for a task
    pub fn get_next_reset_time(&self, task_id: &str) -> Result<chrono::DateTime<Utc>> {
        let now = Utc::now();
        let german_time = now + chrono::Duration::hours(2); // CEST (UTC+2)
        
        match task_id {
            "gate" | "boss" => {
                // Find next 12:00 German time (10:00 UTC)
                let mut next_reset = german_time.date_naive().and_hms_opt(12, 0, 0).unwrap();
                
                // If current time is after 12:00, next reset is tomorrow at 12:00
                if german_time.hour() >= 12 {
                    next_reset = next_reset + chrono::Duration::days(1);
                }
                
                // Convert back to UTC
                let next_reset_utc = next_reset.and_utc() - chrono::Duration::hours(2);
                Ok(next_reset_utc)
            },
            "chaos" | "guardian" => {
                // Daily reset at 10:00 UTC
                let mut next_reset = now.date_naive().and_hms_opt(10, 0, 0).unwrap().and_utc();
                if now >= next_reset {
                    next_reset = next_reset + chrono::Duration::days(1);
                }
                Ok(next_reset)
            },
            "cube" | "paradise" | "shop" | "guild" => {
                // Weekly reset on Wednesday at 10:00 UTC
                let mut reset_time = now.date_naive();
                while reset_time.weekday().num_days_from_sunday() != 3 { // Wednesday
                    reset_time = reset_time + chrono::Duration::days(1);
                }
                let mut next_reset = reset_time.and_hms_opt(10, 0, 0).unwrap().and_utc();
                if now >= next_reset {
                    next_reset = next_reset + chrono::Duration::weeks(1);
                }
                Ok(next_reset)
            },
            task_id if task_id.starts_with("raid_") => {
                // Raids also have weekly reset on Wednesday at 10:00 UTC
                let mut reset_time = now.date_naive();
                while reset_time.weekday().num_days_from_sunday() != 3 { // Wednesday
                    reset_time = reset_time + chrono::Duration::days(1);
                }
                let mut next_reset = reset_time.and_hms_opt(10, 0, 0).unwrap().and_utc();
                if now >= next_reset {
                    next_reset = next_reset + chrono::Duration::weeks(1);
                }
                Ok(next_reset)
            },
            _ => Ok(now + chrono::Duration::days(1)), // Default: tomorrow
        }
    }
}
