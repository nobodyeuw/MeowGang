use crate::handlers::raid_handlers::{RaidConfig, RaidGateConfig};
use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

pub struct RaidRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl RaidRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        Self { pool }
    }

    /// Loads raid tracking flags for a character from the current `conf_tracking` table.
    ///
    /// This command is legacy-facing; Settings > Raids now mostly uses
    /// `get_character_raid_configs`, but the store still exposes a thin wrapper.
    pub fn get_character_raid_config(&self, character_id: i64) -> Result<Vec<crate::models::CharacterRaidState>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT DISTINCT ct.content_id, ct.is_tracked
             FROM conf_tracking ct
             INNER JOIN conf_raid cr
               ON cr.char_id = ct.char_id
              AND cr.content_id = ct.content_id
             WHERE ct.char_id = ?1
             ORDER BY ct.content_id",
        )?;

        let raid_iter = stmt.query_map([character_id], |row| {
            Ok(crate::models::CharacterRaidState {
                char_id: character_id,
                content_id: row.get::<_, String>(0)?,
                tracked: row.get::<_, i64>(1)? == 1,
                current_value: None, // Raids don't have rested values
                lazy_daily: None,
            })
        })?;

        let mut raids = Vec::new();
        for raid in raid_iter {
            raids.push(raid?);
        }

        Ok(raids)
    }

    /// Loads per-gate raid settings grouped by raid content id.
    pub fn get_character_raid_configs(&self, character_id: i64) -> Result<Vec<RaidConfig>> {
        let conn = self.pool.get()?;

        // Get all raid configurations for this character, grouped by content_id
        let mut stmt = conn.prepare(
            "SELECT content_id, gate, difficulty, take_gold, buy_box, reserved_for_static
             FROM conf_raid WHERE char_id = ?1 ORDER BY content_id, gate",
        )?;

        let raid_gate_iter = stmt.query_map([character_id], |row| {
            Ok((
                row.get::<_, String>(0)?,   // content_id
                row.get::<_, String>(1)?,   // gate
                row.get::<_, String>(2)?,   // difficulty
                row.get::<_, i64>(3)? == 1, // take_gold
                row.get::<_, i64>(4)? == 1, // buy_box
                row.get::<_, i64>(5)? == 1, // reserved_for_static
            ))
        })?;

        // Group gates by content_id
        let mut raid_configs: std::collections::HashMap<String, Vec<RaidGateConfig>> = std::collections::HashMap::new();
        for result in raid_gate_iter {
            let (content_id, gate, difficulty, take_gold, buy_box, reserved_for_static) = result?;

            let gate_config = RaidGateConfig {
                gate,
                difficulty,
                take_gold,
                buy_box,
                reserved_for_static,
            };

            raid_configs
                .entry(content_id)
                .or_insert_with(Vec::new)
                .push(gate_config);
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

    /// Replaces all raid gate settings for a character, preserving its roster id.
    ///
    /// Prefer `save_character_raid_configs_with_roster_id` when the caller has
    /// already loaded the active roster; this method is retained for older command
    /// paths that only pass a character id.
    pub fn save_character_raid_configs(&self, character_id: i64, configs: &[RaidConfig]) -> Result<()> {
        let roster_id = self.get_roster_id_for_character(character_id)?;
        self.save_character_raid_configs_with_roster_id(roster_id, character_id, configs)
    }

    /// Replaces all raid gate settings for a character within the active roster.
    pub fn save_character_raid_configs_with_roster_id(
        &self,
        roster_id: String,
        character_id: i64,
        configs: &[RaidConfig],
    ) -> Result<()> {
        let mut conn = self.pool.get()?;

        // Start transaction
        let tx = conn.transaction()?;

        // Clear existing configurations for this character and roster
        tx.execute(
            "DELETE FROM conf_raid WHERE roster_id = ?1 AND char_id = ?2",
            params![&roster_id, character_id],
        )?;

        // Insert new configurations
        for config in configs {
            for gate_config in &config.gates {
                tx.execute(
                    "INSERT INTO conf_raid (roster_id, char_id, content_id, gate, difficulty, take_gold, buy_box, reserved_for_static)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        &roster_id,
                        character_id,
                        config.content_id,
                        gate_config.gate,
                        gate_config.difficulty,
                        if gate_config.take_gold { 1 } else { 0 },
                        if gate_config.buy_box { 1 } else { 0 },
                        if gate_config.reserved_for_static { 1 } else { 0 }
                    ],
                )?;
            }
        }

        // Commit transaction
        tx.commit()?;

        Ok(())
    }

    /// Updates the raid-level tracking flag in `conf_tracking`.
    pub fn update_raid_config(&self, character_id: i64, raid_id: &str, tracked: bool) -> Result<()> {
        let conn = self.pool.get()?;
        let roster_id = self.get_roster_id_for_character(character_id)?;
        conn.execute(
            "INSERT INTO conf_tracking (roster_id, char_id, content_id, is_tracked, lazy_daily)
             VALUES (?1, ?2, ?3, ?4, 0)
             ON CONFLICT(char_id, content_id) DO UPDATE SET
               roster_id = excluded.roster_id,
               is_tracked = excluded.is_tracked",
            params![roster_id, character_id, raid_id, if tracked { 1 } else { 0 }],
        )?;
        Ok(())
    }

    /// Resolves the owning roster for commands that only receive a character id.
    fn get_roster_id_for_character(&self, character_id: i64) -> Result<String> {
        let conn = self.pool.get()?;
        let roster_id = conn.query_row(
            "SELECT roster_id FROM conf_character WHERE char_id = ?1",
            [character_id],
            |row| row.get::<_, String>(0),
        )?;

        Ok(roster_id)
    }
}
