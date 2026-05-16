use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterEngravingRow {
    pub id: i64,
    pub character_id: i64,
    pub engraving_name: String,
    pub books_read: i64,
    pub max_books: i64,
    pub stone_bonus: i64,
    pub is_manual_entry: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterEquipmentRow {
    pub id: i64,
    pub character_id: i64,
    pub slot: String,
    pub enhancement_level: Option<i64>,
    pub tier: Option<String>,
    pub quality: Option<i64>,
    pub item_level: Option<f64>,
    pub is_manual_entry: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterGemRow {
    pub id: i64,
    pub character_id: i64,
    pub slot_index: i64,
    pub gem_name: String,
    pub skill_name: String,
    pub gem_type: String,
    pub gem_level: i64,
    pub is_bound: bool,
    pub is_manual_entry: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionGoalRow {
    pub id: i64,
    pub character_id: i64,
    pub goal_type: String,
    pub target_name: String,
    pub target_value: i64,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterProgressionSnapshot {
    pub character_id: i64,
    pub engravings: Vec<CharacterEngravingRow>,
    pub equipment: Vec<CharacterEquipmentRow>,
    pub gems: Vec<CharacterGemRow>,
    pub goals: Vec<ProgressionGoalRow>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterEngravingInput {
    pub engraving_name: String,
    pub books_read: i64,
    pub max_books: i64,
    pub stone_bonus: i64,
    pub is_manual_entry: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterEquipmentInput {
    pub slot: String,
    pub enhancement_level: Option<i64>,
    pub tier: Option<String>,
    pub quality: Option<i64>,
    pub item_level: Option<f64>,
    pub is_manual_entry: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterGemInput {
    pub slot_index: i64,
    pub gem_name: String,
    pub skill_name: String,
    pub gem_type: String,
    pub gem_level: i64,
    pub is_bound: bool,
    pub is_manual_entry: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionGoalInput {
    pub goal_type: String,
    pub target_name: String,
    pub target_value: i64,
    pub completed_at: Option<i64>,
}

pub struct ProgressionRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl ProgressionRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        Self { pool }
    }

    fn now_ts() -> i64 {
        chrono::Utc::now().timestamp()
    }

    pub fn get_snapshot(&self, character_id: i64) -> Result<CharacterProgressionSnapshot> {
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare(
            "SELECT id, character_id, engraving_name, books_read, max_books, stone_bonus, is_manual_entry, updated_at
             FROM character_engravings WHERE character_id = ?1 ORDER BY engraving_name",
        )?;
        let engravings = stmt
            .query_map([character_id], |row| {
                Ok(CharacterEngravingRow {
                    id: row.get(0)?,
                    character_id: row.get(1)?,
                    engraving_name: row.get(2)?,
                    books_read: row.get(3)?,
                    max_books: row.get(4)?,
                    stone_bonus: row.get(5)?,
                    is_manual_entry: row.get::<_, i64>(6)? != 0,
                    updated_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut stmt = conn.prepare(
            "SELECT id, character_id, slot, enhancement_level, tier, quality, item_level, is_manual_entry, updated_at
             FROM character_equipment WHERE character_id = ?1 ORDER BY slot",
        )?;
        let equipment = stmt
            .query_map([character_id], |row| {
                Ok(CharacterEquipmentRow {
                    id: row.get(0)?,
                    character_id: row.get(1)?,
                    slot: row.get(2)?,
                    enhancement_level: row.get(3)?,
                    tier: row.get(4)?,
                    quality: row.get(5)?,
                    item_level: row.get(6)?,
                    is_manual_entry: row.get::<_, i64>(7)? != 0,
                    updated_at: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut stmt = conn.prepare(
            "SELECT id, character_id, slot_index, gem_name, skill_name, gem_type, gem_level, is_bound, is_manual_entry, updated_at
             FROM character_gems WHERE character_id = ?1 ORDER BY slot_index",
        )?;
        let gems = stmt
            .query_map([character_id], |row| {
                Ok(CharacterGemRow {
                    id: row.get(0)?,
                    character_id: row.get(1)?,
                    slot_index: row.get(2)?,
                    gem_name: row.get(3)?,
                    skill_name: row.get(4)?,
                    gem_type: row.get(5)?,
                    gem_level: row.get(6)?,
                    is_bound: row.get::<_, i64>(7)? != 0,
                    is_manual_entry: row.get::<_, i64>(8)? != 0,
                    updated_at: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut stmt = conn.prepare(
            "SELECT id, character_id, goal_type, target_name, target_value, created_at, completed_at
             FROM progression_goals WHERE character_id = ?1 ORDER BY goal_type, target_name",
        )?;
        let goals = stmt
            .query_map([character_id], |row| {
                Ok(ProgressionGoalRow {
                    id: row.get(0)?,
                    character_id: row.get(1)?,
                    goal_type: row.get(2)?,
                    target_name: row.get(3)?,
                    target_value: row.get(4)?,
                    created_at: row.get(5)?,
                    completed_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(CharacterProgressionSnapshot {
            character_id,
            engravings,
            equipment,
            gems,
            goals,
        })
    }

    pub fn replace_engravings(&self, character_id: i64, rows: &[CharacterEngravingInput]) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        let ts = Self::now_ts();
        tx.execute(
            "DELETE FROM character_engravings WHERE character_id = ?1",
            params![character_id],
        )?;
        for r in rows {
            let manual = if r.is_manual_entry { 1 } else { 0 };
            tx.execute(
                "INSERT INTO character_engravings (character_id, engraving_name, books_read, max_books, stone_bonus, is_manual_entry, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    character_id,
                    &r.engraving_name,
                    r.books_read,
                    r.max_books,
                    r.stone_bonus,
                    manual,
                    ts,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn replace_equipment(&self, character_id: i64, rows: &[CharacterEquipmentInput]) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        let ts = Self::now_ts();
        tx.execute(
            "DELETE FROM character_equipment WHERE character_id = ?1",
            params![character_id],
        )?;
        for r in rows {
            let manual = if r.is_manual_entry { 1 } else { 0 };
            tx.execute(
                "INSERT INTO character_equipment (character_id, slot, enhancement_level, tier, quality, item_level, is_manual_entry, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    character_id,
                    &r.slot,
                    r.enhancement_level,
                    r.tier,
                    r.quality,
                    r.item_level,
                    manual,
                    ts,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn replace_gems(&self, character_id: i64, rows: &[CharacterGemInput]) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        let ts = Self::now_ts();
        tx.execute(
            "DELETE FROM character_gems WHERE character_id = ?1",
            params![character_id],
        )?;
        for r in rows {
            let manual = if r.is_manual_entry { 1 } else { 0 };
            tx.execute(
                "INSERT INTO character_gems (character_id, skill_name, gem_type, gem_level, is_manual_entry, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    character_id,
                    &r.skill_name,
                    &r.gem_type,
                    r.gem_level,
                    manual,
                    ts,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Replace engravings, equipment, and gems in one transaction (e.g. after a scrape).
    pub fn replace_scraped_progression(
        &self,
        character_id: i64,
        engravings: &[CharacterEngravingInput],
        equipment: &[CharacterEquipmentInput],
        gems: &[CharacterGemInput],
    ) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        let ts = Self::now_ts();

        tx.execute(
            "DELETE FROM character_engravings WHERE character_id = ?1",
            params![character_id],
        )?;
        for r in engravings {
            let manual = if r.is_manual_entry { 1 } else { 0 };
            tx.execute(
                "INSERT INTO character_engravings (character_id, engraving_name, books_read, max_books, stone_bonus, is_manual_entry, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    character_id,
                    &r.engraving_name,
                    r.books_read,
                    r.max_books,
                    r.stone_bonus,
                    manual,
                    ts,
                ],
            )?;
        }

        tx.execute(
            "DELETE FROM character_equipment WHERE character_id = ?1",
            params![character_id],
        )?;
        for r in equipment {
            let manual = if r.is_manual_entry { 1 } else { 0 };
            tx.execute(
                "INSERT INTO character_equipment (character_id, slot, enhancement_level, tier, quality, item_level, is_manual_entry, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    character_id,
                    &r.slot,
                    r.enhancement_level,
                    r.tier,
                    r.quality,
                    r.item_level,
                    manual,
                    ts,
                ],
            )?;
        }

        tx.execute(
            "DELETE FROM character_gems WHERE character_id = ?1",
            params![character_id],
        )?;
        for r in gems {
            let manual = if r.is_manual_entry { 1 } else { 0 };
            let bound = if r.is_bound { 1 } else { 0 };
            tx.execute(
                "INSERT INTO character_gems (character_id, slot_index, gem_name, skill_name, gem_type, gem_level, is_bound, is_manual_entry, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    character_id,
                    r.slot_index,
                    &r.gem_name,
                    &r.skill_name,
                    &r.gem_type,
                    r.gem_level,
                    bound,
                    manual,
                    ts,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn upsert_goal(&self, character_id: i64, input: &ProgressionGoalInput) -> Result<i64> {
        let conn = self.pool.get()?;
        let ts = Self::now_ts();

        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM progression_goals WHERE character_id = ?1 AND goal_type = ?2 AND target_name = ?3",
                params![character_id, &input.goal_type, &input.target_name],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = existing {
            conn.execute(
                "UPDATE progression_goals SET target_value = ?1, completed_at = ?2 WHERE id = ?3",
                params![input.target_value, input.completed_at, id],
            )?;
            Ok(id)
        } else {
            conn.execute(
                "INSERT INTO progression_goals (character_id, goal_type, target_name, target_value, created_at, completed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    character_id,
                    &input.goal_type,
                    &input.target_name,
                    input.target_value,
                    ts,
                    input.completed_at,
                ],
            )?;
            Ok(conn.last_insert_rowid())
        }
    }

    pub fn delete_goal(&self, goal_id: i64) -> Result<bool> {
        let conn = self.pool.get()?;
        let n = conn.execute("DELETE FROM progression_goals WHERE id = ?1", params![goal_id])?;
        Ok(n > 0)
    }
}
