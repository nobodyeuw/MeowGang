use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// Market item with latest price from the LOA Buddy API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketItem {
    pub item_slug: String,
    pub item_name: String,
    pub category: String,
    pub price: i64,
    pub fetched_at: i64,
    pub is_manual_override: bool,
    pub favorite: bool,
}

/// Manages the separate `market.db` database for progression planner data.
#[derive(Clone)]
pub struct MarketDatabase {
    pub pool: Pool<SqliteConnectionManager>,
}

impl MarketDatabase {
    /// Opens (or creates) the market database at the given path.
    pub fn new(db_path: &str) -> Result<Self> {
        crate::log_info!("Initializing market database at: {}", db_path);

        let manager = SqliteConnectionManager::file(db_path).with_init(|c| {
            c.execute_batch(
                "
                    PRAGMA journal_mode = WAL;
                    PRAGMA synchronous = NORMAL;
                    PRAGMA foreign_keys = ON;
                    PRAGMA busy_timeout = 5000;
                    PRAGMA cache_size = 5000;
                    PRAGMA temp_store = MEMORY;
                ",
            )
            .unwrap_or_else(|e| {
                crate::log_warn!("Failed to set market DB pragmas: {}", e);
            });
            Ok(())
        });
        let pool = Pool::new(manager)?;

        let db = Self { pool };
        db.initialize_tables()?;
        crate::log_info!("Market database initialized successfully");
        Ok(db)
    }

    fn initialize_tables(&self) -> Result<()> {
        let conn = self.pool.get()?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS market_prices (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                item_slug TEXT NOT NULL,
                item_name TEXT NOT NULL,
                category TEXT NOT NULL,
                price INTEGER NOT NULL,
                fetched_at INTEGER NOT NULL,
                favorite INTEGER NOT NULL DEFAULT 0,
                UNIQUE(item_slug)
            );

            CREATE TABLE IF NOT EXISTS manual_price_overrides (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                item_slug TEXT NOT NULL,
                item_name TEXT NOT NULL,
                category TEXT NOT NULL,
                price INTEGER NOT NULL,
                favorite INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL,
                UNIQUE(item_slug)
            );

            CREATE TABLE IF NOT EXISTS market_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
        ",
        )?;

        // Set defaults if not present
        conn.execute(
            "INSERT OR IGNORE INTO market_settings (key, value) VALUES (?1, ?2)",
            params!["last_full_refresh", "0"],
        )?;

        self.ensure_market_columns(&conn)?;

