use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, params_from_iter};
use crate::roster::Character;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestedValue {
    pub char_id: i64,
    pub content_id: String,
    pub current_value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionStatus {
    pub char_id: i64,
    pub content_id: String,
    pub is_completed: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRaidConfig {
    pub char_id: i64,
    pub content_id: String,
    pub take_gold: i64,
    pub difficulty: String,
    pub buy_box: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingStatus {
    pub char_id: i64,
    pub content_id: String,
    pub is_tracked: i64,
}

pub struct CharacterRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl CharacterRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        Self { pool }
    }

    pub fn get_characters_by_roster(&self, roster_id: &str) -> Result<Vec<Character>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT char_id, char_name, roster_id, roster_name, class_id, item_level, 
                    combat_power, display_order, earns_gold, hide_from_dashboard
             FROM conf_character 
             WHERE roster_id = ?1
             ORDER BY display_order"
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
                display_order: row.get::<_, String>(7)?.parse().unwrap_or(0),
                earns_gold: row.get(8)?,
                hide_from_dashboard: row.get(9)?,
                class_display_name: None, // Not available in conf_character table
            })
        })?;
        
        let mut characters = Vec::new();
        for character in character_iter {
            characters.push(character?);
        }
        
        Ok(characters)
    }

    pub fn get_character_by_id(&self, character_id: i64) -> Result<Option<Character>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT char_id, char_name, roster_id, roster_name, class_id, item_level, 
                    combat_power, display_order, earns_gold, hide_from_dashboard, class_display_name
             FROM conf_character 
             WHERE char_id = ?1"
        )?;
        
        let character_iter = stmt.query_map([character_id], |row| {
            Ok(Character {
                char_id: row.get(0)?,
                char_name: row.get(1)?,
                roster_id: row.get(2)?,
                roster_name: row.get(3)?,
                class_id: row.get(4)?,
                item_level: row.get(5)?,
                combat_power: row.get(6)?,
                display_order: row.get::<_, String>(7)?.parse().unwrap_or(0),
                earns_gold: row.get(8)?,
                hide_from_dashboard: row.get(9)?,
                class_display_name: None, // Not available in conf_character table
            })
        })?;
        
        for character in character_iter {
            return Ok(Some(character?));
        }
        Ok(None)
    }

    pub fn get_dashboard_characters(&self) -> Result<Vec<crate::models::DashboardCharacter>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT c.char_id, c.char_name, c.class_id, c.class_display_name, 
                    c.item_level, c.combat_power, c.roster_name, 
                    c.display_order, c.earns_gold, c.hide_from_dashboard
             FROM conf_character c
             ORDER BY c.display_order, c.char_name"
        )?;
        
        let character_iter = stmt.query_map([], |row| {
            Ok(crate::models::DashboardCharacter {
                char_id: row.get(0)?,
                char_name: row.get(1)?,
                class_id: row.get(2)?,
                class_display_name: row.get(3)?,
                item_level: row.get(4)?,
                combat_power: row.get(5)?,
                roster_name: row.get(6)?,
                last_active: None, // Not in conf_character table
                earns_gold: row.get(8)?,
                display_order: row.get(7)?,
            })
        })?;
        
        let mut characters = Vec::new();
        for character in character_iter {
            characters.push(character?);
        }
        Ok(characters)
    }

    pub fn update_character_settings(&self, character_id: i64, settings: &crate::models::CharacterSettings) -> Result<()> {
        let conn = self.pool.get()?;

        let mut set_clauses = Vec::new();
        let mut params: Vec<rusqlite::types::Value> = Vec::new();

        if let Some(earns_gold) = settings.earns_gold {
            set_clauses.push("earns_gold = ?".to_string());
            params.push(earns_gold.into());
        }

        if let Some(hide_from_dashboard) = settings.hide_from_dashboard {
            set_clauses.push("hide_from_dashboard = ?".to_string());
            params.push(hide_from_dashboard.into());
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        let sql = format!(
            "UPDATE conf_character SET {} WHERE char_id = ?",
            set_clauses.join(", ")
        );

        params.push(character_id.into());
        conn.execute(&sql, params_from_iter(params.iter()))?;
        Ok(())
    }

    pub fn update_character_earns_gold(&self, character_id: i64, earns_gold: bool) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE conf_character SET earns_gold = ?1 WHERE char_id = ?2",
            params![earns_gold, character_id],
        )?;
        Ok(())
    }

    pub fn get_character_matrix_info(&self, roster_id: &str) -> Result<Vec<crate::models::CharacterMatrixInfo>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT char_id, char_name, item_level, combat_power, class_id
             FROM conf_character 
             WHERE roster_id = ?1 
             ORDER BY display_order, char_name"
        )?;
        
        let character_iter = stmt.query_map([roster_id], |row| {
            Ok(crate::models::CharacterMatrixInfo {
                char_id: row.get(0)?,
                char_name: row.get(1)?,
                item_level: row.get(2)?,
                combat_power: row.get(3)?,
                class_id: row.get(4)?,
                display_order: row.get(5)?,
            })
        })?;
        
        let mut characters = Vec::new();
        for character in character_iter {
            characters.push(character?);
        }
        Ok(characters)
    }

    pub fn save_character_from_scraper(&self, character: &Character, roster_id: &str) -> Result<i64> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO conf_character 
             (char_id, char_name, roster_id, roster_name, class_id, item_level, 
              combat_power, display_order, earns_gold, hide_from_dashboard, class_display_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(char_id) DO UPDATE SET
               char_name = excluded.char_name,
               roster_id = excluded.roster_id,
               roster_name = excluded.roster_name,
               class_id = excluded.class_id,
               item_level = excluded.item_level,
               combat_power = excluded.combat_power,
               display_order = excluded.display_order,
               earns_gold = excluded.earns_gold,
               class_display_name = excluded.class_display_name",
            params![
                character.char_id,
                character.char_name,
                roster_id,
                roster_id,
                character.class_id,
                character.item_level,
                character.combat_power,
                character.display_order,
                character.earns_gold,
                false,
                character.class_display_name
            ],
        )?;
        Ok(character.char_id)
    }

    pub fn update_character_order(&self, character_id: i64, new_order: &str) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE conf_character SET display_order = ?1 WHERE char_id = ?2",
            params![new_order, character_id],
        )?;
        Ok(())
    }

    pub fn get_character_rested_values(&self, character_id: i64) -> Result<Vec<RestedValue>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT char_id, content_id, current_value 
             FROM rested_values 
             WHERE char_id = ?1"
        )?;
        
        let rested_iter = stmt.query_map([character_id], |row| {
            Ok(RestedValue {
                char_id: row.get(0)?,
                content_id: row.get(1)?,
                current_value: row.get(2)?,
            })
        })?;
        
        let mut rested_values = Vec::new();
        for rested in rested_iter {
            rested_values.push(rested?);
        }
        
        Ok(rested_values)
    }

    pub fn get_character_completion_status(&self, character_id: i64) -> Result<Vec<CompletionStatus>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT char_id, content_id, is_completed 
             FROM completion_status 
             WHERE char_id = ?1"
        )?;
        
        let completion_iter = stmt.query_map([character_id], |row| {
            Ok(CompletionStatus {
                char_id: row.get(0)?,
                content_id: row.get(1)?,
                is_completed: row.get(2)?,
            })
        })?;
        
        let mut completion_status = Vec::new();
        for completion in completion_iter {
            completion_status.push(completion?);
        }
        
        Ok(completion_status)
    }

    pub fn get_character_raid_configs(&self, character_id: i64) -> Result<Vec<CharacterRaidConfig>> {
        let mut conn = self.pool.get()?;
        
        let mut stmt = conn.prepare(
            "SELECT char_id, content_id, take_gold, difficulty, buy_box
             FROM conf_raid 
             WHERE char_id = ?1"
        )?;
        
        let raid_iter = stmt.query_map([character_id], |row| {
            Ok(CharacterRaidConfig {
                char_id: row.get(0)?,
                content_id: row.get(1)?,
                take_gold: row.get(2)?,
                difficulty: row.get(3)?,
                buy_box: row.get(4)?,
            })
        })?;
        
        let mut raid_configs = Vec::new();
        for raid in raid_iter {
            raid_configs.push(raid?);
        }
        
        Ok(raid_configs)
    }

    pub fn get_character_tracking_status(&self, character_id: i64) -> Result<Vec<TrackingStatus>> {
        let mut conn = self.pool.get()?;
        
        let mut stmt = conn.prepare(
            "SELECT char_id, content_id, is_tracked 
             FROM conf_tracking 
             WHERE char_id = ?1"
        )?;
        
        let tracking_iter = stmt.query_map([character_id], |row| {
            Ok(TrackingStatus {
                char_id: row.get(0)?,
                content_id: row.get(1)?,
                is_tracked: row.get(2)?,
            })
        })?;
        
        let mut tracking_status = Vec::new();
        for tracking in tracking_iter {
            tracking_status.push(tracking?);
        }
        
        Ok(tracking_status)
    }

    pub fn delete_character(&self, character_id: i64) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        
        // Delete from related tables first
        tx.execute("DELETE FROM conf_todo WHERE char_id = ?1", params![character_id])?;
        tx.execute("DELETE FROM conf_raid WHERE char_id = ?1", params![character_id])?;
        tx.execute("DELETE FROM completion_status WHERE char_id = ?1", params![character_id])?;
        tx.execute("DELETE FROM rested_values WHERE char_id = ?1", params![character_id])?;
        tx.execute("DELETE FROM gold_logs WHERE char_id = ?1", params![character_id])?;
        
        // Delete character
        tx.execute("DELETE FROM conf_character WHERE char_id = ?1", params![character_id])?;
        
        tx.commit()?;
        Ok(())
    }

    /// Load rested values for all given characters in a single query.
    pub fn get_batch_rested_values(&self, char_ids: &[i64]) -> Result<HashMap<i64, Vec<RestedValue>>> {
        if char_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let conn = self.pool.get()?;
        let placeholders: String = char_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT char_id, content_id, current_value FROM rested_values WHERE char_id IN ({})",
            placeholders
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(char_ids.iter()), |row| {
            Ok(RestedValue {
                char_id: row.get(0)?,
                content_id: row.get(1)?,
                current_value: row.get(2)?,
            })
        })?;
        let mut map: HashMap<i64, Vec<RestedValue>> = HashMap::new();
        for row in rows {
            let val = row?;
            map.entry(val.char_id).or_default().push(val);
        }
        Ok(map)
    }

    /// Load completion status for all given characters in a single query.
    pub fn get_batch_completion_status(&self, char_ids: &[i64]) -> Result<HashMap<i64, Vec<CompletionStatus>>> {
        if char_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let conn = self.pool.get()?;
        let placeholders: String = char_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT char_id, content_id, is_completed FROM completion_status WHERE char_id IN ({})",
            placeholders
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(char_ids.iter()), |row| {
            Ok(CompletionStatus {
                char_id: row.get(0)?,
                content_id: row.get(1)?,
                is_completed: row.get(2)?,
            })
        })?;
        let mut map: HashMap<i64, Vec<CompletionStatus>> = HashMap::new();
        for row in rows {
            let val = row?;
            map.entry(val.char_id).or_default().push(val);
        }
        Ok(map)
    }

    /// Load tracking status for all given characters in a single query.
    pub fn get_batch_tracking_status(&self, char_ids: &[i64]) -> Result<HashMap<i64, Vec<TrackingStatus>>> {
        if char_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let conn = self.pool.get()?;
        let placeholders: String = char_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT char_id, content_id, is_tracked FROM conf_tracking WHERE char_id IN ({})",
            placeholders
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(char_ids.iter()), |row| {
            Ok(TrackingStatus {
                char_id: row.get(0)?,
                content_id: row.get(1)?,
                is_tracked: row.get(2)?,
            })
        })?;
        let mut map: HashMap<i64, Vec<TrackingStatus>> = HashMap::new();
        for row in rows {
            let val = row?;
            map.entry(val.char_id).or_default().push(val);
        }
        Ok(map)
    }

    /// Load raid configs for all given characters in a single query.
    pub fn get_batch_raid_configs(&self, char_ids: &[i64]) -> Result<HashMap<i64, Vec<CharacterRaidConfig>>> {
        if char_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let conn = self.pool.get()?;
        let placeholders: String = char_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT char_id, content_id, take_gold, difficulty, buy_box FROM conf_raid WHERE char_id IN ({})",
            placeholders
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(char_ids.iter()), |row| {
            Ok(CharacterRaidConfig {
                char_id: row.get(0)?,
                content_id: row.get(1)?,
                take_gold: row.get(2)?,
                difficulty: row.get(3)?,
                buy_box: row.get(4)?,
            })
        })?;
        let mut map: HashMap<i64, Vec<CharacterRaidConfig>> = HashMap::new();
        for row in rows {
            let val = row?;
            map.entry(val.char_id).or_default().push(val);
        }
        Ok(map)
    }
}
