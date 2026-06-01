use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

/// Runs versioned local SQLite migrations for `userlogs.db`.
///
/// Fresh table creation lives in `database/schema.rs`; this module only owns
/// changes needed by users upgrading from older app versions.
pub fn migrate_database(pool: &Pool<SqliteConnectionManager>, current_version: i32, target_version: i32) -> Result<()> {
    if current_version >= target_version {
        return Ok(());
    }

    let mut conn = pool.get()?;
    let tx = conn.transaction()?;

    if current_version < 2 {
        if !column_exists(&tx, "conf_character", "hide_from_dashboard") {
            tx.execute(
                "ALTER TABLE conf_character ADD COLUMN hide_from_dashboard BOOLEAN DEFAULT 0",
                [],
            )?;
        }
    }

    if current_version < 3 {
        if !column_exists(&tx, "conf_character", "hide_from_dashboard_temp") {
            let needs_fix = column_exists(&tx, "conf_character", "hide_from_dashboard");
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

    if current_version < 7 && !column_exists(&tx, "character_equipment", "effects_json") {
        tx.execute("ALTER TABLE character_equipment ADD COLUMN effects_json TEXT", [])?;
    }

    if current_version < 8 && !column_exists(&tx, "conf_character", "roster_display_order") {
        tx.execute(
            "ALTER TABLE conf_character ADD COLUMN roster_display_order INTEGER DEFAULT 0",
            [],
        )?;
    }

    if current_version < 9 && !column_exists(&tx, "conf_tracking", "lazy_daily") {
        tx.execute("ALTER TABLE conf_tracking ADD COLUMN lazy_daily INTEGER DEFAULT 0", [])?;
    }

    if current_version < 10 {
        tx.execute("DROP TABLE IF EXISTS gold_logs", [])?;
    }

    if current_version < 11 {
        normalize_roster_wide_tasks(&tx)?;
    }

    if current_version < 12 && !column_exists(&tx, "conf_raid", "reserved_for_static") {
        tx.execute(
            "ALTER TABLE conf_raid ADD COLUMN reserved_for_static INTEGER DEFAULT 0",
            [],
        )?;
    }

    if current_version < 13 && !column_exists(&tx, "conf_character", "meow_connect_enabled") {
        tx.execute(
            "ALTER TABLE conf_character ADD COLUMN meow_connect_enabled BOOLEAN DEFAULT 0",
            [],
        )?;
    }

    if current_version < 14 && !column_exists(&tx, "conf_character", "class_display_name") {
        tx.execute("ALTER TABLE conf_character ADD COLUMN class_display_name TEXT", [])?;
    }

    if current_version < 15 && !column_exists(&tx, "conf_character", "removed_from_roster") {
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
    crate::database::data_manager::DataManager::set_schema_version(pool, target_version)?;
    crate::log_info!(
        "Database migrated from version {} to {}",
        current_version,
        target_version
    );
    Ok(())
}

/// Check whether a column exists on a table within the active migration transaction.
fn column_exists(conn: &rusqlite::Connection, table: &str, column: &str) -> bool {
    conn.query_row(
        &format!("SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = ?1", table),
        [column],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count > 0)
    .unwrap_or(false)
}

/// Backfills older roster-wide calendar task rows into the current tracking model.
///
/// Roster-wide tasks such as Chaos Gate and Field Boss are represented by one
/// completion row per roster, but tracking rows need to exist for each character
/// so the settings matrix and To Do matrix stay complete.
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
