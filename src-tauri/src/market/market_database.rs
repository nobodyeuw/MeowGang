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
    pub gem_tier: Option<String>,
    pub gem_kind: Option<String>,
    pub gem_level: Option<i64>,
    pub is_manual_only: bool,
    pub estimated_price: Option<i64>,
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
        self.ensure_column_exists(conn, "market_prices", "gem_tier TEXT")?;
        self.ensure_column_exists(conn, "market_prices", "gem_kind TEXT")?;
        self.ensure_column_exists(conn, "market_prices", "gem_level INTEGER")?;
        self.ensure_column_exists(conn, "market_prices", "is_manual_only INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column_exists(conn, "market_prices", "estimated_price INTEGER")?;
        self.ensure_column_exists(conn, "manual_price_overrides", "favorite INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column_exists(conn, "manual_price_overrides", "gem_tier TEXT")?;
        self.ensure_column_exists(conn, "manual_price_overrides", "gem_kind TEXT")?;
        self.ensure_column_exists(conn, "manual_price_overrides", "gem_level INTEGER")?;
        self.ensure_column_exists(
            conn,
            "manual_price_overrides",
            "is_manual_only INTEGER NOT NULL DEFAULT 0",
        )?;
        self.ensure_column_exists(conn, "manual_price_overrides", "estimated_price INTEGER")?;
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
                "SELECT item_slug, item_name, category, price, favorite, updated_at, gem_tier, gem_kind, gem_level, is_manual_only, estimated_price
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
                        gem_tier: row.get(6)?,
                        gem_kind: row.get(7)?,
                        gem_level: row.get(8)?,
                        is_manual_only: row.get::<_, i32>(9)? == 1,
                        estimated_price: row.get(10)?,
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
                "SELECT item_slug, item_name, category, price, favorite, fetched_at, gem_tier, gem_kind, gem_level, is_manual_only, estimated_price
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
                        gem_tier: row.get(6)?,
                        gem_kind: row.get(7)?,
                        gem_level: row.get(8)?,
                        is_manual_only: row.get::<_, i32>(9)? == 1,
                        estimated_price: row.get(10)?,
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
                CASE WHEN o.item_slug IS NOT NULL THEN 1 ELSE 0 END AS is_manual,
                COALESCE(o.gem_tier, m.gem_tier) AS gem_tier,
                COALESCE(o.gem_kind, m.gem_kind) AS gem_kind,
                COALESCE(o.gem_level, m.gem_level) AS gem_level,
                COALESCE(o.is_manual_only, m.is_manual_only, 0) AS is_manual_only,
                COALESCE(o.estimated_price, m.estimated_price) AS estimated_price
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
                    gem_tier: row.get(7)?,
                    gem_kind: row.get(8)?,
                    gem_level: row.get(9)?,
                    is_manual_only: row.get::<_, i32>(10)? == 1,
                    estimated_price: row.get(11)?,
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
                CASE WHEN o.item_slug IS NOT NULL THEN 1 ELSE 0 END AS is_manual,
                COALESCE(o.gem_tier, m.gem_tier) AS gem_tier,
                COALESCE(o.gem_kind, m.gem_kind) AS gem_kind,
                COALESCE(o.gem_level, m.gem_level) AS gem_level,
                COALESCE(o.is_manual_only, m.is_manual_only, 0) AS is_manual_only,
                COALESCE(o.estimated_price, m.estimated_price) AS estimated_price
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
                    gem_tier: row.get(7)?,
                    gem_kind: row.get(8)?,
                    gem_level: row.get(9)?,
                    is_manual_only: row.get::<_, i32>(10)? == 1,
                    estimated_price: row.get(11)?,
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

    /// Reset a manual-only item back to its seeded estimated price.
    pub fn reset_manual_price_to_estimate(&self, item_slug: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        let now = chrono::Utc::now().timestamp();
        let rows = conn.execute(
            "UPDATE manual_price_overrides
             SET price = estimated_price, updated_at = ?1
             WHERE item_slug = ?2
               AND is_manual_only = 1
               AND estimated_price IS NOT NULL",
            params![now, item_slug],
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

    /// Seeds manual-only gem rows if they do not exist yet.
    ///
    /// The marketplace API does not provide gem prices, so these rows are a
    /// small local allowlist for user-entered progression cost estimates.
    pub fn seed_gem_entries(&self) -> Result<usize> {
        let conn = self.pool.get()?;
        let mut count = 0;

        let gem_entries = Self::estimated_gem_entries();

        let mut stmt = conn.prepare_cached(
            "INSERT OR IGNORE INTO manual_price_overrides
                (item_slug, item_name, category, price, favorite, updated_at, gem_tier, gem_kind, gem_level, is_manual_only, estimated_price)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )?;

        let now = chrono::Utc::now().timestamp();
        for (slug, name, tier, kind, level, estimated_price) in &gem_entries {
            let rows = stmt.execute(params![
                slug,
                name,
                "gems",
                estimated_price,
                0,
                now,
                tier,
                kind,
                level,
                1,
                estimated_price
            ])?;
            count += rows;
            conn.execute(
                "UPDATE manual_price_overrides
                 SET gem_tier = ?1,
                     gem_kind = ?2,
                     gem_level = ?3,
                     is_manual_only = 1,
                     estimated_price = ?4,
                     price = CASE WHEN price = 0 THEN ?4 ELSE price END
                 WHERE item_slug = ?5 AND category = 'gems'",
                params![tier, kind, level, estimated_price, slug],
            )?;
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
            "SELECT item_slug, item_name, category, price, favorite, updated_at, gem_tier, gem_kind, gem_level, is_manual_only, estimated_price
             FROM manual_price_overrides
             WHERE category = 'gems'
             ORDER BY COALESCE(gem_tier, ''), COALESCE(gem_kind, ''), COALESCE(gem_level, 0), item_slug ASC",
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
                    gem_tier: row.get(6)?,
                    gem_kind: row.get(7)?,
                    gem_level: row.get(8)?,
                    is_manual_only: row.get::<_, i32>(9)? == 1,
                    estimated_price: row.get(10)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(items)
    }

    fn parse_gem_slug(slug: &str) -> Option<(&'static str, &'static str, i64)> {
        let parts: Vec<&str> = slug.split('-').collect();
        if parts.len() != 4 || parts[0] != "gem" {
            return None;
        }
        let tier = match parts[1] {
            "t3" => "T3",
            "t4" => "T4",
            _ => return None,
        };
        let kind = match parts[2] {
            "damage" | "attack" => "attack",
            "cooldown" => "cooldown",
            _ => return None,
        };
        let level = parts[3].strip_prefix("lv")?.parse().ok()?;
        Some((tier, kind, level))
    }

    fn estimated_gem_entries() -> Vec<(String, String, &'static str, &'static str, i64, i64)> {
        let mut entries = Vec::new();

        for tier in ["t3", "t4"] {
            let min_level = if tier == "t3" { 6 } else { 1 };
            let max_level = 10;
            for kind in ["damage", "cooldown"] {
                for level in min_level..=max_level {
                    let slug = format!("gem-{}-{}-lv{}", tier, kind, level);
                    let name = format!(
                        "{} {} Gem Lv. {}",
                        tier.to_uppercase(),
                        if kind == "damage" { "Damage" } else { "Cooldown" },
                        level
                    );
                    let (normalized_tier, normalized_kind, normalized_level) =
                        Self::parse_gem_slug(&slug).expect("generated gem slug should parse");
                    entries.push((
                        slug,
                        name,
                        normalized_tier,
                        normalized_kind,
                        normalized_level,
                        Self::estimate_gem_price(normalized_tier, normalized_kind, normalized_level),
                    ));
                }
            }
        }

        entries
    }

    fn estimate_gem_price(tier: &str, kind: &str, level: i64) -> i64 {
        // Damage and cooldown gems intentionally pass through separate kinds so future price gaps
        // (for example T4 Lv.10 damage costing more than cooldown) only need this estimator changed.
        let kind_adjustment = match kind {
            "attack" => 0,
            "cooldown" => 0,
            _ => 0,
        };

        match tier {
            // T3 Lv.6 can be converted into T4 Lv.4, then each next level is 3x the previous.
            "T3" => {
                let t4_level_10_price = 3_150_000_f64;
                let t4_level_4_price = t4_level_10_price / 3_f64.powi(6);
                (t4_level_4_price * 3_f64.powi((level - 6).max(0) as i32)).round() as i64 + kind_adjustment
            }
            "T4" => {
                let level_10_price = 3_150_000_f64;
                (level_10_price / 3_f64.powi((10 - level).max(0) as i32)).round() as i64 + kind_adjustment
            }
            _ => 0,
        }
    }

    /// Seeds manual-only accessory estimate rows from the EUC efficiency sheet.
    pub fn seed_accessory_entries(&self) -> Result<usize> {
        let conn = self.pool.get()?;
        let now = chrono::Utc::now().timestamp();
        let mut count = 0;

        let mut stmt = conn.prepare_cached(
            "INSERT OR IGNORE INTO manual_price_overrides
                (item_slug, item_name, category, price, favorite, updated_at, is_manual_only, estimated_price)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;

        for (slug, name, estimated_price) in Self::estimated_accessory_entries() {
            let rows = stmt.execute(params![
                slug,
                name,
                "accessories",
                estimated_price,
                0,
                now,
                1,
                estimated_price
            ])?;
            count += rows;
            conn.execute(
                "UPDATE manual_price_overrides
                 SET item_name = ?1,
                     category = 'accessories',
                     is_manual_only = 1,
                     estimated_price = ?2,
                     price = CASE WHEN price = 0 THEN ?2 ELSE price END
                 WHERE item_slug = ?3 AND category = 'accessories'",
                params![name, estimated_price, slug],
            )?;
        }

        if count > 0 {
            crate::log_info!("Seeded {} new accessory estimate entries", count);
        }
        Ok(count)
    }

    /// Get all manual-only accessory estimate rows.
    pub fn get_accessory_prices(&self) -> Result<Vec<MarketItem>> {
        self.get_manual_only_prices_by_category("accessories")
    }

    fn get_manual_only_prices_by_category(&self, category: &str) -> Result<Vec<MarketItem>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT item_slug, item_name, category, price, favorite, updated_at, gem_tier, gem_kind, gem_level, is_manual_only, estimated_price
             FROM manual_price_overrides
             WHERE category = ?1
             ORDER BY item_name ASC",
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
                    is_manual_override: true,
                    gem_tier: row.get(6)?,
                    gem_kind: row.get(7)?,
                    gem_level: row.get(8)?,
                    is_manual_only: row.get::<_, i32>(9)? == 1,
                    estimated_price: row.get(10)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(items)
    }

    fn estimated_accessory_entries() -> Vec<(String, String, i64)> {
        let mut entries: Vec<(String, String, i64)> = vec![
            (
                "accessory-necklace-add-low-out-none",
                "Necklace Additional Damage Low",
                1_000,
            ),
            (
                "accessory-necklace-add-low-out-low",
                "Necklace Additional Damage Low + Outgoing Damage Low",
                1_200,
            ),
            (
                "accessory-necklace-add-low-out-mid",
                "Necklace Additional Damage Low + Outgoing Damage Mid",
                2_000,
            ),
            (
                "accessory-necklace-add-low-out-high",
                "Necklace Additional Damage Low + Outgoing Damage High",
                60_000,
            ),
            (
                "accessory-necklace-add-mid-out-none",
                "Necklace Additional Damage Mid",
                1_000,
            ),
            (
                "accessory-necklace-add-mid-out-low",
                "Necklace Additional Damage Mid + Outgoing Damage Low",
                2_500,
            ),
            (
                "accessory-necklace-add-mid-out-mid",
                "Necklace Additional Damage Mid + Outgoing Damage Mid",
                45_000,
            ),
            (
                "accessory-necklace-add-mid-out-high",
                "Necklace Additional Damage Mid + Outgoing Damage High",
                520_000,
            ),
            (
                "accessory-necklace-add-high-out-none",
                "Necklace Additional Damage High",
                1_000,
            ),
            (
                "accessory-necklace-add-high-out-low",
                "Necklace Additional Damage High + Outgoing Damage Low",
                40_000,
            ),
            (
                "accessory-necklace-add-high-out-mid",
                "Necklace Additional Damage High + Outgoing Damage Mid",
                425_000,
            ),
            (
                "accessory-necklace-add-high-out-high",
                "Necklace Additional Damage High + Outgoing Damage High",
                2_400_000,
            ),
            (
                "accessory-necklace-add-none-out-low",
                "Necklace Outgoing Damage Low",
                1_000,
            ),
            (
                "accessory-necklace-add-none-out-mid",
                "Necklace Outgoing Damage Mid",
                1_000,
            ),
            (
                "accessory-necklace-add-none-out-high",
                "Necklace Outgoing Damage High",
                12_000,
            ),
            (
                "accessory-earring-atk-low-weapon-none",
                "Earring Attack Power Low",
                1_000,
            ),
            (
                "accessory-earring-atk-low-weapon-low",
                "Earring Attack Power Low + Weapon Power Low",
                1_000,
            ),
            (
                "accessory-earring-atk-low-weapon-mid",
                "Earring Attack Power Low + Weapon Power Mid",
                2_000,
            ),
            (
                "accessory-earring-atk-low-weapon-high",
                "Earring Attack Power Low + Weapon Power High",
                70_000,
            ),
            (
                "accessory-earring-atk-mid-weapon-none",
                "Earring Attack Power Mid",
                1_000,
            ),
            (
                "accessory-earring-atk-mid-weapon-low",
                "Earring Attack Power Mid + Weapon Power Low",
                2_000,
            ),
            (
                "accessory-earring-atk-mid-weapon-mid",
                "Earring Attack Power Mid + Weapon Power Mid",
                35_000,
            ),
            (
                "accessory-earring-atk-mid-weapon-high",
                "Earring Attack Power Mid + Weapon Power High",
                350_000,
            ),
            (
                "accessory-earring-atk-high-weapon-none",
                "Earring Attack Power High",
                10_000,
            ),
            (
                "accessory-earring-atk-high-weapon-low",
                "Earring Attack Power High + Weapon Power Low",
                60_000,
            ),
            (
                "accessory-earring-atk-high-weapon-mid",
                "Earring Attack Power High + Weapon Power Mid",
                450_000,
            ),
            (
                "accessory-earring-atk-high-weapon-high",
                "Earring Attack Power High + Weapon Power High",
                2_200_000,
            ),
            (
                "accessory-earring-atk-none-weapon-low",
                "Earring Weapon Power Low",
                1_000,
            ),
            (
                "accessory-earring-atk-none-weapon-mid",
                "Earring Weapon Power Mid",
                4_000,
            ),
            (
                "accessory-earring-atk-none-weapon-high",
                "Earring Weapon Power High",
                25_000,
            ),
            (
                "accessory-ring-crit-rate-low-crit-damage-none",
                "Ring Crit Rate Low",
                800,
            ),
            (
                "accessory-ring-crit-rate-low-crit-damage-low",
                "Ring Crit Rate Low + Crit Damage Low",
                1_000,
            ),
            (
                "accessory-ring-crit-rate-low-crit-damage-mid",
                "Ring Crit Rate Low + Crit Damage Mid",
                2_000,
            ),
            (
                "accessory-ring-crit-rate-low-crit-damage-high",
                "Ring Crit Rate Low + Crit Damage High",
                80_000,
            ),
            (
                "accessory-ring-crit-rate-mid-crit-damage-none",
                "Ring Crit Rate Mid",
                1_000,
            ),
            (
                "accessory-ring-crit-rate-mid-crit-damage-low",
                "Ring Crit Rate Mid + Crit Damage Low",
                1_000,
            ),
            (
                "accessory-ring-crit-rate-mid-crit-damage-mid",
                "Ring Crit Rate Mid + Crit Damage Mid",
                30_000,
            ),
            (
                "accessory-ring-crit-rate-mid-crit-damage-high",
                "Ring Crit Rate Mid + Crit Damage High",
                600_000,
            ),
            (
                "accessory-ring-crit-rate-high-crit-damage-none",
                "Ring Crit Rate High",
                10_000,
            ),
            (
                "accessory-ring-crit-rate-high-crit-damage-low",
                "Ring Crit Rate High + Crit Damage Low",
                25_000,
            ),
            (
                "accessory-ring-crit-rate-high-crit-damage-mid",
                "Ring Crit Rate High + Crit Damage Mid",
                340_000,
            ),
            (
                "accessory-ring-crit-rate-high-crit-damage-high",
                "Ring Crit Rate High + Crit Damage High",
                2_100_000,
            ),
            (
                "accessory-ring-crit-rate-none-crit-damage-low",
                "Ring Crit Damage Low",
                400,
            ),
            (
                "accessory-ring-crit-rate-none-crit-damage-mid",
                "Ring Crit Damage Mid",
                500,
            ),
            (
                "accessory-ring-crit-rate-none-crit-damage-high",
                "Ring Crit Damage High",
                35_000,
            ),
        ]
        .into_iter()
        .map(|(slug, name, price)| (slug.to_string(), name.to_string(), price))
        .map(|(slug, name, price)| {
            let estimated_price = Self::estimate_accessory_price(&slug).unwrap_or(price);
            (slug, name, estimated_price)
        })
        .collect();

        entries.extend(Self::estimated_support_accessory_entries());
        entries
    }

    fn estimate_accessory_price(slug: &str) -> Option<i64> {
        fn single_price(grade: &str, low: i64, mid: i64, high: i64) -> i64 {
            match grade {
                "low" => low,
                "mid" => mid,
                "high" => high,
                "none" => 0,
                _ => 0,
            }
        }

        fn generic_combo_price(first: &str, second: &str, high_high: i64, high_mid: i64, mid_high: i64) -> i64 {
            match (first, second) {
                ("high", "high") => high_high,
                ("high", "mid") => high_mid,
                ("mid", "high") => mid_high,
                ("mid", "mid") => ((high_mid.min(mid_high) as f64) * 0.10).round() as i64,
                ("high", "low") => ((high_mid as f64) * 0.14).round() as i64,
                ("low", "high") => ((mid_high as f64) * 0.14).round() as i64,
                ("mid", "low") | ("low", "mid") => 2_500,
                ("low", "low") => 1_200,
                ("high", "none") | ("none", "high") => 25_000,
                ("mid", "none") | ("none", "mid") => 4_000,
                ("low", "none") | ("none", "low") => 1_000,
                _ => 1_000,
            }
        }

        if let Some(rest) = slug.strip_prefix("accessory-necklace-add-") {
            let (additional, outgoing) = rest.split_once("-out-")?;
            return Some(match (additional, outgoing) {
                ("high", "high") => 2_400_000,
                ("high", "mid") => 425_000,
                ("mid", "high") => 520_000,
                _ => generic_combo_price(additional, outgoing, 2_400_000, 425_000, 520_000),
            });
        }

        if let Some(rest) = slug.strip_prefix("accessory-earring-atk-") {
            let (attack_power, weapon_power) = rest.split_once("-weapon-")?;
            return Some(match (attack_power, weapon_power) {
                ("high", "high") => 2_200_000,
                ("high", "mid") => 450_000,
                ("mid", "high") => 350_000,
                ("none", grade) => single_price(grade, 1_000, 4_000, 25_000),
                (grade, "none") => single_price(grade, 1_000, 8_000, 35_000),
                _ => generic_combo_price(attack_power, weapon_power, 2_200_000, 450_000, 350_000),
            });
        }

        if let Some(rest) = slug.strip_prefix("accessory-ring-crit-rate-") {
            let (crit_rate, crit_damage) = rest.split_once("-crit-damage-")?;
            return Some(match (crit_rate, crit_damage) {
                ("high", "high") => 2_100_000,
                ("high", "mid") => 340_000,
                ("mid", "high") => 600_000,
                _ => generic_combo_price(crit_rate, crit_damage, 2_100_000, 340_000, 600_000),
            });
        }

        if let Some(rest) = slug.strip_prefix("support-accessory-necklace-brand-") {
            let (brand, identity) = rest.split_once("-identity-")?;
            return Some(generic_combo_price(brand, identity, 2_000_000, 425_000, 520_000));
        }

        if let Some(rest) = slug.strip_prefix("support-accessory-earring-weapon-") {
            let (weapon_power, flat_weapon_power) = rest.split_once("-flat-weapon-")?;
            return Some(match (weapon_power, flat_weapon_power) {
                ("none", grade) | (grade, "none") => single_price(grade, 1_000, 4_000, 25_000),
                _ => generic_combo_price(weapon_power, flat_weapon_power, 1_100_000, 160_000, 120_000),
            });
        }

        if let Some(rest) = slug.strip_prefix("support-accessory-ring-ally-atk-") {
            let (ally_attack, ally_damage) = rest.split_once("-ally-dmg-")?;
            return Some(match (ally_attack, ally_damage) {
                ("high", "high") => 2_200_000,
                ("high", "mid") => 1_100_000,
                ("mid", "high") => 1_500_000,
                _ => generic_combo_price(ally_attack, ally_damage, 2_200_000, 1_100_000, 1_500_000),
            });
        }

        None
    }

    fn estimated_support_accessory_entries() -> Vec<(String, String, i64)> {
        fn grade_price(grade: &str) -> i64 {
            match grade {
                "low" => 1_000,
                "mid" => 12_000,
                "high" => 90_000,
                "none" => 0,
                _ => 1_000,
            }
        }

        fn combo_price(first: &str, second: &str) -> i64 {
            let first_price = grade_price(first);
            let second_price = grade_price(second);
            let premium = match (first, second) {
                ("high", "high") => 650_000,
                ("high", "mid") | ("mid", "high") => 220_000,
                ("high", "low") | ("low", "high") => 60_000,
                ("mid", "mid") => 45_000,
                ("mid", "low") | ("low", "mid") => 15_000,
                ("low", "low") => 2_500,
                ("none", "high") | ("high", "none") => 30_000,
                ("none", "mid") | ("mid", "none") => 5_000,
                ("none", "low") | ("low", "none") => 1_000,
                _ => 1_000,
            };
            first_price + second_price + premium
        }

        let grades = ["low", "mid", "high", "none"];
        let mut entries = Vec::new();

        for brand in grades {
            for identity in grades {
                if brand == "none" && identity == "none" {
                    continue;
                }
                let slug = format!("support-accessory-necklace-brand-{brand}-identity-{identity}");
                entries.push((
                    slug.clone(),
                    format!(
                        "Support Necklace Brand Power {} + Identity Gain {}",
                        brand.to_ascii_uppercase(),
                        identity.to_ascii_uppercase()
                    ),
                    Self::estimate_accessory_price(&slug).unwrap_or_else(|| combo_price(brand, identity)),
                ));
            }
        }

        for weapon in grades {
            for flat_weapon in grades {
                if weapon == "none" && flat_weapon == "none" {
                    continue;
                }
                let slug = format!("support-accessory-earring-weapon-{weapon}-flat-weapon-{flat_weapon}");
                entries.push((
                    slug.clone(),
                    format!(
                        "Support Earring Weapon Power {} + Flat Weapon Power {}",
                        weapon.to_ascii_uppercase(),
                        flat_weapon.to_ascii_uppercase()
                    ),
                    Self::estimate_accessory_price(&slug).unwrap_or_else(|| combo_price(weapon, flat_weapon)),
                ));
            }
        }

        for ally_atk in grades {
            for ally_dmg in grades {
                if ally_atk == "none" && ally_dmg == "none" {
                    continue;
                }
                let slug = format!("support-accessory-ring-ally-atk-{ally_atk}-ally-dmg-{ally_dmg}");
                entries.push((
                    slug.clone(),
                    format!(
                        "Support Ring Ally Attack Power {} + Ally Damage {}",
                        ally_atk.to_ascii_uppercase(),
                        ally_dmg.to_ascii_uppercase()
                    ),
                    Self::estimate_accessory_price(&slug).unwrap_or_else(|| combo_price(ally_atk, ally_dmg)),
                ));
            }
        }

        entries
    }
}
