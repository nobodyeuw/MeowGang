use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use crate::roster::{Character, ScraperData};

pub struct RosterRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl RosterRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        Self { pool }
    }

    pub fn get_all_rosters(&self) -> Result<Vec<crate::models::Roster>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT DISTINCT roster_id as id, roster_name as roster_name, NULL as last_updated
             FROM conf_character 
             GROUP BY roster_id, roster_name
             ORDER BY roster_name"
        )?;
        
        let roster_iter = stmt.query_map([], |row| {
            Ok(crate::models::Roster {
                id: row.get(0)?,
                roster_name: row.get(1)?,
                last_updated: row.get(2)?,
            })
        })?;
        
        let mut rosters = Vec::new();
        for roster in roster_iter {
            rosters.push(roster?);
        }
        Ok(rosters)
    }

    pub fn get_characters_by_roster(&self, roster_id: &str) -> Result<Vec<Character>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT char_id, char_name, roster_id, roster_name, class_id, item_level, 
                    combat_power, CAST(display_order AS INTEGER), earns_gold, hide_from_dashboard
             FROM conf_character 
             WHERE roster_id = ?1 
             ORDER BY CAST(display_order AS INTEGER), char_name"
        )?;
        
        let character_iter = stmt.query_map([roster_id], |row| {
            Ok(Character {
                char_id: row.get(0)?,
                char_name: row.get(1)?,
                roster_id: row.get(2)?,
                roster_name: row.get(3)?,
                class_id: row.get(4)?,
                item_level: row.get(5)?,
                combat_power: row.get(6)?,
                display_order: row.get(7)?,
                earns_gold: row.get(8)?,
                hide_from_dashboard: row.get(9)?,
                class_display_name: None,
            })
        })?;
        
        let mut characters = Vec::new();
        for character in character_iter {
            characters.push(character?);
        }
        Ok(characters)
    }

    pub fn save_roster_from_scraper(&self, scraper_data: &ScraperData) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        
        // Insert characters from scraper data
        for character in &scraper_data.characters {
            tx.execute(
                "INSERT INTO conf_character 
                 (char_id, char_name, roster_id, roster_name, class_id, item_level, 
                  combat_power, display_order, earns_gold, hide_from_dashboard)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(char_id) DO UPDATE SET
                   char_name = excluded.char_name,
                   roster_id = excluded.roster_id,
                   roster_name = excluded.roster_name,
                   class_id = excluded.class_id,
                   item_level = excluded.item_level,
                   combat_power = excluded.combat_power,
                   display_order = excluded.display_order,
                   earns_gold = excluded.earns_gold",
                params![
                    character.char_id,
                    character.char_name,
                    scraper_data.roster_name,
                    scraper_data.roster_name,
                    character.class_id,
                    character.item_level,
                    character.combat_power,
                    character.display_order,
                    character.earns_gold,
                    false
                ],
            )?;
        }
        
        tx.commit()?;
        Ok(())
    }

    pub fn should_update_roster(&self, roster_name: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT MAX(created_timestamp) FROM conf_character WHERE roster_id = ?1"
        )?;
        
        let last_updated = stmt.query_row([roster_name], |row| {
            Ok::<Option<i64>, rusqlite::Error>(row.get(0)?)
        })?;
        
        match last_updated {
            Some(timestamp) => {
                if timestamp > 0 {
                    let now = chrono::Utc::now().timestamp_millis();
                    let hours_diff = (now - timestamp) / 3600;
                    Ok(hours_diff >= 24)
                } else {
                    Ok(true)
                }
            }
            None => Ok(true), // No roster exists, should create
        }
    }

    pub fn update_character_order(&self, character_id: i64, new_order: &str) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE conf_character SET display_order = ?1 WHERE char_id = ?2",
            params![new_order, character_id],
        )?;
        Ok(())
    }

    pub fn update_character_roster_name(&self, character_id: i64, new_roster_name: &str) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE conf_character SET roster_name = ?1 WHERE char_id = ?2",
            params![new_roster_name, character_id],
        )?;
        Ok(())
    }

    pub fn delete_roster(&self, roster_id: &str) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        
        // Delete from related tables first
        tx.execute("DELETE FROM conf_tracking WHERE char_id IN (SELECT char_id FROM conf_character WHERE roster_id = ?1)", params![roster_id])?;
        tx.execute("DELETE FROM conf_raid WHERE char_id IN (SELECT char_id FROM conf_character WHERE roster_id = ?1)", params![roster_id])?;
        tx.execute("DELETE FROM completion_status WHERE char_id IN (SELECT char_id FROM conf_character WHERE roster_id = ?1)", params![roster_id])?;
        tx.execute("DELETE FROM rested_values WHERE char_id IN (SELECT char_id FROM conf_character WHERE roster_id = ?1)", params![roster_id])?;
        tx.execute("DELETE FROM gold_logs WHERE char_id IN (SELECT char_id FROM conf_character WHERE roster_id = ?1)", params![roster_id])?;
        
        // Delete characters
        tx.execute("DELETE FROM conf_character WHERE roster_id = ?1", params![roster_id])?;
        
        tx.commit()?;
        Ok(())
    }

    pub fn get_roster_resources(&self, roster_id: &str) -> Result<Option<crate::models::RosterResources>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT bound_gold_current, trade_gold_current, total_gold_current, timestamp
             FROM roster_resources 
             WHERE roster_id = ?1 
             ORDER BY timestamp DESC 
             LIMIT 1"
        )?;
        
        let resource_iter = stmt.query_map([roster_id], |row| {
            Ok(crate::models::RosterResources {
                bound_gold: row.get(0)?,
                trade_gold: row.get(1)?,
                total_gold: row.get(2)?,
                timestamp: row.get(3)?,
            })
        })?;
        
        for resource in resource_iter {
            return Ok(Some(resource?));
        }
        Ok(None)
    }

    pub fn update_roster_resources(&self, roster_id: &str, resources: &crate::models::RosterResources) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO roster_resources 
             (roster_id, bound_gold_current, trade_gold_current, total_gold_current, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                roster_id,
                resources.bound_gold,
                resources.trade_gold,
                resources.total_gold,
                chrono::Utc::now().timestamp_millis()
            ],
        )?;
        Ok(())
    }
}
