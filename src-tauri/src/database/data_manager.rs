use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::Datelike;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameTask {
    pub id: String,
    pub name: String,
    pub category: String, // "roster" | "character"
    pub reset_schedule: String, // "daily" | "weekly"
    pub logic_type: String, // "normal" | "calendar" | "rested"
    pub max_rest_value: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidGate {
    pub gate: String,
    #[serde(rename = "minIlvl")]
    pub min_ilvl: i32,
    #[serde(rename = "tradableGold")]
    pub tradable_gold: i32,
    #[serde(rename = "boundGold")]
    pub bound_gold: i32,
    #[serde(rename = "boxPrice")]
    pub box_price: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Raid {
    pub id: String,
    pub name: String,
    pub difficulty: String, // "Solo" | "Normal" | "Hard" | "Nightmare"
    pub gates: Vec<RaidGate>,
}

// Convert DataManager Raid to Repository Raid
impl From<Raid> for crate::models::Raid {
    fn from(dm_raid: Raid) -> Self {
        crate::models::Raid {
            id: dm_raid.id,
            name: dm_raid.name,
            difficulty: dm_raid.difficulty,
            min_ilvl: dm_raid.gates.first().map(|g| g.min_ilvl as i64).unwrap_or(0),
            max_players: 4, // Default to 4 players
            gates: dm_raid.gates.into_iter().map(|g| crate::models::RaidGate {
                gate: g.gate.clone(),
                name: g.gate,
                min_ilvl: g.min_ilvl as i64,
                tradable_gold: Some(g.tradable_gold),
                bound_gold: Some(g.bound_gold),
                box_price: Some(g.box_price),
            }).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameClass {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "iconId")]
    pub icon_id: String,
}

pub struct DataManager;

impl DataManager {
    /// Get current schema version from app_metadata
    pub fn get_schema_version(pool: &Pool<SqliteConnectionManager>) -> Result<i32> {
        let conn = pool.get()?;
        match conn.query_row(
            "SELECT value FROM app_metadata WHERE key = 'schema_version'",
            [],
            |row| {
                let value_str: String = row.get(0)?;
                value_str.parse::<i32>().map_err(|_| rusqlite::Error::InvalidColumnType(0, "value".to_string(), rusqlite::types::Type::Text))
            }) {
            Ok(version) => Ok(version),
            Err(_) => Ok(1), // Default version 1
        }
    }

    /// Set schema version in app_metadata
    pub fn set_schema_version(pool: &Pool<SqliteConnectionManager>, version: i32) -> Result<()> {
        let conn = pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO app_metadata (key, value, timestamp, app_version) VALUES ('schema_version', ?1, ?2, '1.0.0')",
            params![version.to_string(), chrono::Utc::now().timestamp_millis()],
        )?;
        Ok(())
    }

    /// Check whether a column exists on a table (uses the same connection/transaction).
    fn column_exists(conn: &rusqlite::Connection, table: &str, column: &str) -> bool {
        conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = ?1",
                table
            ),
            [column],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false)
    }

    /// Migrate database schema from current to target version
    pub fn migrate_database(pool: &Pool<SqliteConnectionManager>, current_version: i32, target_version: i32) -> Result<()> {
        if current_version >= target_version {
            return Ok(());
        }

        let mut conn = pool.get()?;
        let tx = conn.transaction()?;

        if current_version < 2 {
            // Column already exists on fresh installs (CREATE TABLE includes it)
            if !Self::column_exists(&tx, "conf_character", "hide_from_dashboard") {
                tx.execute("ALTER TABLE conf_character ADD COLUMN hide_from_dashboard BOOLEAN DEFAULT 0", [])?;
            }
        }

        if current_version < 3 {
            // Only needed for databases where migration v2 created the column as TEXT
            if !Self::column_exists(&tx, "conf_character", "hide_from_dashboard_temp") {
                let needs_fix = Self::column_exists(&tx, "conf_character", "hide_from_dashboard");
                if needs_fix {
                    tx.execute("ALTER TABLE conf_character ADD COLUMN hide_from_dashboard_temp BOOLEAN DEFAULT 0", [])?;
                    tx.execute("UPDATE conf_character SET hide_from_dashboard_temp = CASE WHEN hide_from_dashboard = 'false' THEN 0 ELSE 1 END", [])?;
                    tx.execute("ALTER TABLE conf_character DROP COLUMN hide_from_dashboard", [])?;
                    tx.execute("ALTER TABLE conf_character RENAME COLUMN hide_from_dashboard_temp TO hide_from_dashboard", [])?;
                }
            }
        }

        tx.commit()?;
        Self::set_schema_version(pool, target_version)?;
        println!("Database migrated from version {} to {}", current_version, target_version);
        Ok(())
    }

    /// Initialize default application data from frontend data
    pub fn initialize_default_data(
        pool: &Pool<SqliteConnectionManager>,
        _tasks: HashMap<String, GameTask>,
        _raids: Vec<Raid>,
        _classes: HashMap<String, GameClass>,
    ) -> Result<()> {
        let mut conn = pool.get()?;
        let tx = conn.transaction()?;

        // Initialize app_state if empty
        let app_state_count: i64 = tx.query_row("SELECT COUNT(*) FROM app_state", [], |row| row.get(0))?;
        if app_state_count == 0 {
            tx.execute(
                "INSERT INTO app_state (last_daily_reset, last_weekly_reset) VALUES (?1, ?2)",
                params![0, 0],
            )?;
        }


        // Initialize app_metadata
        tx.execute(
            "INSERT OR REPLACE INTO app_metadata (key, value, timestamp, app_version) VALUES ('initial_setup', 'completed', ?1, '1.0.0')",
            params![chrono::Utc::now().timestamp_millis()],
        )?;

        tx.commit()?;
        println!("Default application data initialized");
        Ok(())
    }

    /// Update reset timestamps based on current time
    pub fn update_reset_timestamps(pool: &Pool<SqliteConnectionManager>) -> Result<()> {
        // Do not advance reset timestamps without actually performing a reset.
        // The reset schedule is handled by ResetService, and last_daily_reset
        // / last_weekly_reset should only change when a reset is executed.
        Ok(())
    }

    /// Ensure all characters have complete data in related tables
    pub fn ensure_character_data_complete(
        pool: &Pool<SqliteConnectionManager>,
        tasks: HashMap<String, GameTask>,
        raids: Vec<Raid>,
    ) -> Result<()> {
        
        // Get all characters first, then process in separate transaction
        let mut conn = pool.get()?;
        let mut stmt = conn.prepare("SELECT char_id, roster_id FROM conf_character")?;
        let character_rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        
        let mut characters = Vec::new();
        for char_result in character_rows {
            characters.push(char_result?);
        }
        
        for (char_id, roster_id) in &characters {
        }
        
        drop(stmt);
        drop(conn);
        
        // Now create a new transaction for the inserts
        let mut conn = pool.get()?;
        let tx = conn.transaction()?;
        
        // Initialize conf_raid entries for all characters
        let _total_raid_entries = 0;
        
        for (character_id, roster_id) in &characters {
            
            for raid in &raids {
                for gate in &raid.gates {                    
                    tx.execute(
                        "INSERT OR IGNORE INTO conf_raid (roster_id, char_id, content_id, gate, difficulty, take_gold, buy_box) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![roster_id, character_id, raid.id, gate.gate, raid.difficulty, 0, 0],
                    )?;
                    
                    let _total_raid_entries = 1; // Suppress unused warning
                }
            }
        }
        
        // Initialize conf_tracking entries for all characters
        
        let _total_task_entries = 0;
        
        for (character_id, roster_id) in &characters {
            
            
            // Process ALL tasks (both character and roster tasks)
            for (task_id, task) in tasks.iter() {
                
                
                tx.execute(
                    "INSERT OR IGNORE INTO conf_tracking (roster_id, char_id, content_id, is_tracked) VALUES (?1, ?2, ?3, ?4)",
                    params![roster_id, character_id, task_id, 1],
                )?;
                
                let _total_task_entries = 1; // Suppress unused warning
                
                // Initialize rested_values only for rested character tasks
                if task.category == "character" && task.logic_type == "rested" {
                    let max_value = task.max_rest_value.unwrap_or(100);
                    
                    
                    tx.execute(
                        "INSERT OR IGNORE INTO rested_values (roster_id, char_id, content_id, current_value, last_updated, max_value) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![roster_id, character_id, task_id, max_value, chrono::Utc::now().timestamp_millis(), max_value],
                    )?;
                }
            }
            
            // Also insert raid IDs into conf_tracking
            for raid in &raids {
                
                tx.execute(
                    "INSERT OR IGNORE INTO conf_tracking (roster_id, char_id, content_id, is_tracked) VALUES (?1, ?2, ?3, ?4)",
                    params![roster_id, character_id, raid.id, 1],
                )?;
                
                let _total_task_entries = 1; // Suppress unused warning
            }
        }
        
        tx.commit()?;
        Ok(())
    }

    /// Initialize data for a specific character
    pub fn initialize_character_data(
        pool: &Pool<SqliteConnectionManager>,
        character_id: i64,
        roster_id: &str,
        tasks: HashMap<String, GameTask>,
        raids: Vec<Raid>,
    ) -> Result<()> {
        let mut conn = pool.get()?;
        let tx = conn.transaction()?;

        // Initialize conf_tracking entries for character tasks
        for (task_id, task) in tasks.iter() {
            if task.category == "character" {
                tx.execute(
                    "INSERT OR IGNORE INTO conf_tracking (roster_id, char_id, content_id, is_tracked) VALUES (?1, ?2, ?3, ?4)",
                    params![roster_id, character_id, task_id, 1],
                )?;

                // Initialize rested_values for rested tasks
                if task.logic_type == "rested" {
                    let max_value = task.max_rest_value.unwrap_or(100);
                    tx.execute(
                        "INSERT OR IGNORE INTO rested_values (roster_id, char_id, content_id, current_value, last_updated, max_value) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![roster_id, character_id, task_id, max_value, chrono::Utc::now().timestamp_millis(), max_value],
                    )?;
                }
            }
        }

        // Initialize conf_raid entries
        
        for raid in &raids {
            for gate in &raid.gates {
                tx.execute(
                    "INSERT OR IGNORE INTO conf_raid (roster_id, char_id, content_id, gate, difficulty, take_gold, buy_box) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![roster_id, character_id, raid.id, gate.gate, raid.difficulty, 0, 0],
                )?;
            }
        }
        println!("=== END CONF_RAID INITIALIZATION ===");

        tx.commit()?;
        println!("Character {} data initialized", character_id);
        Ok(())
    }

    /// Get all game tasks configuration
    pub fn get_game_tasks() -> Result<HashMap<String, GameTask>> {
        let mut tasks = HashMap::new();
        
        // Daily tasks
        tasks.insert("gate".to_string(), GameTask {
            id: "gate".to_string(),
            name: "Chaos Gate".to_string(),
            category: "character".to_string(),
            reset_schedule: "daily".to_string(),
            logic_type: "calendar".to_string(),
            max_rest_value: None,
        });
        
        tasks.insert("boss".to_string(), GameTask {
            id: "boss".to_string(),
            name: "World Boss".to_string(),
            category: "character".to_string(),
            reset_schedule: "daily".to_string(),
            logic_type: "calendar".to_string(),
            max_rest_value: None,
        });
        
        tasks.insert("chaos".to_string(), GameTask {
            id: "chaos".to_string(),
            name: "Chaos Dungeon".to_string(),
            category: "character".to_string(),
            reset_schedule: "daily".to_string(),
            logic_type: "rested".to_string(),
            max_rest_value: Some(100),
        });
        
        tasks.insert("guardian".to_string(), GameTask {
            id: "guardian".to_string(),
            name: "Guardian Raid".to_string(),
            category: "character".to_string(),
            reset_schedule: "daily".to_string(),
            logic_type: "rested".to_string(),
            max_rest_value: Some(100),
        });
        
        // Weekly tasks
        tasks.insert("cube".to_string(), GameTask {
            id: "cube".to_string(),
            name: "Cube".to_string(),
            category: "roster".to_string(),
            reset_schedule: "weekly".to_string(),
            logic_type: "normal".to_string(),
            max_rest_value: None,
        });
        
        tasks.insert("paradise".to_string(), GameTask {
            id: "paradise".to_string(),
            name: "Paradise".to_string(),
            category: "roster".to_string(),
            reset_schedule: "weekly".to_string(),
            logic_type: "normal".to_string(),
            max_rest_value: None,
        });
        
        tasks.insert("shop".to_string(), GameTask {
            id: "shop".to_string(),
            name: "Weekly Shop".to_string(),
            category: "roster".to_string(),
            reset_schedule: "weekly".to_string(),
            logic_type: "normal".to_string(),
            max_rest_value: None,
        });
        
        tasks.insert("guild".to_string(), GameTask {
            id: "guild".to_string(),
            name: "Guild Shop".to_string(),
            category: "roster".to_string(),
            reset_schedule: "weekly".to_string(),
            logic_type: "normal".to_string(),
            max_rest_value: None,
        });
        
        Ok(tasks)
    }
}
