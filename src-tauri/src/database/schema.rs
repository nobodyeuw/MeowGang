/// Table creation SQL for the local `userlogs.db` bootstrap.
///
/// `DataManager::migrate_database` owns versioned migrations. These statements
/// only make sure a fresh database has the current base tables before migrations,
/// resets, repositories, and handlers start using them.
pub const TABLES: &[(&str, &str)] = &[
    (
        "sync_metadata",
        "CREATE TABLE IF NOT EXISTS sync_metadata (
            sync_id TEXT NOT NULL,
            table_name TEXT,
            record_id TEXT,
            operation TEXT,
            timestamp INTEGER,
            sync_status TEXT NOT NULL DEFAULT 'pending',
            source TEXT,
            data TEXT,
            PRIMARY KEY(sync_id)
        )",
    ),
    (
        "app_metadata",
        "CREATE TABLE IF NOT EXISTS app_metadata (
            key TEXT,
            value TEXT,
            timestamp INTEGER,
            app_version TEXT,
            PRIMARY KEY(key)
        )",
    ),
    (
        "completion_status",
        "CREATE TABLE IF NOT EXISTS completion_status (
            rowid INTEGER,
            roster_id TEXT NOT NULL,
            char_id INTEGER NOT NULL,
            content_id TEXT NOT NULL,
            is_completed INTEGER,
            completion_source TEXT DEFAULT 'manual',
            timestamp INTEGER,
            details TEXT,
            session_id TEXT,
            PRIMARY KEY(rowid),
            FOREIGN KEY(char_id) REFERENCES conf_character(char_id)
        )",
    ),
    (
        "conf_character",
        "CREATE TABLE IF NOT EXISTS conf_character (
            char_id INTEGER NOT NULL,
            char_name TEXT,
            roster_id TEXT NOT NULL,
            roster_name TEXT,
            class_id TEXT,
            item_level REAL,
            combat_power REAL,
            display_order TEXT,
            roster_display_order INTEGER DEFAULT 0,
            earns_gold BOOLEAN DEFAULT 0,
            hide_from_dashboard BOOLEAN DEFAULT 0,
            meow_connect_enabled BOOLEAN DEFAULT 0,
            removed_from_roster BOOLEAN DEFAULT 0,
            class_display_name TEXT,
            PRIMARY KEY(char_id)
        )",
    ),
    (
        "app_state",
        "CREATE TABLE IF NOT EXISTS app_state (
            last_daily_reset INTEGER,
            last_weekly_reset INTEGER
        )",
    ),
    (
        "conf_tracking",
        "CREATE TABLE IF NOT EXISTS conf_tracking (
            roster_id TEXT,
            char_id INTEGER,
            content_id TEXT,
            is_tracked INTEGER DEFAULT 1,
            lazy_daily INTEGER DEFAULT 0,
            UNIQUE(char_id, content_id)
        )",
    ),
    (
        "conf_raid",
        "CREATE TABLE IF NOT EXISTS conf_raid (
            roster_id TEXT,
            char_id INTEGER,
            content_id TEXT,
            gate TEXT,
            difficulty TEXT,
            take_gold INTEGER DEFAULT 0,
            buy_box INTEGER DEFAULT 0,
            reserved_for_static INTEGER DEFAULT 0,
            UNIQUE(char_id, content_id, gate)
        )",
    ),
    (
        "meow_group_raid_tags",
        "CREATE TABLE IF NOT EXISTS meow_group_raid_tags (
            char_id INTEGER NOT NULL,
            content_id TEXT NOT NULL,
            group_id TEXT NOT NULL,
            group_tag TEXT NOT NULL DEFAULT '',
            group_name TEXT NOT NULL DEFAULT '',
            updated_at INTEGER NOT NULL,
            PRIMARY KEY(char_id, content_id, group_id)
        )",
    ),
    (
        "rested_values",
        "CREATE TABLE IF NOT EXISTS rested_values (
            roster_id TEXT,
            char_id INTEGER,
            content_id TEXT,
            current_value INTEGER DEFAULT 0,
            last_updated INTEGER DEFAULT 0,
            max_value INTEGER DEFAULT 100,
            UNIQUE(char_id, content_id)
        )",
    ),
    (
        "character_engravings",
        "CREATE TABLE IF NOT EXISTS character_engravings (
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
        )",
    ),
    (
        "character_equipment",
        "CREATE TABLE IF NOT EXISTS character_equipment (
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
        )",
    ),
    (
        "character_gems",
        "CREATE TABLE IF NOT EXISTS character_gems (
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
        )",
    ),
    (
        "progression_goals",
        "CREATE TABLE IF NOT EXISTS progression_goals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            character_id INTEGER NOT NULL,
            goal_type TEXT NOT NULL,
            target_name TEXT NOT NULL,
            target_value INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            completed_at INTEGER,
            UNIQUE(character_id, goal_type, target_name),
            FOREIGN KEY(character_id) REFERENCES conf_character(char_id) ON DELETE CASCADE
        )",
    ),
];

