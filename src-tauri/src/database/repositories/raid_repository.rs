use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Transaction};
use crate::models::*;
use crate::handlers::raid_handlers::{RaidConfig, RaidGateConfig};

pub struct RaidRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl RaidRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        Self { pool }
    }

    pub fn get_character_raid_config(&self, character_id: i64) -> Result<Vec<crate::models::CharacterRaidState>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT raid_id, tracked FROM raid_config WHERE char_id = ?1"
        )?;
        
        let raid_iter = stmt.query_map([character_id], |row| {
            Ok(crate::models::CharacterRaidState {
                char_id: character_id,
                content_id: row.get::<_, String>(0)?,
                tracked: row.get::<_, i64>(1)? == 1,
                current_value: None, // Raids don't have rested values
            })
        })?;
        
        let mut raids = Vec::new();
        for raid in raid_iter {
            raids.push(raid?);
        }
        
        Ok(raids)
    }

    pub fn get_character_raid_configs(&self, character_id: i64) -> Result<Vec<RaidConfig>> {
        let conn = self.pool.get()?;
        
        // Get all raid configurations for this character, grouped by content_id
        let mut stmt = conn.prepare(
            "SELECT content_id, gate, difficulty, take_gold, buy_box 
             FROM conf_raid WHERE char_id = ?1 ORDER BY content_id, gate"
        )?;
        
        let raid_gate_iter = stmt.query_map([character_id], |row| {
            Ok((
                row.get::<_, String>(0)?, // content_id
                row.get::<_, String>(1)?, // gate
                row.get::<_, String>(2)?, // difficulty
                row.get::<_, i64>(3)? == 1, // take_gold
                row.get::<_, i64>(4)? == 1, // buy_box
            ))
        })?;
        
        // Group gates by content_id
        let mut raid_configs: std::collections::HashMap<String, Vec<RaidGateConfig>> = std::collections::HashMap::new();
        for result in raid_gate_iter {
            let (content_id, gate, difficulty, take_gold, buy_box) = result?;
            
            let gate_config = RaidGateConfig {
                gate,
                difficulty,
                take_gold,
                buy_box,
            };
            
            raid_configs.entry(content_id).or_insert_with(Vec::new).push(gate_config);
        }
        
        // Convert to RaidConfig structures
        let mut configs = Vec::new();
        for (content_id, gates) in raid_configs {
            // Check if any gate has take_gold or buy_box enabled
            let take_gold = gates.iter().any(|g| g.take_gold);
            let buy_box = gates.iter().any(|g| g.buy_box);
            
            configs.push(RaidConfig {
                content_id,
                gates,
                take_gold,
                buy_box,
            });
        }
        
        Ok(configs)
    }

    pub fn save_character_raid_configs(&self, character_id: i64, configs: &[RaidConfig]) -> Result<()> {
        let mut conn = self.pool.get()?;
        
        // Start transaction
        let tx = conn.transaction()?;
        
        // Clear existing configurations for this character
        tx.execute("DELETE FROM conf_raid WHERE char_id = ?1", [character_id])?;
        
        // Insert new configurations with roster_id
        for config in configs {
            for gate_config in &config.gates {
                tx.execute(
                    "INSERT INTO conf_raid (roster_id, char_id, content_id, gate, difficulty, take_gold, buy_box) 
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        "default", // TODO: Get actual roster_id from context
                        character_id,
                        config.content_id,
                        gate_config.gate,
                        gate_config.difficulty,
                        if gate_config.take_gold { 1 } else { 0 },
                        if gate_config.buy_box { 1 } else { 0 }
                    ]
                )?;
            }
        }
        
        // Commit transaction
        tx.commit()?;
        
        Ok(())
    }

    pub fn save_character_raid_configs_with_roster_id(&self, roster_id: String, character_id: i64, configs: &[RaidConfig]) -> Result<()> {
        let mut conn = self.pool.get()?;
        
        // Start transaction
        let tx = conn.transaction()?;
        
        // Clear existing configurations for this character and roster
        tx.execute("DELETE FROM conf_raid WHERE roster_id = ?1 AND char_id = ?2", [&roster_id as &str, &character_id.to_string() as &str])?;
        
        // Insert new configurations
        for config in configs {
            for gate_config in &config.gates {
                tx.execute(
                    "INSERT INTO conf_raid (roster_id, char_id, content_id, gate, difficulty, take_gold, buy_box) 
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        roster_id,
                        character_id,
                        config.content_id,
                        gate_config.gate,
                        gate_config.difficulty,
                        if gate_config.take_gold { 1 } else { 0 },
                        if gate_config.buy_box { 1 } else { 0 }
                    ]
                )?;
            }
        }
        
        // Commit transaction
        tx.commit()?;
        
        Ok(())
    }

    pub fn update_raid_config(&self, character_id: i64, raid_id: &str, tracked: bool) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO raid_config (char_id, raid_id, tracked) VALUES (?1, ?2, ?3)",
            params![character_id, raid_id, if tracked { 1 } else { 0 }],
        )?;
        Ok(())
    }

    pub fn get_raid_gate_matrix(&self, character_id: i64, raid_ids: Vec<String>) -> Result<Vec<crate::models::RaidMatrixItem>> {
        let conn = self.pool.get()?;
        let mut matrix_items = Vec::new();
        
        for raid_id in raid_ids {
            let mut stmt = conn.prepare(
                "SELECT rg.gate, rg.tracked, rg.cleared, rg.clear_time, rg.take_gold, rg.buy_box
                 FROM raid_gates rg 
                 WHERE rg.char_id = ?1 AND rg.raid_id = ?2
                 ORDER BY rg.gate"
            )?;
            
            let gate_iter = stmt.query_map([character_id, raid_id.parse::<i64>().unwrap_or(0)], |row| {
                Ok(crate::models::CharacterRaidState {
                    char_id: character_id,
                    content_id: raid_id.clone(),
                    tracked: row.get::<_, i64>(1)? == 1,
                    current_value: None, // Raids don't have rested values
                })
            })?;
            
            let mut character_states = Vec::new();
            for gate in gate_iter {
                character_states.push(gate?);
            }
            
            matrix_items.push(crate::models::RaidMatrixItem {
                raid_id: raid_id.clone(),
                raid_name: raid_id.clone(), // This should come from raid data
                min_ilvl: 0, // This should come from raid data
                character_states,
            });
        }
        
        Ok(matrix_items)
    }

    pub fn get_raid_settings(&self, character_id: i64) -> Result<Vec<crate::models::RaidSettingsEntry>> {
        let conn = self.pool.get()?;
        
        // Get all raid configurations for this character
        let mut stmt = conn.prepare(
            "SELECT content_id, take_gold, buy_box FROM conf_raid WHERE char_id = ?1"
        )?;
        
        let raid_entries_iter = stmt.query_map([character_id], |row| {
            let content_id: String = row.get(0)?;
            let take_gold: i64 = row.get(1)?;
            let buy_box: i64 = row.get(2)?;
            
            // Parse content_id to extract raid info (format: "raid_name:gate:difficulty")
            let parts: Vec<&str> = content_id.split(':').collect();
            let (raid_name, gate, difficulty): (&str, &str, &str) = match parts.as_slice() {
                [name, gate, diff] => (*name, *gate, *diff),
                [name, gate] => (*name, *gate, "Normal"),
                [name] => (*name, "1", "Normal"),
                _ => (content_id.as_str(), "1", "Normal"),
            };
            
            Ok(crate::models::RaidSettingsEntry {
                raid_id: content_id.clone(),
                raid_name: raid_name.to_string(),
                difficulty: difficulty.to_string(),
                take_gold: take_gold == 1,
                buy_box: buy_box == 1,
                gate_count: gate.parse::<i64>().unwrap_or(1),
                completion_status: 0, // TODO: Get from completion_status table
                max_difficulty: "Normal".to_string(), // TODO: Get from raid data
            })
        })?;
        
        let mut entries = Vec::new();
        for entry in raid_entries_iter {
            entries.push(entry?);
        }
        
        Ok(entries)
    }

    pub fn get_raid_completion_status(&self, character_id: i64, raid_id: &str) -> Result<Vec<RaidGateCompletion>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT content_id, is_completed, timestamp, session_id 
             FROM completion_status 
             WHERE char_id = ?1 AND content_id LIKE ?2 
             ORDER BY content_id"
        )?;
        
        let completion_iter = stmt.query_map([character_id, raid_id.parse::<i64>().unwrap_or(0)], |row| {
            Ok(RaidGateCompletion {
                gate: row.get::<_, String>(0)?
                    .replace(&format!("{}_gate_", raid_id), ""),
                completed: row.get::<_, i64>(1)? == 1,
                completion_time: row.get::<_, Option<i64>>(2)?,
                session_id: row.get::<_, Option<String>>(3)?,
            })
        })?;
        
        let mut completions = Vec::new();
        for completion in completion_iter {
            completions.push(completion?);
        }
        
        Ok(completions)
    }
}
