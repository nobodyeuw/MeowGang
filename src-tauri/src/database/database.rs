use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

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

        let manager = SqliteConnectionManager::file(db_path).with_init(|c| {
            c.execute_batch(
                "
                    PRAGMA journal_mode = WAL;          -- Enables concurrent read/write
                    PRAGMA synchronous = NORMAL;       -- Faster write operations
                    PRAGMA foreign_keys = ON;          -- Data integrity
                    PRAGMA busy_timeout = 5000;        -- Wait up to 5 seconds if locked
                    PRAGMA cache_size = 10000;          -- Larger cache for better performance
                    PRAGMA temp_store = MEMORY;         -- Store temp tables in memory
                    PRAGMA mmap_size = 268435456;       -- 256MB memory-mapped I/O
                ",
            )
            .unwrap_or_else(|e| {
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

        for (table_name, create_sql) in crate::database::schema::TABLES {
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

        crate::log_debug!("Creating database indexes");
        for index_sql in crate::database::schema::INDEXES {
            match conn.execute(index_sql, []) {
                Ok(_) => {} // Index creation successful, no need to log each one
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
