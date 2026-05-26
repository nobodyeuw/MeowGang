use anyhow::Result;
use chrono::Datelike;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameTask {
    pub id: String,
    pub name: String,
    pub category: String,       // "roster" | "character"
    pub reset_schedule: String, // "daily" | "weekly"
    pub logic_type: String,     // "normal" | "calendar" | "rested"
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
            gates: dm_raid
                .gates
                .into_iter()
                .map(|g| crate::models::RaidGate {
                    gate: g.gate.clone(),
                    name: g.gate,
                    min_ilvl: g.min_ilvl as i64,
                    tradable_gold: Some(g.tradable_gold),
                    bound_gold: Some(g.bound_gold),
                    box_price: Some(g.box_price),
                })
                .collect(),
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
                value_str.parse::<i32>().map_err(|_| {
                    rusqlite::Error::InvalidColumnType(0, "value".to_string(), rusqlite::types::Type::Text)
                })
            },
        ) {
            Ok(version) => Ok(version),
            Err(_) => Ok(1), // Default version 1
        }
    }

    /// Set schema version in app_metadata
    pub fn set_schema_version(pool: &Pool<SqliteConnectionManager>, version: i32) -> Result<()> {
        let conn = pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO app_metadata (key, value, timestamp, app_version) VALUES ('schema_version', ?1, ?2, ?3)",
            params![version.to_string(), chrono::Utc::now().timestamp_millis(), crate::version::APP_VERSION],
        )?;
        Ok(())
    }

    /// Check whether a column exists on a table (uses the same connection/transaction).
    fn column_exists(conn: &rusqlite::Connection, table: &str, column: &str) -> bool {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = ?1", table),
            [column],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false)
    }

    fn normalize_roster_wide_tasks(tx: &rusqlite::Transaction) -> Result<()> {
        let mut roster_stmt = tx.prepare(
            "SELECT DISTINCT roster_id
             FROM conf_character
             WHERE roster_id IS NOT NULL AND roster_id <> ''",
        )?;
        let roster_rows = roster_stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut roster_ids = Vec::new();
        for roster_id in roster_rows {
            roster_ids.push(roster_id?);
        }
        drop(roster_stmt);

        for roster_id in roster_ids {
            let canonical_char_id: i64 = match tx.query_row(
                "SELECT char_id
                 FROM conf_character
                 WHERE roster_id = ?1
                 ORDER BY CAST(display_order AS INTEGER), char_name, char_id
                 LIMIT 1",
                [&roster_id],
                |row| row.get(0),
            ) {
                Ok(char_id) => char_id,
                Err(_) => continue,
            };

            for task_id in ["gate", "boss"] {
                let tracked_value = tx
                    .query_row(
                        "SELECT is_tracked
                         FROM conf_tracking
                         WHERE roster_id = ?1
                           AND char_id = ?2
                           AND content_id = ?3
                         LIMIT 1",
                        params![&roster_id, canonical_char_id, task_id],
                        |row| row.get::<_, i64>(0),
                    )
                    .or_else(|_| {
                        tx.query_row(
                            "SELECT COALESCE(MAX(is_tracked), 1)
                             FROM conf_tracking
                             WHERE roster_id = ?1
                               AND content_id = ?2",
                            params![&roster_id, task_id],
                            |row| row.get::<_, i64>(0),
                        )
                    })
                    .unwrap_or(1);

                tx.execute(
                    "INSERT INTO conf_tracking (roster_id, char_id, content_id, is_tracked, lazy_daily)
                     SELECT roster_id, char_id, ?2, ?3, 0
                     FROM conf_character
                     WHERE roster_id = ?1
                     ON CONFLICT(char_id, content_id) DO UPDATE SET
                       roster_id = excluded.roster_id,
                       is_tracked = excluded.is_tracked",
                    params![&roster_id, task_id, tracked_value],
                )?;

                let latest_completion = tx
                    .query_row(
                        "SELECT is_completed, COALESCE(timestamp, 0)
                         FROM completion_status
                         WHERE roster_id = ?1
                           AND content_id = ?2
                           AND session_id IS NULL
                         ORDER BY timestamp DESC, rowid DESC
                         LIMIT 1",
                        params![&roster_id, task_id],
                        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                    )
                    .ok();

                if let Some((is_completed, timestamp)) = latest_completion {
                    tx.execute(
                        "DELETE FROM completion_status
                         WHERE roster_id = ?1
                           AND content_id = ?2
                           AND session_id IS NULL",
                        params![&roster_id, task_id],
                    )?;

                    tx.execute(
                        "INSERT INTO completion_status
                            (roster_id, char_id, content_id, is_completed, completion_source, timestamp, session_id)
                         VALUES (?1, ?2, ?3, ?4, 'manual', ?5, NULL)",
                        params![&roster_id, canonical_char_id, task_id, is_completed, timestamp],
                    )?;
                }
            }
        }

        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_completion_roster_task_unique
             ON completion_status(roster_id, content_id)
             WHERE session_id IS NULL AND content_id IN ('gate', 'boss')",
            [],
        )?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_completion_status_roster_content_session
             ON completion_status(roster_id, content_id, session_id)",
            [],
        )?;

        Ok(())
    }

    /// Migrate database schema from current to target version
    pub fn migrate_database(
        pool: &Pool<SqliteConnectionManager>,
        current_version: i32,
        target_version: i32,
    ) -> Result<()> {
        if current_version >= target_version {
            return Ok(());
        }

        let mut conn = pool.get()?;
        let tx = conn.transaction()?;

        if current_version < 2 {
            // Column already exists on fresh installs (CREATE TABLE includes it)
            if !Self::column_exists(&tx, "conf_character", "hide_from_dashboard") {
                tx.execute(
                    "ALTER TABLE conf_character ADD COLUMN hide_from_dashboard BOOLEAN DEFAULT 0",
                    [],
                )?;
            }
        }

        if current_version < 3 {
            // Only needed for databases where migration v2 created the column as TEXT
            if !Self::column_exists(&tx, "conf_character", "hide_from_dashboard_temp") {
                let needs_fix = Self::column_exists(&tx, "conf_character", "hide_from_dashboard");
                if needs_fix {
                    tx.execute(
                        "ALTER TABLE conf_character ADD COLUMN hide_from_dashboard_temp BOOLEAN DEFAULT 0",
                        [],
                    )?;
                    tx.execute("UPDATE conf_character SET hide_from_dashboard_temp = CASE WHEN hide_from_dashboard = 'false' THEN 0 ELSE 1 END", [])?;
                    tx.execute("ALTER TABLE conf_character DROP COLUMN hide_from_dashboard", [])?;
                    tx.execute(
                        "ALTER TABLE conf_character RENAME COLUMN hide_from_dashboard_temp TO hide_from_dashboard",
                        [],
                    )?;
                }
            }
        }

        if current_version < 4 {
            tx.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS character_engravings (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    character_id INTEGER NOT NULL,
                    engraving_name TEXT NOT NULL,
                    books_read INTEGER NOT NULL DEFAULT 0,
                    max_books INTEGER NOT NULL DEFAULT 20,
                    stone_bonus INTEGER NOT NULL DEFAULT 0,
                    is_manual_entry INTEGER NOT NULL DEFAULT 0,
                    updated_at INTEGER NOT NULL,
                    UNIQUE(character_id, engraving_name),
                    FOREIGN KEY(character_id) REFERENCES conf_character(char_id) ON DELETE CASCADE
                );
                CREATE TABLE IF NOT EXISTS character_equipment (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    character_id INTEGER NOT NULL,
                    slot TEXT NOT NULL,
                    enhancement_level INTEGER,
                    tier TEXT,
                    quality INTEGER,
                    item_level REAL,
                    effects_json TEXT,
                    is_manual_entry INTEGER NOT NULL DEFAULT 0,
                    updated_at INTEGER NOT NULL,
                    UNIQUE(character_id, slot),
                    FOREIGN KEY(character_id) REFERENCES conf_character(char_id) ON DELETE CASCADE
                );
                CREATE TABLE IF NOT EXISTS character_gems (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    character_id INTEGER NOT NULL,
                    skill_name TEXT NOT NULL,
                    gem_type TEXT NOT NULL,
                    gem_level INTEGER NOT NULL,
                    is_manual_entry INTEGER NOT NULL DEFAULT 0,
                    updated_at INTEGER NOT NULL,
                    UNIQUE(character_id, skill_name, gem_type),
                    FOREIGN KEY(character_id) REFERENCES conf_character(char_id) ON DELETE CASCADE
                );
                CREATE TABLE IF NOT EXISTS progression_goals (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    character_id INTEGER NOT NULL,
                    goal_type TEXT NOT NULL,
                    target_name TEXT NOT NULL,
                    target_value INTEGER NOT NULL,
                    created_at INTEGER NOT NULL,
                    completed_at INTEGER,
                    UNIQUE(character_id, goal_type, target_name),
                    FOREIGN KEY(character_id) REFERENCES conf_character(char_id) ON DELETE CASCADE
                );
                CREATE INDEX IF NOT EXISTS idx_character_engravings_char ON character_engravings(character_id);
                CREATE INDEX IF NOT EXISTS idx_character_equipment_char ON character_equipment(character_id);
                CREATE INDEX IF NOT EXISTS idx_character_gems_char ON character_gems(character_id);
                CREATE INDEX IF NOT EXISTS idx_progression_goals_char ON progression_goals(character_id);
                "#,
            )?;
        }

        if current_version < 5 {
            // Rebuild character_gems with new schema:
            // - unique key changed from (character_id, skill_name, gem_type) to (character_id, slot_index)
            // - added columns: slot_index, gem_name, is_bound
            // SQLite cannot drop constraints, so we recreate the table.
            tx.execute_batch(
                r#"
                DROP TABLE IF EXISTS character_gems;
                CREATE TABLE character_gems (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    character_id INTEGER NOT NULL,
                    slot_index INTEGER NOT NULL DEFAULT 0,
                    gem_name TEXT NOT NULL DEFAULT '',
                    gem_item_id INTEGER,
                    skill_id INTEGER,
                    skill_name TEXT NOT NULL,
                    skill_icon TEXT,
                    gem_type TEXT NOT NULL,
                    gem_level INTEGER NOT NULL,
                    effect_value REAL,
                    is_bound INTEGER NOT NULL DEFAULT 0,
                    is_manual_entry INTEGER NOT NULL DEFAULT 0,
                    updated_at INTEGER NOT NULL,
                    UNIQUE(character_id, slot_index),
                    FOREIGN KEY(character_id) REFERENCES conf_character(char_id) ON DELETE CASCADE
                );
                CREATE INDEX IF NOT EXISTS idx_character_gems_char ON character_gems(character_id);
                CREATE INDEX IF NOT EXISTS idx_character_gems_slot ON character_gems(character_id, slot_index);
                "#,
            )?;
        }

        if current_version >= 5 && current_version < 6 {
            tx.execute_batch(
                r#"
                ALTER TABLE character_gems ADD COLUMN gem_item_id INTEGER;
                ALTER TABLE character_gems ADD COLUMN skill_id INTEGER;
                ALTER TABLE character_gems ADD COLUMN skill_icon TEXT;
                ALTER TABLE character_gems ADD COLUMN effect_value REAL;
                "#,
            )?;
        }

        if current_version < 7 && !Self::column_exists(&tx, "character_equipment", "effects_json") {
            tx.execute("ALTER TABLE character_equipment ADD COLUMN effects_json TEXT", [])?;
        }

        if current_version < 8 && !Self::column_exists(&tx, "conf_character", "roster_display_order") {
            tx.execute(
                "ALTER TABLE conf_character ADD COLUMN roster_display_order INTEGER DEFAULT 0",
                [],
            )?;
        }

        if current_version < 9 && !Self::column_exists(&tx, "conf_tracking", "lazy_daily") {
            tx.execute("ALTER TABLE conf_tracking ADD COLUMN lazy_daily INTEGER DEFAULT 0", [])?;
        }

        if current_version < 10 {
            tx.execute("DROP TABLE IF EXISTS gold_logs", [])?;
        }

        if current_version < 11 {
            Self::normalize_roster_wide_tasks(&tx)?;
        }

        if current_version < 12 && !Self::column_exists(&tx, "conf_raid", "reserved_for_static") {
            tx.execute(
                "ALTER TABLE conf_raid ADD COLUMN reserved_for_static INTEGER DEFAULT 0",
                [],
            )?;
        }

        if current_version < 13 && !Self::column_exists(&tx, "conf_character", "meow_connect_enabled") {
            tx.execute(
                "ALTER TABLE conf_character ADD COLUMN meow_connect_enabled BOOLEAN DEFAULT 0",
                [],
            )?;
        }

        if current_version < 14 && !Self::column_exists(&tx, "conf_character", "class_display_name") {
            tx.execute(
                "ALTER TABLE conf_character ADD COLUMN class_display_name TEXT",
                [],
            )?;
        }

        if current_version < 15 && !Self::column_exists(&tx, "conf_character", "removed_from_roster") {
            tx.execute(
                "ALTER TABLE conf_character ADD COLUMN removed_from_roster BOOLEAN DEFAULT 0",
                [],
            )?;
        }

        if current_version < 16 {
            tx.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS meow_group_raid_tags (
                    char_id INTEGER NOT NULL,
                    content_id TEXT NOT NULL,
                    group_id TEXT NOT NULL,
                    group_tag TEXT NOT NULL DEFAULT '',
                    group_name TEXT NOT NULL DEFAULT '',
                    updated_at INTEGER NOT NULL,
                    PRIMARY KEY(char_id, content_id, group_id)
                );
                CREATE INDEX IF NOT EXISTS idx_meow_group_raid_tags_char_content
                  ON meow_group_raid_tags(char_id, content_id);
                "#,
            )?;
        }

        tx.commit()?;
        Self::set_schema_version(pool, target_version)?;
        println!(
            "Database migrated from version {} to {}",
            current_version, target_version
        );
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
            "INSERT OR REPLACE INTO app_metadata (key, value, timestamp, app_version) VALUES ('initial_setup', 'completed', ?1, ?2)",
            params![chrono::Utc::now().timestamp_millis(), crate::version::APP_VERSION],
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
        let character_rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?;

        let mut characters = Vec::new();
        for char_result in character_rows {
            characters.push(char_result?);
        }

        for (char_id, roster_id) in &characters {}

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
        tasks.insert(
            "gate".to_string(),
            GameTask {
                id: "gate".to_string(),
                name: "Chaos Gate".to_string(),
                category: "character".to_string(),
                reset_schedule: "daily".to_string(),
                logic_type: "calendar".to_string(),
                max_rest_value: None,
            },
        );

        tasks.insert(
            "boss".to_string(),
            GameTask {
                id: "boss".to_string(),
                name: "World Boss".to_string(),
                category: "character".to_string(),
                reset_schedule: "daily".to_string(),
                logic_type: "calendar".to_string(),
                max_rest_value: None,
            },
        );

        tasks.insert(
            "event_argeos_winter".to_string(),
            GameTask {
                id: "event_argeos_winter".to_string(),
                name: "Argeos Winter Event".to_string(),
                category: "roster".to_string(),
                reset_schedule: "weekly".to_string(),
                logic_type: "normal".to_string(),
                max_rest_value: None,
            },
        );

        tasks.insert(
            "chaos".to_string(),
            GameTask {
                id: "chaos".to_string(),
                name: "Chaos Dungeon".to_string(),
                category: "character".to_string(),
                reset_schedule: "daily".to_string(),
                logic_type: "rested".to_string(),
                max_rest_value: Some(100),
            },
        );

        tasks.insert(
            "guardian".to_string(),
            GameTask {
                id: "guardian".to_string(),
                name: "Guardian Raid".to_string(),
                category: "character".to_string(),
                reset_schedule: "daily".to_string(),
                logic_type: "rested".to_string(),
                max_rest_value: Some(100),
            },
        );

        // Weekly tasks
        tasks.insert(
            "cube".to_string(),
            GameTask {
                id: "cube".to_string(),
                name: "Cube".to_string(),
                category: "roster".to_string(),
                reset_schedule: "weekly".to_string(),
                logic_type: "normal".to_string(),
                max_rest_value: None,
            },
        );

        tasks.insert(
            "paradise".to_string(),
            GameTask {
                id: "paradise".to_string(),
                name: "Paradise".to_string(),
                category: "roster".to_string(),
                reset_schedule: "weekly".to_string(),
                logic_type: "normal".to_string(),
                max_rest_value: None,
            },
        );

        tasks.insert(
            "shop".to_string(),
            GameTask {
                id: "shop".to_string(),
                name: "Weekly Shop".to_string(),
                category: "roster".to_string(),
                reset_schedule: "weekly".to_string(),
                logic_type: "normal".to_string(),
                max_rest_value: None,
            },
        );

        tasks.insert(
            "guild".to_string(),
            GameTask {
                id: "guild".to_string(),
                name: "Guild Shop".to_string(),
                category: "roster".to_string(),
                reset_schedule: "weekly".to_string(),
                logic_type: "normal".to_string(),
                max_rest_value: None,
            },
        );

        Ok(tasks)
    }
}
