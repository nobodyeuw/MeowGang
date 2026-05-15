use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

/// Manages the SQLite database connection pool and schema migrations.
///
/// Uses r2d2 connection pooling with WAL mode for concurrent access.
#[derive(Clone)]
pub struct DatabaseManager {
    pub pool: Pool<SqliteConnectionManager>,
}

impl DatabaseManager {
    /// Opens (or creates) the database at `db_path` and runs any pending migrations.
    pub fn new(db_path: &str) -> Result<Self> {
        crate::log_info!("Initializing database manager with path: {}", db_path);
        
        let manager = SqliteConnectionManager::file(db_path)
            .with_init(|c| {
                c.execute_batch("
                    PRAGMA journal_mode = WAL;          -- Enables concurrent read/write
                    PRAGMA synchronous = NORMAL;       -- Faster write operations
                    PRAGMA foreign_keys = ON;          -- Data integrity
                    PRAGMA busy_timeout = 5000;        -- Wait up to 5 seconds if locked
                    PRAGMA cache_size = 10000;          -- Larger cache for better performance
                    PRAGMA temp_store = MEMORY;         -- Store temp tables in memory
                    PRAGMA mmap_size = 268435456;       -- 256MB memory-mapped I/O
                ").unwrap_or_else(|e| {
                    crate::log_warn!("Failed to set SQLite pragmas: {}", e);
                });
                crate::log_debug!("Database connection initialized with SQLite pragmas");
                Ok(())
            });
        let pool = Pool::new(manager)?;
        
        let db = Self { pool };
        db.initialize_missing_tables()?;
        crate::log_info!("Database manager initialized successfully");
        Ok(db)
    }

    fn initialize_missing_tables(&self) -> Result<()> {
        crate::log_debug!("Checking and creating missing database tables");
        let conn = self.pool.get()?;
        
        // Create tables exactly matching the original schema
        let tables_to_create = vec![
            ("sync_metadata",
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
                )"),
            ("app_metadata",
                "CREATE TABLE IF NOT EXISTS app_metadata (
                    key TEXT,
                    value TEXT,
                    timestamp INTEGER,
                    app_version TEXT,
                    PRIMARY KEY(key)
                )"),
            ("completion_status",
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
                )"),
            ("conf_character",
                "CREATE TABLE IF NOT EXISTS conf_character (
                    char_id INTEGER NOT NULL,
                    char_name TEXT,
                    roster_id TEXT NOT NULL,
                    roster_name TEXT,
                    class_id TEXT,
                    item_level REAL,
                    combat_power REAL,
                    display_order TEXT,
                    earns_gold BOOLEAN DEFAULT 0,
                    hide_from_dashboard BOOLEAN DEFAULT 0,
                    PRIMARY KEY(char_id)
                )"),
            ("app_state",
                "CREATE TABLE IF NOT EXISTS app_state (
                    last_daily_reset INTEGER,
                    last_weekly_reset INTEGER
                )"),
            ("gold_logs",
                "CREATE TABLE IF NOT EXISTS gold_logs (
                    timestamp INTEGER NOT NULL,
                    rowid INTEGER,
                    char_id INTEGER NOT NULL,
                    source TEXT,
                    gold_value_total INTEGER NOT NULL,
                    gold_bound INTEGER,
                    gold_tradable INTEGER,
                    notes TEXT,
                    PRIMARY KEY(rowid AUTOINCREMENT)
                )"),
            ("conf_tracking",
                "CREATE TABLE IF NOT EXISTS conf_tracking (
                    roster_id TEXT,
                    char_id INTEGER,
                    content_id TEXT,
                    is_tracked INTEGER DEFAULT 1,
                    UNIQUE(char_id, content_id)
                )"),
            ("conf_raid",
                "CREATE TABLE IF NOT EXISTS conf_raid (
                    roster_id TEXT,
                    char_id INTEGER,
                    content_id TEXT,
                    gate TEXT,
                    difficulty TEXT,
                    take_gold INTEGER DEFAULT 0,
                    buy_box INTEGER DEFAULT 0,
                    UNIQUE(char_id, content_id, gate)
                )"),
            ("rested_values",
                "CREATE TABLE IF NOT EXISTS rested_values (
                    roster_id TEXT,
                    char_id INTEGER,
                    content_id TEXT,
                    current_value INTEGER DEFAULT 0,
                    last_updated INTEGER DEFAULT 0,
                    max_value INTEGER DEFAULT 100,
                    UNIQUE(char_id, content_id)
                )"),
        ];
        
        for (table_name, create_sql) in &tables_to_create {
            crate::log_debug!("Creating table: {}", table_name);
            match conn.execute(create_sql, []) {
                Ok(_) => crate::log_debug!("Table {} created or verified successfully", table_name),
                Err(e) => {
                    crate::log_error!("Failed to create table {}: {}", table_name, e);
                    return Err(e.into());
                }
            }
        }
        
        crate::log_info!("All database tables initialized successfully");
        
        // Add indexes matching the original schema
        let indexes = vec![
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
            "CREATE INDEX IF NOT EXISTS idx_gold_logs_char_content ON gold_logs (char_id, source)",
            "CREATE INDEX IF NOT EXISTS idx_gold_logs_char_id ON gold_logs(char_id)",
            "CREATE INDEX IF NOT EXISTS idx_gold_logs_char_timestamp ON gold_logs(char_id, timestamp)",
            "CREATE INDEX IF NOT EXISTS idx_gold_logs_notes ON gold_logs(notes)",
            "CREATE INDEX IF NOT EXISTS idx_gold_logs_source ON gold_logs(source)",
            "CREATE INDEX IF NOT EXISTS idx_gold_logs_timestamp ON gold_logs(timestamp)",
            "CREATE INDEX IF NOT EXISTS idx_gold_logs_timestamp_char ON gold_logs(timestamp, char_id)",
        ];

        crate::log_debug!("Creating database indexes");
        for index_sql in indexes {
            match conn.execute(index_sql, []) {
                Ok(_) => {}, // Index creation successful, no need to log each one
                Err(e) => {
                    crate::log_warn!("Failed to create database index: {}", e);
                    // Don't fail the whole initialization for index errors
                }
            }
        }

        crate::log_info!("Database initialization completed successfully");
        Ok(())
    }

    
    pub fn get_connection(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>> {
        self.pool.get().map_err(|e| {
            crate::log_error!("Failed to get database connection: {}", e);
            anyhow::anyhow!("Failed to get database connection: {}", e)
        })
    }
}
