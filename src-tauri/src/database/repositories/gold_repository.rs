use anyhow::{Result, Context};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use serde::Serialize; // WICHTIG für Tauri

pub struct GoldRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl GoldRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        Self { pool }
    }
    
    /// Get database connection
    pub fn get_connection(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>> {
        self.pool.get()
            .context("Failed to get database connection from pool")
    }
    
    /// Get weekly gold summary for a character
    pub fn get_weekly_gold_summary(&self, char_id: i64, week_start: i64) -> Result<WeeklyGoldSummary> {
        let conn = self.get_connection()?;
        
        // Get raid gold (excluding box purchases)
        let mut stmt = conn.prepare(
            "SELECT 
                SUM(gold_tradable) as tradable,
                SUM(gold_bound) as bound,
                SUM(gold_value_total) as total,
                COUNT(*) as entries
             FROM gold_logs 
             WHERE char_id = ?1 AND timestamp >= ?2 AND source = 'raid'"
        )?;
        
        let raid_summary = stmt.query_row(params![char_id, week_start], |row: &rusqlite::Row| {
            Ok((
                row.get::<_, Option<i64>>(0)?.unwrap_or(0), // tradable
                row.get::<_, Option<i64>>(1)?.unwrap_or(0), // bound
                row.get::<_, Option<i64>>(2)?.unwrap_or(0), // total
                row.get::<_, Option<i64>>(3)?.unwrap_or(0), // entries
            ))
        })?;
        
        // Get box purchase costs
        let mut stmt = conn.prepare(
            "SELECT 
                SUM(ABS(gold_value_total)) as total,
                COUNT(*) as entries
             FROM gold_logs 
             WHERE char_id = ?1 AND timestamp >= ?2 AND source = 'box_purchase'"
        )?;
        
        let box_summary = stmt.query_row(params![char_id, week_start], |row: &rusqlite::Row| {
            Ok((
                row.get::<_, Option<i64>>(0)?.unwrap_or(0), // total cost
                row.get::<_, Option<i64>>(1)?.unwrap_or(0), // purchase count
            ))
        })?;
        
        // Get extra income (all sources except raid and box_purchase)
        let mut stmt = conn.prepare(
            "SELECT 
                SUM(gold_value_total) as total
             FROM gold_logs 
             WHERE char_id = ?1 AND timestamp >= ?2 AND source NOT IN ('raid', 'box_purchase')"
        )?;
        
        let extra_total = stmt.query_row(params![char_id, week_start], |row: &rusqlite::Row| {
            Ok(row.get::<_, Option<i64>>(0)?.unwrap_or(0))
        })?;
        
        Ok(WeeklyGoldSummary {
            tradable_gold: raid_summary.0,
            bound_gold: raid_summary.1,
            total_gold: raid_summary.2,
            total_entries: raid_summary.3,
            extra_income_gold: extra_total,
            box_purchase_cost: box_summary.0,
        })
    }

    pub fn log_raid_gold_completion(
        &self,
        char_id: i64,
        timestamp: i64,
        gold_bound: i64,
        gold_tradable: i64,
        notes: &str,
    ) -> Result<()> {
        let conn = self.get_connection()?;
        let gold_value_total = gold_bound + gold_tradable;
        
        conn.execute(
            "INSERT INTO gold_logs (timestamp, char_id, source, gold_value_total, gold_bound, gold_tradable, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![timestamp, char_id, "raid", gold_value_total, gold_bound, gold_tradable, notes]
        ).context("could not insert gold log entry")?;

        Ok(())
    }


    
    pub fn get_gold_by_source(&self, char_id: i64, source: &str, limit: i32) -> Result<Vec<GoldLogEntry>> {
        let conn = self.get_connection()?;
        
        let mut stmt = conn.prepare(
            "SELECT timestamp, source, gold_value_total, gold_bound, gold_tradable, notes
             FROM gold_logs 
             WHERE char_id = ?1 AND source LIKE ?2 ESCAPE '\\'
             ORDER BY timestamp DESC
             LIMIT ?3"
        )?;

        let escaped_source = source
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let entries = stmt.query_map(params![char_id, format!("%{}%", escaped_source), limit], |row| {
            Ok(GoldLogEntry {
                timestamp: row.get(0)?,
                source: row.get(1)?,
                gold_value_total: row.get(2)?,
                gold_bound: row.get(3)?,
                gold_tradable: row.get(4)?,
                notes: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Get aggregated weekly gold stats for all gold-earning characters.
    ///
    /// Uses a single query with parameterized IN clause instead of dynamic SQL
    /// string formatting, avoiding potential injection vectors.
    pub fn get_weekly_gold_stats_all(&self, week_start: i64) -> Result<WeeklyGoldSummary> {
        let conn = self.get_connection()?;

        let character_ids: Vec<i64> = conn
            .prepare("SELECT char_id FROM conf_character WHERE earns_gold = 1")?
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<i64>, _>>()?;

        if character_ids.is_empty() {
            return Ok(WeeklyGoldSummary {
                tradable_gold: 0,
                bound_gold: 0,
                total_gold: 0,
                total_entries: 0,
                extra_income_gold: 0,
                box_purchase_cost: 0,
            });
        }

        let placeholders: String = character_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT COALESCE(SUM(gold_tradable), 0),
                    COALESCE(SUM(gold_bound), 0),
                    COALESCE(SUM(gold_value_total), 0),
                    COUNT(*)
             FROM gold_logs
             WHERE char_id IN ({}) AND timestamp >= ?",
            placeholders
        );

        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = character_ids
            .iter()
            .map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        params.push(Box::new(week_start));

        let mut stmt = conn.prepare(&sql)?;
        let summary = stmt.query_row(rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
            Ok(WeeklyGoldSummary {
                tradable_gold: row.get(0)?,
                bound_gold: row.get(1)?,
                total_gold: row.get(2)?,
                total_entries: row.get(3)?,
                extra_income_gold: 0,
                box_purchase_cost: 0,
            })
        })?;

        Ok(summary)
    }

    /// Delete gold logs older than the weekly reset timestamp
    pub fn delete_old_gold_logs(&self, weekly_reset_timestamp: i64) -> Result<usize> {
        let conn = self.get_connection()?;
        
        let rows_affected = conn.execute(
            "DELETE FROM gold_logs WHERE timestamp < ?1",
            params![weekly_reset_timestamp]
        ).context("could not delete old gold logs")?;
        
        Ok(rows_affected)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GoldLogEntry {
    pub timestamp: i64,
    pub source: Option<String>,
    pub gold_value_total: i64,
    pub gold_bound: i64,
    pub gold_tradable: i64,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeeklyGoldSummary {
    pub tradable_gold: i64,
    pub bound_gold: i64,
    pub total_gold: i64,
    pub total_entries: i64,
    pub extra_income_gold: i64,
    pub box_purchase_cost: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoxPurchaseSummary {
    pub total_cost: i64,
    pub purchase_count: i64,
}