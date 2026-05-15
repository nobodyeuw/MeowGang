use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use std::sync::Arc;
use crate::database::data_manager::{Raid, RaidGate};

/// Service responsible for tracking gold income and expenditure from raid completions.
///
/// Gold logs are generated when encounters are synced from the external encounters.db
/// file. The service deduplicates entries within a weekly reset window and supports
/// both gold earnings and box purchase tracking.
pub struct GoldLoggingService {
    pool: Arc<Pool<SqliteConnectionManager>>,
}

impl GoldLoggingService {
    /// Creates a new `GoldLoggingService` backed by the given connection pool.
    pub fn new(pool: Arc<Pool<SqliteConnectionManager>>) -> Self {
        Self { pool }
    }

    /// Process pending gold logs for raid completions with optimized transaction management
    pub fn process_pending_gold_logs(&self, raid_state: &crate::state::RaidDataState) -> Result<usize> {
        // Use a single connection for the read operation, then batch process writes
        let conn = self.pool.get()?;
        
        // Find raid completions without corresponding gold logs
        let mut stmt = conn.prepare(
            "SELECT cs.char_id, cs.content_id, cs.details, cs.timestamp, cs.session_id
             FROM completion_status cs
             LEFT JOIN gold_logs gl ON cs.char_id = gl.char_id AND cs.timestamp = gl.timestamp
             WHERE cs.is_completed = 1 
             AND gl.rowid IS NULL
             AND cs.details IS NOT NULL"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,  // char_id
                row.get::<_, String>(1)?, // content_id
                row.get::<_, String>(2)?, // details (difficulty)
                row.get::<_, i64>(3)?,   // timestamp
                row.get::<_, String>(4)?, // session_id
            ))
        })?;

        // Materialize pending row tuples before doing any additional DB work
        let mut pending_rows = Vec::new();
        for row_result in rows {
            pending_rows.push(row_result?);
        }

        drop(stmt);
        drop(conn);

        // Evaluate pending entries on a fresh connection to avoid nested active statements
        let conn = self.pool.get()?;
        let mut pending_entries = Vec::new();
        for (char_id, content_id, difficulty, timestamp, session_id) in pending_rows {
            let gate = if let Some(gate_part) = session_id.split('_').last() {
                gate_part.to_string()
            } else {
                crate::log_warn!("Invalid session_id format: {}", session_id);
                continue;
            };

            // Check if character earns gold
            let earns_gold: bool = conn.query_row(
                "SELECT earns_gold FROM conf_character WHERE char_id = ?1",
                [char_id],
                |row| row.get(0)
            ).optional()?.unwrap_or(false);

            if !earns_gold {
                continue; // Skip characters that don't earn gold
            }

            // Get raid configuration
            let raid_config = conn.query_row(
                "SELECT take_gold, buy_box FROM conf_raid WHERE char_id = ?1 AND content_id = ?2 AND gate = ?3",
                params![char_id, content_id, gate],
                |row| Ok((
                    row.get::<_, i64>(0)? == 1,
                    row.get::<_, i64>(1)? == 1
                ))
            ).optional()?;

            let (take_gold, buy_box) = match raid_config {
                Some(config) => config,
                None => continue, // No raid configuration found
            };

            if !take_gold && !buy_box {
                continue; // Character doesn't take gold or buy boxes
            }

            // Get raid data from RaidDataState (frontend-driven, NO backend hardcoding)
            // Normalize difficulty to lowercase to handle case-sensitivity (e.g., "Hard" vs "hard")
            let normalized_difficulty = difficulty.to_lowercase();
            let raid_data = match self.get_raid_data_from_state(&content_id, &normalized_difficulty, &session_id, raid_state) {
                Ok(data) => data,
                Err(e) => {
                    crate::log_warn!("Failed to get raid data for {} {} from state: {}", content_id, difficulty, e);
                    continue;
                }
            };

            pending_entries.push((char_id, content_id, difficulty, gate, timestamp, raid_data, take_gold, buy_box));
        }

        drop(conn);

        // Process all entries in a single transaction for better performance
        let mut processed_count = 0;
        if !pending_entries.is_empty() {
            crate::log_debug!("Found {} pending gold entries to process", pending_entries.len());
            let mut conn = self.pool.get()?;
            let tx = conn.transaction()?;
            
            for (char_id, content_id, difficulty, gate, timestamp, raid_data, take_gold, buy_box) in pending_entries {
                crate::log_debug!("Processing gold for char {} raid {} gate {} take_gold {} buy_box {}", char_id, content_id, gate, take_gold, buy_box);
                if let Err(e) = self.log_gold_for_raid_completion_in_transaction(
                    &tx,
                    char_id,
                    &content_id,
                    &difficulty,
                    &gate,
                    timestamp,
                    &raid_data,
                    take_gold,
                    buy_box,
                ) {
                    crate::log_error!("Failed to log gold for character {} raid {}: {}", char_id, content_id, e);
                } else {
                    processed_count += 1;
                    crate::log_debug!("Successfully logged gold for {} {}", char_id, content_id);
                }
            }
            
            tx.commit()?;
        }

        Ok(processed_count)
    }

    /// Log gold earnings for a raid completion within a transaction.
    /// Deduplicates by (char_id, notes) within the current weekly reset window
    /// so that timestamp changes in completion_status don't cause double entries.
    fn log_gold_for_raid_completion_in_transaction(
        &self,
        tx: &rusqlite::Transaction,
        char_id: i64,
        raid_id: &str,
        difficulty: &str,
        gate: &str,
        timestamp: i64,
        raid_data: &Raid,
        take_gold: bool,
        buy_box: bool,
    ) -> Result<()> {
        // Find gate data
        let gate_data = raid_data.gates.iter()
            .find(|g| g.gate == gate)
            .ok_or_else(|| anyhow::anyhow!("Gate {} not found for raid {}", gate, raid_id))?;

        let weekly_reset: i64 = tx.query_row(
            "SELECT last_weekly_reset FROM app_state LIMIT 1",
            [],
            |row| row.get(0)
        ).unwrap_or(0);

        // Calculate gold values based on character settings
        let (gold_bound, gold_tradable) = if take_gold {
            (gate_data.bound_gold, gate_data.tradable_gold)
        } else {
            (0, 0)
        };

        // Log gold earnings
        if gold_bound > 0 || gold_tradable > 0 {
            let raid_name = &raid_data.name;
            let notes_value = format!("{} {} {}", raid_name, difficulty, gate);

            let already_logged: i64 = tx.query_row(
                "SELECT COUNT(*) FROM gold_logs WHERE char_id = ?1 AND notes = ?2 AND timestamp >= ?3",
                params![char_id, notes_value, weekly_reset],
                |row| row.get(0)
            ).unwrap_or(0);

            if already_logged == 0 {
                let gold_value_total = gold_bound + gold_tradable;
                tx.execute(
                    "INSERT INTO gold_logs (timestamp, char_id, source, gold_value_total, gold_bound, gold_tradable, notes)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        timestamp,
                        char_id,
                        "raid",
                        gold_value_total,
                        gold_bound,
                        gold_tradable,
                        notes_value
                    ]
                )?;
            }
        }

        // Handle box purchases if enabled
        if buy_box {
            let raid_name = &raid_data.name;
            let box_notes = format!("Box Purchase {} {} {}", raid_name, difficulty, gate);

            let already_logged: i64 = tx.query_row(
                "SELECT COUNT(*) FROM gold_logs WHERE char_id = ?1 AND notes = ?2 AND timestamp >= ?3",
                params![char_id, box_notes, weekly_reset],
                |row| row.get(0)
            ).unwrap_or(0);

            if already_logged == 0 {
                tx.execute(
                    "INSERT INTO gold_logs (timestamp, char_id, source, gold_value_total, gold_bound, gold_tradable, notes)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        timestamp,
                        char_id,
                        "box_purchase",
                        -gate_data.box_price,
                        0,
                        -gate_data.box_price,
                        box_notes
                    ]
                )?;
            }
        }

        Ok(())
    }

    
    /// Check and process gold logs for all recent completions
    pub fn check_and_process_recent_completions(&self, raid_state: &crate::state::RaidDataState) -> Result<usize> {
        self.process_pending_gold_logs(raid_state)
    }

    /// Get raid data from RaidDataState (frontend-driven, NO backend hardcoding)
    fn get_raid_data_from_state(&self, content_id: &str, details: &str, session_id: &str, raid_state: &crate::state::RaidDataState) -> Result<Raid> {
        // Use content_id directly as raid_id: "act_4_armoche"
        let raid_id = content_id;
        
        // Use details as difficulty
        let difficulty = details;

        // Find matching raid in RaidDataState
        let raid = raid_state.find_raid(raid_id, difficulty)
            .ok_or_else(|| anyhow::anyhow!("Raid {} with difficulty {} not found in RaidDataState", raid_id, difficulty))?;

        // Extract gate from session_id: "act_4_armoche_Gate 2" -> "Gate 2"
        let gate = if let Some(gate_part) = session_id.split('_').last() {
            gate_part
        } else {
            return Err(anyhow::anyhow!("Invalid session_id format: {}", session_id));
        };

        // Find matching gate
        let gate_data = raid.gates.iter()
            .find(|g| g.gate == gate)
            .ok_or_else(|| anyhow::anyhow!("Gate {} not found for raid {} with difficulty {}", gate, raid_id, difficulty))?;

        Ok(Raid {
            id: raid.id.clone(),
            name: raid.name.clone(),
            difficulty: raid.difficulty.clone(),
            gates: vec![
                RaidGate {
                    gate: gate_data.gate.clone(),
                    min_ilvl: gate_data.min_ilvl,
                    tradable_gold: gate_data.tradable_gold,
                    bound_gold: gate_data.bound_gold,
                    box_price: gate_data.box_price,
                }
            ],
        })
    }

    /// Removes gold log entries for a specific raid completion.
    ///
    /// Called when a user un-marks a raid gate as completed so that the
    /// corresponding gold income / box purchase entries are reversed.
    pub fn delete_gold_logs_for_raid_completion(
        &self,
        char_id: i64,
        content_id: &str,
        difficulty: &str,
        _session_id: &str,
    ) -> Result<usize> {
        crate::log_info!("Deleting gold logs for char {} raid {} difficulty {}", char_id, content_id, difficulty);
        
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        
        // Delete gold logs associated with this specific raid completion
        // Extract raid name from content_id and use more flexible pattern matching
        let raid_name = if content_id.contains('_') {
            content_id.split('_').last().unwrap_or(content_id)
        } else {
            content_id
        };
        
        let result = tx.execute(
            "DELETE FROM gold_logs 
             WHERE char_id = ?1 
             AND source IN ('raid', 'box_purchase')
             AND (notes LIKE ?2 OR notes LIKE ?3 OR notes LIKE ?4 OR notes LIKE ?5)",
            params![
                char_id, 
                format!("%{}%", raid_name),                    // Match raid name anywhere
                format!("%{}%", content_id),                   // Match full content_id
                format!("%{} {}%", raid_name, difficulty),     // Raid name with difficulty
                format!("%{} {}%", content_id, difficulty)     // Full content_id with difficulty
            ]
        )?;
        
        tx.commit()?;
        
        let total_deleted = result;
        crate::log_info!("Deleted {} gold log entries", result);
        
        Ok(total_deleted)
    }

    /// Clean up duplicate gold log entries for a character
    /// Keeps the earliest entry and removes duplicates with same char_id, source, and notes
    pub fn clean_duplicate_gold_logs(&self, char_id: i64) -> Result<usize> {
        crate::log_info!("Cleaning up duplicate gold logs for character {}", char_id);
        
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        
        // Find and remove duplicates, keeping only the earliest entry for each unique combination
        let result = tx.execute(
            "DELETE FROM gold_logs 
             WHERE rowid NOT IN (
                 SELECT MIN(rowid) 
                 FROM gold_logs 
                 WHERE char_id = ?1 
                 GROUP BY char_id, source, notes
             ) AND char_id = ?1",
            [char_id]
        )?;
        
        tx.commit()?;
        
        crate::log_info!("Cleaned up {} duplicate gold log entries for character {}", result, char_id);
        
        Ok(result)
    }

    /// Clean up all duplicate gold log entries in the database
    pub fn clean_all_duplicate_gold_logs(&self) -> Result<usize> {
        crate::log_info!("Cleaning up all duplicate gold log entries");
        
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        
        // Find and remove duplicates, keeping only the earliest entry for each unique combination
        let result = tx.execute(
            "DELETE FROM gold_logs 
             WHERE rowid NOT IN (
                 SELECT MIN(rowid) 
                 FROM gold_logs 
                 GROUP BY char_id, source, notes
             )",
            []
        )?;
        
        tx.commit()?;
        
        crate::log_info!("Cleaned up {} duplicate gold log entries total", result);
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pool() -> Arc<Pool<SqliteConnectionManager>> {
        let manager = SqliteConnectionManager::memory();
        let pool = Pool::builder().max_size(1).build(manager).unwrap();
        let conn = pool.get().unwrap();

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS gold_logs (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                char_id INTEGER NOT NULL,
                source TEXT NOT NULL,
                gold_value_total INTEGER NOT NULL,
                gold_bound INTEGER NOT NULL DEFAULT 0,
                gold_tradable INTEGER NOT NULL DEFAULT 0,
                notes TEXT
            );
            CREATE TABLE IF NOT EXISTS app_state (
                last_daily_reset INTEGER DEFAULT 0,
                last_weekly_reset INTEGER DEFAULT 0
            );
            INSERT INTO app_state (last_daily_reset, last_weekly_reset) VALUES (0, 0);
            CREATE TABLE IF NOT EXISTS conf_character (
                char_id INTEGER PRIMARY KEY,
                char_name TEXT,
                roster_id TEXT,
                earns_gold INTEGER DEFAULT 1
            );
            CREATE TABLE IF NOT EXISTS conf_raid (
                char_id INTEGER,
                content_id TEXT,
                gate TEXT,
                take_gold INTEGER DEFAULT 1,
                buy_box INTEGER DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS completion_status (
                roster_id TEXT,
                char_id INTEGER,
                content_id TEXT,
                is_completed INTEGER,
                timestamp INTEGER,
                session_id TEXT,
                completion_source TEXT,
                details TEXT
            );"
        ).unwrap();

        Arc::new(pool)
    }

    #[test]
    fn test_clean_all_duplicate_gold_logs_no_duplicates() {
        let pool = create_test_pool();
        let service = GoldLoggingService::new(pool.clone());

        let conn = pool.get().unwrap();
        conn.execute(
            "INSERT INTO gold_logs (timestamp, char_id, source, gold_value_total, gold_bound, gold_tradable, notes)
             VALUES (1000, 1, 'raid', 5000, 2000, 3000, 'Mordum Hard Gate 1')",
            [],
        ).unwrap();

        let cleaned = service.clean_all_duplicate_gold_logs().unwrap();
        assert_eq!(cleaned, 0);
    }

    #[test]
    fn test_clean_all_duplicate_gold_logs_removes_duplicates() {
        let pool = create_test_pool();
        let service = GoldLoggingService::new(pool.clone());

        let conn = pool.get().unwrap();
        conn.execute(
            "INSERT INTO gold_logs (timestamp, char_id, source, gold_value_total, gold_bound, gold_tradable, notes)
             VALUES (1000, 1, 'raid', 5000, 2000, 3000, 'Mordum Hard Gate 1')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO gold_logs (timestamp, char_id, source, gold_value_total, gold_bound, gold_tradable, notes)
             VALUES (2000, 1, 'raid', 5000, 2000, 3000, 'Mordum Hard Gate 1')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO gold_logs (timestamp, char_id, source, gold_value_total, gold_bound, gold_tradable, notes)
             VALUES (3000, 1, 'raid', 5000, 2000, 3000, 'Mordum Hard Gate 1')",
            [],
        ).unwrap();

        let cleaned = service.clean_all_duplicate_gold_logs().unwrap();
        assert_eq!(cleaned, 2);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM gold_logs", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_clean_duplicate_gold_logs_per_character() {
        let pool = create_test_pool();
        let service = GoldLoggingService::new(pool.clone());

        let conn = pool.get().unwrap();
        // Insert duplicates for char 1
        conn.execute(
            "INSERT INTO gold_logs (timestamp, char_id, source, gold_value_total, notes) VALUES (100, 1, 'raid', 500, 'A')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO gold_logs (timestamp, char_id, source, gold_value_total, notes) VALUES (200, 1, 'raid', 500, 'A')",
            [],
        ).unwrap();
        // Insert unique entry for char 2
        conn.execute(
            "INSERT INTO gold_logs (timestamp, char_id, source, gold_value_total, notes) VALUES (100, 2, 'raid', 300, 'B')",
            [],
        ).unwrap();

        let cleaned = service.clean_duplicate_gold_logs(1).unwrap();
        assert_eq!(cleaned, 1);

        // Char 2's entry should be untouched
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM gold_logs WHERE char_id = 2", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_delete_gold_logs_for_raid_completion() {
        let pool = create_test_pool();
        let service = GoldLoggingService::new(pool.clone());

        let conn = pool.get().unwrap();
        conn.execute(
            "INSERT INTO gold_logs (timestamp, char_id, source, gold_value_total, notes) VALUES (100, 1, 'raid', 5000, 'Mordum Hard Gate 1')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO gold_logs (timestamp, char_id, source, gold_value_total, notes) VALUES (200, 1, 'box_purchase', -500, 'Box Purchase Mordum Hard Gate 1')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO gold_logs (timestamp, char_id, source, gold_value_total, notes) VALUES (300, 1, 'raid', 3000, 'Armoche Normal Gate 1')",
            [],
        ).unwrap();

        let deleted = service.delete_gold_logs_for_raid_completion(
            1, "act_3_mordum", "Hard", "act_3_mordum_Gate 1"
        ).unwrap();

        // Should delete the Mordum entries but not Armoche
        assert!(deleted >= 2);

        let remaining: i64 = conn.query_row(
            "SELECT COUNT(*) FROM gold_logs WHERE notes LIKE '%Armoche%'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(remaining, 1);
    }

    #[test]
    fn test_process_pending_gold_logs_empty() {
        let pool = create_test_pool();
        let service = GoldLoggingService::new(pool);
        let raid_state = crate::state::RaidDataState::new();

        let processed = service.process_pending_gold_logs(&raid_state).unwrap();
        assert_eq!(processed, 0);
    }
}