/// Indexes used by repositories and dashboard/bootstrap snapshots.
///
/// Index failures are logged but do not abort startup because missing indexes
/// affect performance, not local data correctness.
pub const INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_sync_metadata_operation ON sync_metadata(operation)",
    "CREATE INDEX IF NOT EXISTS idx_sync_metadata_record_id ON sync_metadata(record_id)",
    "CREATE INDEX IF NOT EXISTS idx_sync_metadata_source ON sync_metadata(source)",
    "CREATE INDEX IF NOT EXISTS idx_sync_metadata_sync_status ON sync_metadata(sync_status)",
    "CREATE INDEX IF NOT EXISTS idx_sync_metadata_table_name ON sync_metadata(table_name)",
    "CREATE INDEX IF NOT EXISTS idx_sync_metadata_timestamp ON sync_metadata(timestamp)",
    "CREATE INDEX IF NOT EXISTS idx_app_metadata_key ON app_metadata(key)",
    "CREATE INDEX IF NOT EXISTS idx_app_metadata_timestamp ON app_metadata(timestamp)",
    "CREATE INDEX IF NOT EXISTS idx_completion_status_char_content ON completion_status(char_id, content_id)",
    "CREATE INDEX IF NOT EXISTS idx_completion_status_char_content_session ON completion_status(char_id, content_id, session_id)",
    "CREATE INDEX IF NOT EXISTS idx_completion_status_roster_content_session ON completion_status(roster_id, content_id, session_id)",
    "CREATE INDEX IF NOT EXISTS idx_completion_status_char_timestamp ON completion_status(char_id, timestamp)",
    "CREATE INDEX IF NOT EXISTS idx_conf_character_char_id ON conf_character(char_id)",
    "CREATE INDEX IF NOT EXISTS idx_conf_character_class ON conf_character(class_id)",
    "CREATE INDEX IF NOT EXISTS idx_conf_character_display_order ON conf_character(display_order)",
    "CREATE INDEX IF NOT EXISTS idx_conf_character_roster ON conf_character(roster_name)",
    "CREATE INDEX IF NOT EXISTS idx_conf_character_roster_display ON conf_character(roster_name, display_order)",
    "CREATE INDEX IF NOT EXISTS idx_conf_character_roster_id_display ON conf_character(roster_id, display_order)",
    "CREATE INDEX IF NOT EXISTS idx_conf_tracking_roster_char_content ON conf_tracking(roster_id, char_id, content_id)",
    "CREATE INDEX IF NOT EXISTS idx_rested_values_char_content ON rested_values(char_id, content_id)",
    "CREATE INDEX IF NOT EXISTS idx_conf_raid_char_content_gate_diff ON conf_raid(char_id, content_id, gate, difficulty)",
    "CREATE INDEX IF NOT EXISTS idx_meow_group_raid_tags_char_content ON meow_group_raid_tags(char_id, content_id)",
    "CREATE INDEX IF NOT EXISTS idx_character_engravings_char ON character_engravings(character_id)",
    "CREATE INDEX IF NOT EXISTS idx_character_equipment_char ON character_equipment(character_id)",
    "CREATE INDEX IF NOT EXISTS idx_character_gems_char ON character_gems(character_id)",
    "CREATE INDEX IF NOT EXISTS idx_character_gems_slot ON character_gems(character_id, slot_index)",
    "CREATE INDEX IF NOT EXISTS idx_progression_goals_char ON progression_goals(character_id)",
];