        Ok(())
    }

    fn ensure_market_columns(&self, conn: &Connection) -> Result<()> {
        self.ensure_column_exists(conn, "market_prices", "favorite INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column_exists(conn, "manual_price_overrides", "favorite INTEGER NOT NULL DEFAULT 0")?;
        Ok(())
    }

    fn ensure_column_exists(&self, conn: &Connection, table: &str, column_def: &str) -> Result<()> {
        let column_name = column_def.split_whitespace().next().unwrap_or("");
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
        let mut rows = stmt.query([])?;
        let mut found = false;

        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == column_name {
                found = true;
                break;
            }
        }

        if !found {
            conn.execute(&format!("ALTER TABLE {} ADD COLUMN {}", table, column_def), [])?;
        }

        Ok(())
    }

    /// Upsert a batch of market prices from the API.
    pub fn upsert_prices(&self, items: &[(String, String, String, i64)], fetched_at: i64) -> Result<usize> {
        let conn = self.pool.get()?;
        let mut count = 0;

        let mut stmt = conn.prepare_cached(
            "INSERT INTO market_prices (item_slug, item_name, category, price, fetched_at, favorite)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(item_slug) DO UPDATE SET
                item_name = excluded.item_name,
                category = excluded.category,
                price = excluded.price,
                fetched_at = excluded.fetched_at",
        )?;

        for (slug, name, category, price) in items {
            stmt.execute(params![slug, name, category, price, fetched_at, 0])?;
            count += 1;
        }

        crate::log_info!("Upserted {} market prices", count);
        Ok(count)
    }

    /// Get effective price for an item (manual override takes priority).
    pub fn get_effective_price(&self, item_slug: &str) -> Result<Option<MarketItem>> {
        let conn = self.pool.get()?;

        // Check manual override first
        let manual: Option<MarketItem> = conn
            .query_row(
                "SELECT item_slug, item_name, category, price, favorite, updated_at
             FROM manual_price_overrides WHERE item_slug = ?1",
                params![item_slug],
                |row| {
                    Ok(MarketItem {
                        item_slug: row.get(0)?,
                        item_name: row.get(1)?,
                        category: row.get(2)?,
                        price: row.get(3)?,
                        favorite: row.get::<_, i32>(4)? == 1,
                        fetched_at: row.get(5)?,
                        is_manual_override: true,
                    })
                },
            )
            .ok();

        if manual.is_some() {
            return Ok(manual);
        }

        // Fall back to API price
        let market: Option<MarketItem> = conn
            .query_row(
                "SELECT item_slug, item_name, category, price, favorite, fetched_at
             FROM market_prices WHERE item_slug = ?1",
                params![item_slug],
                |row| {
                    Ok(MarketItem {
                        item_slug: row.get(0)?,
                        item_name: row.get(1)?,
                        category: row.get(2)?,
                        price: row.get(3)?,
                        favorite: row.get::<_, i32>(4)? == 1,
                        fetched_at: row.get(5)?,
                        is_manual_override: false,
                    })
                },
            )
            .ok();

        Ok(market)
    }

    /// Get all prices for a category, with manual overrides applied.
    pub fn get_prices_by_category(&self, category: &str) -> Result<Vec<MarketItem>> {
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare(
            "SELECT
                m.item_slug,
                COALESCE(o.item_name, m.item_name) AS item_name,
                COALESCE(o.category, m.category) AS category,
                COALESCE(o.price, m.price) AS price,
                COALESCE(o.favorite, m.favorite, 0) AS favorite,
                COALESCE(o.updated_at, m.fetched_at) AS fetched_at,
                CASE WHEN o.item_slug IS NOT NULL THEN 1 ELSE 0 END AS is_manual
             FROM market_prices m
             LEFT JOIN manual_price_overrides o ON m.item_slug = o.item_slug
             WHERE m.category = ?1
             ORDER BY m.item_name ASC",
        )?;

        let items = stmt
            .query_map(params![category], |row| {
                Ok(MarketItem {
                    item_slug: row.get(0)?,
                    item_name: row.get(1)?,
                    category: row.get(2)?,
                    price: row.get(3)?,
                    favorite: row.get::<_, i32>(4)? == 1,
                    fetched_at: row.get(5)?,
                    is_manual_override: row.get::<_, i32>(6)? == 1,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(items)
    }

    /// Get all prices across all categories.
    pub fn get_all_prices(&self) -> Result<Vec<MarketItem>> {
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare(
            "SELECT
                m.item_slug,
                COALESCE(o.item_name, m.item_name) AS item_name,
                COALESCE(o.category, m.category) AS category,
                COALESCE(o.price, m.price) AS price,
                COALESCE(o.favorite, m.favorite, 0) AS favorite,
                COALESCE(o.updated_at, m.fetched_at) AS fetched_at,
                CASE WHEN o.item_slug IS NOT NULL THEN 1 ELSE 0 END AS is_manual
             FROM market_prices m
             LEFT JOIN manual_price_overrides o ON m.item_slug = o.item_slug
             ORDER BY m.category ASC, m.item_name ASC",
        )?;

        let items = stmt
            .query_map([], |row| {
                Ok(MarketItem {
                    item_slug: row.get(0)?,
                    item_name: row.get(1)?,
                    category: row.get(2)?,
                    price: row.get(3)?,
                    favorite: row.get::<_, i32>(4)? == 1,
                    fetched_at: row.get(5)?,
                    is_manual_override: row.get::<_, i32>(6)? == 1,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(items)
    }

    /// Set a manual price override for an item.
    pub fn set_manual_price(&self, item_slug: &str, item_name: &str, category: &str, price: i64) -> Result<()> {
        let conn = self.pool.get()?;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO manual_price_overrides (item_slug, item_name, category, price, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(item_slug) DO UPDATE SET
                item_name = excluded.item_name,
                category = excluded.category,
                price = excluded.price,
                updated_at = excluded.updated_at",
            params![item_slug, item_name, category, price, now],
        )?;

        crate::log_info!("Set manual price override: {} = {} gold", item_slug, price);
        Ok(())
    }

    /// Set or clear an item as a favorite.
    pub fn set_favorite(&self, item_slug: &str, favorite: bool) -> Result<()> {
        let conn = self.pool.get()?;
        let flag = if favorite { 1 } else { 0 };

        let rows = conn.execute(
            "UPDATE market_prices SET favorite = ?1 WHERE item_slug = ?2",
            params![flag, item_slug],
        )?;

        if rows == 0 {
            conn.execute(
                "UPDATE manual_price_overrides SET favorite = ?1 WHERE item_slug = ?2",
                params![flag, item_slug],
            )?;
        }

        Ok(())
    }

    /// Remove a manual price override, reverting to API price.
    pub fn remove_manual_price(&self, item_slug: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        let rows = conn.execute(
            "DELETE FROM manual_price_overrides WHERE item_slug = ?1",
            params![item_slug],
        )?;
        Ok(rows > 0)
    }

    /// Get a market setting value.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.pool.get()?;
        let val = conn
            .query_row(
                "SELECT value FROM market_settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .ok();
        Ok(val)
    }

    /// Set a market setting value.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO market_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Check if market data needs refresh (older than 1 hour).
    pub fn needs_refresh(&self) -> Result<bool> {
        let last_refresh = self
            .get_setting("last_full_refresh")?
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);
        let now = chrono::Utc::now().timestamp();
        Ok(now - last_refresh > 3600)
    }

    /// Seed gem entries into manual_price_overrides if they don't exist yet.
    /// Gems are manual-only (no API source), so we pre-populate them with price 0.
    pub fn seed_gem_entries(&self) -> Result<usize> {
        let conn = self.pool.get()?;
        let mut count = 0;

        let gem_entries: Vec<(&str, &str)> = vec![
            ("gem-t3-damage-lv6", "T3 Damage Gem Lv. 6"),
            ("gem-t3-damage-lv7", "T3 Damage Gem Lv. 7"),
            ("gem-t3-damage-lv8", "T3 Damage Gem Lv. 8"),
            ("gem-t3-damage-lv9", "T3 Damage Gem Lv. 9"),
            ("gem-t3-damage-lv10", "T3 Damage Gem Lv. 10"),
            ("gem-t3-cooldown-lv6", "T3 Cooldown Gem Lv. 6"),
            ("gem-t3-cooldown-lv7", "T3 Cooldown Gem Lv. 7"),
            ("gem-t3-cooldown-lv8", "T3 Cooldown Gem Lv. 8"),
            ("gem-t3-cooldown-lv9", "T3 Cooldown Gem Lv. 9"),
            ("gem-t3-cooldown-lv10", "T3 Cooldown Gem Lv. 10"),
            ("gem-t4-damage-lv6", "T4 Damage Gem Lv. 6"),
            ("gem-t4-damage-lv7", "T4 Damage Gem Lv. 7"),
            ("gem-t4-damage-lv8", "T4 Damage Gem Lv. 8"),
            ("gem-t4-damage-lv9", "T4 Damage Gem Lv. 9"),
            ("gem-t4-damage-lv10", "T4 Damage Gem Lv. 10"),
            ("gem-t4-cooldown-lv6", "T4 Cooldown Gem Lv. 6"),
            ("gem-t4-cooldown-lv7", "T4 Cooldown Gem Lv. 7"),
            ("gem-t4-cooldown-lv8", "T4 Cooldown Gem Lv. 8"),
            ("gem-t4-cooldown-lv9", "T4 Cooldown Gem Lv. 9"),
            ("gem-t4-cooldown-lv10", "T4 Cooldown Gem Lv. 10"),
        ];

        let mut stmt = conn.prepare_cached(
            "INSERT OR IGNORE INTO manual_price_overrides (item_slug, item_name, category, price, favorite, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;

        let now = chrono::Utc::now().timestamp();
        for (slug, name) in &gem_entries {
            let rows = stmt.execute(params![slug, name, "gems", 0, 0, now])?;
            count += rows;
        }

        if count > 0 {
            crate::log_info!("Seeded {} new gem entries", count);
        }
        Ok(count)
    }

    /// Get all gem prices (manual-only items from manual_price_overrides).
    pub fn get_gem_prices(&self) -> Result<Vec<MarketItem>> {
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare(
            "SELECT item_slug, item_name, category, price, favorite, updated_at
             FROM manual_price_overrides
             WHERE category = 'gems'
             ORDER BY item_slug ASC",
        )?;

        let items = stmt
            .query_map([], |row| {
                Ok(MarketItem {
                    item_slug: row.get(0)?,
                    item_name: row.get(1)?,
                    category: row.get(2)?,
                    price: row.get(3)?,
                    favorite: row.get::<_, i32>(4)? == 1,
                    fetched_at: row.get(5)?,
                    is_manual_override: true,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(items)
    }
}
