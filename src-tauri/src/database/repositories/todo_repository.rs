use anyhow::Result;
use rusqlite::params;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::database::data_manager::{Raid, RaidGate};

// Todo-specific data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoMatrixResponse {
    pub characters: Vec<TodoCharacter>,
    pub daily_tasks: Vec<TodoTask>,
    pub roster_tasks: Vec<TodoTask>,
    pub weekly_tasks: Vec<TodoTask>,
    pub raids: Vec<TodoRaid>,
    pub character_states: Option<std::collections::HashMap<String, CharacterTaskState>>,
    pub rested_entries: Option<Vec<(i64, String, i64)>>,
    pub todo_entries: Option<Vec<(i64, String, bool)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoCharacter {
    pub id: i64,
    pub name: String,
    pub class: String,
    pub ilvl: f64,
    pub combat_power: f64,
    pub earns_gold: bool,
    pub display_order: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoTask {
    pub id: String,
    pub name: String,
    pub category: String,
    pub reset_schedule: String,
    pub logic_type: String,
    pub max_rest_value: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterTaskState {
    pub tracked: bool,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoRaid {
    pub id: String,
    pub name: String,
    pub difficulty: String,
    pub gates: Vec<TodoRaidGate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoRaidGate {
    pub gate: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRaidState {
    pub raid_id: String,
    pub gates: Vec<RaidGateState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidGateState {
    pub gate: String,
    pub cleared: bool,
    pub clear_time: Option<String>,
}

pub struct TodoRepository {
    pub(crate) pool: Arc<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>,
}

impl TodoRepository {
    pub fn new(pool: Arc<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>) -> Self {
        Self { pool }
    }

    pub fn get_todo_matrix(&self, roster_id: &str) -> Result<TodoMatrixResponse> {
        let conn = self.pool.get()?;
        
        // Get characters for this roster
        let mut stmt = conn.prepare(
            "SELECT char_id, char_name, class_id, item_level, combat_power, earns_gold, display_order 
             FROM conf_character WHERE roster_id = ?1 
             ORDER BY display_order, char_name"
        )?;
        
        let character_iter = stmt.query_map([roster_id], |row| {
            Ok(TodoCharacter {
                id: row.get(0)?,
                name: row.get(1)?,
                class: row.get(2)?,
                ilvl: row.get(3)?,
                combat_power: row.get(4)?,
                earns_gold: row.get(5)?,
                display_order: row.get(6)?,
            })
        })?;
        
        let mut characters = Vec::new();
        for character in character_iter {
            characters.push(character?);
        }
        
        // Load all tracked tasks from conf_tracking (like tracking_repository does)
        let mut todo_stmt = conn.prepare(
            "SELECT char_id, content_id, is_tracked FROM conf_tracking WHERE roster_id = ?1"
        )?;
        
        let todo_iter = todo_stmt.query_map([roster_id], |row| {
            Ok((
                row.get::<_, i64>(0)?, // char_id
                row.get::<_, String>(1)?, // content_id
                row.get::<_, i64>(2)? == 1, // is_tracked
            ))
        })?;
        
        let todo_entries: Vec<(i64, String, bool)> = todo_iter.filter_map(Result::ok).collect();
        
        // Load rested values
        let mut rested_stmt = conn.prepare(
            "SELECT char_id, content_id, current_value 
             FROM rested_values 
             WHERE char_id IN (
                 SELECT char_id FROM conf_character WHERE roster_id = ?1
             )"
        )?;
        
        let rested_iter = rested_stmt.query_map([roster_id], |row| {
            Ok((
                row.get::<_, i64>(0)?, // char_id
                row.get::<_, String>(1)?, // content_id
                row.get::<_, i64>(2)?, // current_value
            ))
        })?;
        
        let rested_entries: Vec<(i64, String, i64)> = rested_iter.filter_map(Result::ok).collect();
        
        // Create tasks from unique content_ids in todo_entries
        let mut task_map = std::collections::HashMap::new();
        for (_, content_id, _) in &todo_entries {
            if !task_map.contains_key(content_id) {
                task_map.insert(content_id.clone(), TodoTask {
                    id: content_id.clone(),
                    name: content_id.clone(),
                    category: "unknown".to_string(),
                    reset_schedule: "daily".to_string(),
                    logic_type: "normal".to_string(),
                    max_rest_value: None,
                });
            }
        }
        
        let mut tasks: Vec<TodoTask> = task_map.into_values().collect();
        
        // Load raids from conf_raid and group by raid_id
        let mut raid_map = std::collections::HashMap::new();
        for character in &characters {
            let mut raid_stmt = conn.prepare(
                "SELECT DISTINCT content_id FROM conf_raid WHERE char_id = ?1"
            )?;
            
            let raid_iter = raid_stmt.query_map([character.id], |row| {
                Ok(row.get::<_, String>(0)?) // content_id
            })?;
            
            for raid_result in raid_iter {
                let content_id = raid_result?;
                if !raid_map.contains_key(&content_id) {
                    // Load gates for this raid from conf_raid
                    let mut gate_stmt = conn.prepare(
                        "SELECT gate FROM conf_raid WHERE char_id = ?1 AND content_id = ?2"
                    )?;
                    
                    let gate_iter = gate_stmt.query_map(params![character.id, content_id.clone()], |row| {
                        Ok(row.get::<_, String>(0)?) // gate
                    })?;
                    
                    let mut gates = Vec::new();
                    for gate_result in gate_iter {
                        gates.push(gate_result?);
                    }
                    
                    // Use content_id as name since content_name doesn't exist
                    raid_map.insert(content_id.clone(), TodoRaid {
                        id: content_id.clone(),
                        name: content_id, // Use content_id as name
                        difficulty: "normal".to_string(), // Default difficulty
                        gates: gates.into_iter().map(|gate| TodoRaidGate { gate: gate.clone(), name: gate }).collect(),
                    });
                }
            }
        }
        
        let raids: Vec<TodoRaid> = raid_map.into_values().collect();
        
        // Create character states using todo_entries and rested_entries (like tracking_repository)
        let mut character_states = std::collections::HashMap::new();
        
        // Get first character ID for roster task checking
        let first_char_id = characters.first().map(|c| c.id);
        // Track which roster tasks have already been processed to avoid duplicates
        let mut processed_roster_tasks = std::collections::HashSet::new();
        
        for character in &characters {
            for (char_id, content_id, is_tracked) in &todo_entries {
                // Create state for character-specific tasks
                if *char_id == character.id {
                    // Get rested value if exists, default to 0
                    let current_value = rested_entries
                        .iter()
                        .find(|(rested_char_id, rested_content_id, _)| 
                            rested_char_id == char_id && rested_content_id == content_id)
                        .map(|(_, _, value)| *value)
                        .unwrap_or(0);
                    
                    // Check if completed - special handling for gate/boss tasks
                    let completed = if content_id == "gate" || content_id == "boss" {
                        // For gate/boss tasks, check for both roster task entries (session_id IS NULL) 
                        // and individual gate entries (session_id LIKE 'boss_%' OR 'gate_%')
                        if let Ok(conn) = self.pool.get() {
                            let count: i64 = conn.query_row(
                                "SELECT COUNT(*) FROM completion_status 
                                 WHERE char_id = ?1 AND content_id = ?2 
                                 AND (session_id IS NULL OR session_id LIKE ?3)",
                                params![character.id, content_id, format!("{}_%", content_id)],
                                |row| row.get(0)
                            ).unwrap_or(0);
                            
                            count > 0
                        } else {
                            false
                        }
                    } else {
                        // For other tasks, use the regular method
                        if let Ok(is_completed) = self.get_task_completed(character.id, content_id) {
                            is_completed
                        } else {
                            false
                        }
                    };
                    
                    let state = CharacterTaskState {
                        tracked: *is_tracked,
                        completed,
                    };
                    
                    character_states.insert(format!("{}_{}", character.id, content_id), state);
                }
                
                // Create roster task states for all characters using first character's entry
                if let Some(first_id) = first_char_id {
                    if *char_id == first_id && !processed_roster_tasks.contains(content_id.as_str()) {
                        // Check if this is a roster task by checking if it exists in completion_status with the first character
                        // and has session_id IS NULL (indicating it's a roster-wide task)
                        let is_roster_task = if let Ok(conn) = self.pool.get() {
                            let mut stmt = conn.prepare(
                                "SELECT COUNT(*) FROM completion_status WHERE char_id = ?1 AND content_id = ?2 AND session_id IS NULL"
                            );
                            
                            if let Ok(mut stmt) = stmt {
                                if let Ok(count) = stmt.query_row(params![first_id, content_id], |row| row.get::<_, i64>(0)) {
                                    count > 0
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        
                        if is_roster_task {
                            processed_roster_tasks.insert(content_id.clone());
                            
                            // Get completion status from first character's entry (per-roster)
                            let completed = if let Ok(is_completed) = self.get_task_completed(first_id, content_id) {
                                is_completed
                            } else {
                                false
                            };
                            
                            // Create state for all characters in roster for this roster task
                            for roster_char in &characters {
                                let state = CharacterTaskState {
                                    tracked: true, // Roster tasks are always tracked
                                    completed,
                                };
                                
                                let key = format!("{}_{}", roster_char.id, content_id);
                                character_states.insert(key.clone(), state);
                            }
                        }
                    }
                }
            }
        }
        
        // Create matrix with basic data - frontend will handle task definitions
        let matrix = TodoMatrixResponse {
            characters,
            daily_tasks: vec![], // Frontend will populate from GAME_TASKS
            roster_tasks: vec![], // Frontend will populate from GAME_TASKS  
            weekly_tasks: vec![], // Frontend will populate from GAME_TASKS
            raids,
            character_states: Some(character_states),
            rested_entries: Some(rested_entries),
            todo_entries: Some(todo_entries),
        };
        
        Ok(matrix)
    }

    pub fn is_task_tracked(&self, char_id: i64, task_id: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        
        let mut stmt = conn.prepare(
            "SELECT is_tracked FROM conf_tracking WHERE char_id = ?1 AND content_id = ?2"
        )?;
        
        let result: i64 = stmt.query_row(params![char_id, task_id], |row| row.get(0))?;
        
        Ok(result == 1)
    }

    pub fn get_task_completed(&self, char_id: i64, task_id: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        
        // First try to find regular task entry
        let mut stmt = conn.prepare(
            "SELECT is_completed FROM completion_status WHERE char_id = ?1 AND content_id = ?2 AND session_id IS NOT NULL"
        )?;
        
        match stmt.query_row(params![char_id, task_id], |row| row.get::<_, i64>(0)) {
            Ok(result) => Ok(result == 1),
            Err(_) => {
                // If not found, try roster task entry (session_id IS NULL)
                let mut stmt = conn.prepare(
                    "SELECT is_completed FROM completion_status WHERE char_id = ?1 AND content_id = ?2 AND session_id IS NULL"
                )?;
                
                match stmt.query_row(params![char_id, task_id], |row| row.get::<_, i64>(0)) {
                    Ok(result) => Ok(result == 1),
                    Err(_) => Ok(false) // Not found anywhere
                }
            }
        }
    }

    pub fn get_raid_gate_difficulty(&self, char_id: i64, raid_id: &str, gate_id: &str) -> Result<Option<String>> {
        let conn = self.pool.get()?;
        
        let mut stmt = conn.prepare(
            "SELECT difficulty FROM conf_raid WHERE char_id = ?1 AND content_id = ?2 AND gate = ?3"
        )?;
        
        let result = stmt.query_row(params![char_id, raid_id, gate_id], |row| {
            Ok(Some(row.get::<_, String>(0)?))
        });
        
        match result {
            Ok(difficulty) => Ok(difficulty),
            Err(_) => Ok(None) // Return None if no configuration found
        }
    }

    pub fn get_raid_gate_completed(&self, char_id: i64, raid_id: &str, gate_id: &str, difficulty: &str) -> Result<Option<bool>> {
        let conn = self.pool.get()?;
        
        // First try with the provided gate_id (could be boss name from encounterMap)
        let base_session_id = format!("{}_{}", raid_id, gate_id);
        
        let mut stmt = conn.prepare(
            "SELECT is_completed FROM completion_status WHERE char_id = ?1 AND session_id = ?2"
        )?;
        
        match stmt.query_row(params![char_id, &base_session_id], |row| {
            let completed: i64 = row.get::<_, i64>(0)?;
            Ok(completed)
        }) {
            Ok(completed) => Ok(Some(completed == 1)),
            Err(_) => {
                // If boss name mapping failed, try to extract gate number and use Gate format
                if gate_id.contains("Gate") {
                    // gate_id is already in Gate format, try content_id fallback
                    let mut stmt = conn.prepare(
                        "SELECT is_completed FROM completion_status WHERE char_id = ?1 AND content_id = ?2 AND session_id IS NULL"
                    )?;
                    
                    match stmt.query_row(params![char_id, raid_id], |row| {
                        Ok(row.get::<_, i64>(0)?)
                    }) {
                        Ok(completed) => Ok(Some(completed == 1)),
                        Err(_) => Ok(None)
                    }
                } else {
                    // gate_id is a boss name, try to find corresponding gate entry
                    let mut stmt = conn.prepare(
                        "SELECT is_completed FROM completion_status WHERE char_id = ?1 AND content_id = ?2 AND session_id LIKE ?3"
                    )?;
                    
                    let gate_pattern = format!("{}_Gate %", raid_id);
                    match stmt.query_row(params![char_id, raid_id, &gate_pattern], |row| {
                        Ok(row.get::<_, i64>(0)?)
                    }) {
                        Ok(completed) => Ok(Some(completed == 1)),
                        Err(_) => {
                            // Final fallback to content_id with NULL session_id
                            let mut stmt = conn.prepare(
                                "SELECT is_completed FROM completion_status WHERE char_id = ?1 AND content_id = ?2 AND session_id IS NULL"
                            )?;
                            
                            match stmt.query_row(params![char_id, raid_id], |row| {
                                Ok(row.get::<_, i64>(0)?)
                            }) {
                                Ok(completed) => Ok(Some(completed == 1)),
                                Err(_) => Ok(None)
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn get_rested_value(&self, char_id: i64, task_id: &str) -> Result<i64> {
        let conn = self.pool.get()?;
        
        let mut stmt = conn.prepare(
            "SELECT current_value FROM rested_values WHERE char_id = ?1 AND content_id = ?2"
        )?;
        
        let result: i64 = stmt.query_row(params![char_id, task_id], |row| row.get(0))?;
        Ok(result)
    }

    /// Check if a task is a roster-wide task by checking game_tasks table
    fn is_roster_task(&self, task_id: &str) -> bool {
        if let Ok(conn) = self.pool.get() {
            // Check if game_tasks table exists first
            let mut check_stmt = conn.prepare(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='game_tasks'"
            );
            
            if let Ok(mut stmt) = check_stmt {
                if let Ok(table_exists) = stmt.query_row([], |row| {
                    let name: String = row.get(0)?;
                    Ok(name == "game_tasks")
                }) {
                    if table_exists {
                        let mut stmt = conn.prepare(
                            "SELECT COUNT(*) FROM game_tasks WHERE id = ?1 AND category = 'roster'"
                        ).unwrap_or_else(|_| {
                            // If prepare fails, return false
                            conn.prepare("SELECT 0 WHERE 0").unwrap()
                        });
                        
                        if let Ok(count) = stmt.query_row([task_id], |row| row.get::<_, i64>(0)) {
                            return count > 0;
                        }
                    }
                }
            }
        }
        false
    }

    pub fn set_task_completed(&self, char_id: i64, task_id: &str, completed: bool) -> Result<()> {
        let conn = self.pool.get()?;
        
        // Get roster_id for this character
        let mut stmt = conn.prepare(
            "SELECT roster_id FROM conf_character WHERE char_id = ?1"
        )?;
        
        let roster_id: String = stmt.query_row([char_id], |row| row.get(0))?;
        
        let completed_value = if completed { 1i64 } else { 0i64 };
        let timestamp = chrono::Utc::now().timestamp_millis();
        
        // Check if entry already exists
        let mut stmt = conn.prepare(
            "SELECT is_completed FROM completion_status WHERE char_id = ?1 AND content_id = ?2"
        )?;
        
        match stmt.query_row(params![char_id, task_id], |row| {
            Ok(row.get::<_, i64>(0)?)
        }) {
            Ok(_existing_completed) => {
                // Update existing entry
                conn.execute(
                    "UPDATE completion_status SET is_completed = ?1, timestamp = ?2 WHERE char_id = ?3 AND content_id = ?4",
                    params![completed_value, timestamp, char_id, task_id]
                )?;
            }
            Err(_) => {
                // Insert new entry
                let session_id = format!("{}_{:x}", chrono::Utc::now().timestamp_millis(), rand::random::<u32>());
                
                conn.execute(
                    "INSERT INTO completion_status (roster_id, char_id, content_id, is_completed, timestamp, session_id) 
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![roster_id, char_id, task_id, completed_value, timestamp, session_id]
                )?;
            }
        }

        // Handle rested value consumption for chaos/guardian on completion
        if completed && (task_id == "chaos" || task_id == "guardian") {
            self.consume_rested_on_completion(char_id, task_id)?;
        }
        
        Ok(())
    }

    /// Consume 20 rested points when chaos/guardian is completed (if available)
    fn consume_rested_on_completion(&self, char_id: i64, task_id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        
        // Get roster_id for this character
        let roster_id: String = conn.query_row(
            "SELECT roster_id FROM conf_character WHERE char_id = ?1",
            [char_id],
            |row| row.get(0)
        )?;
        
        // Get current rested value
        let current_rested = conn.query_row(
            "SELECT current_value FROM rested_values WHERE char_id = ?1 AND content_id = ?2",
            params![char_id, task_id],
            |row| Ok(row.get::<_, i64>(0)?)
        ).unwrap_or(0);

        // Only consume if we have at least 20 points
        if current_rested >= 20 {
            let new_rested = current_rested - 20;
            let last_updated = chrono::Utc::now().timestamp_millis();
            
            conn.execute(
                "INSERT OR REPLACE INTO rested_values (roster_id, char_id, content_id, current_value, last_updated) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![roster_id, char_id, task_id, new_rested, last_updated],
            )?;
            
            println!("Consumed 20 rested points for {} (character {}): {} -> {}", 
                     task_id, char_id, current_rested, new_rested);
        }

        Ok(())
    }

    pub fn set_raid_gate_completed(&self, char_id: i64, task_id: &str, completed: bool, difficulty: &str, gate_id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        
        // Get roster_id for this character
        let mut stmt = conn.prepare(
            "SELECT roster_id FROM conf_character WHERE char_id = ?1"
        )?;
        
        let roster_id: String = stmt.query_row([char_id], |row| row.get(0))?;
        
        let completed_value = if completed { 1i64 } else { 0i64 };
        let timestamp = chrono::Utc::now().timestamp_millis();
        
        // Create base session_id without difficulty for consistent identification
        let base_session_id = format!("{}_{}", task_id, gate_id);
        
        // Check if entry already exists for this specific gate
        let mut stmt = conn.prepare(
            "SELECT is_completed FROM completion_status WHERE char_id = ?1 AND content_id = ?2 AND session_id = ?3"
        )?;
        
        match stmt.query_row(params![char_id, task_id, &base_session_id], |row| {
            Ok(row.get::<_, i64>(0)?)
        }) {
            Ok(_existing_completed) => {
                // Update existing entry
                conn.execute(
                    "UPDATE completion_status SET is_completed = ?1, timestamp = ?2, details = ?3 WHERE char_id = ?4 AND content_id = ?5 AND session_id = ?6",
                    params![completed_value, timestamp, difficulty, char_id, task_id, &base_session_id]
                )?;
            }
            Err(_) => {
                // Insert new entry
                conn.execute(
                    "INSERT INTO completion_status (roster_id, char_id, content_id, is_completed, timestamp, session_id, completion_source, details) 
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![roster_id, char_id, task_id, completed_value, timestamp, &base_session_id, "manual", difficulty]
                )?;
            }
        }
        
                
        Ok(())
    }

    
    pub fn set_roster_task_completed(&self, roster_id: &str, task_id: &str, completed: bool) -> Result<()> {
        let conn = self.pool.get()?;
        
        let completed_value = if completed { 1i64 } else { 0i64 };
        let timestamp = chrono::Utc::now().timestamp_millis();
        
        // Get first character in roster for foreign key constraint
        let mut stmt = conn.prepare(
            "SELECT char_id FROM conf_character WHERE roster_id = ?1 LIMIT 1"
        )?;
        
        let char_id: i64 = stmt.query_row([roster_id], |row| row.get(0))?;
        
        // Check if entry already exists for this roster and task
        let mut stmt = conn.prepare(
            "SELECT rowid FROM completion_status WHERE roster_id = ?1 AND char_id = ?2 AND content_id = ?3"
        )?;
        
        let exists = stmt.exists(params![roster_id, char_id, task_id])?;

        if exists {
            // If yes: UPDATE
            conn.execute(
                "UPDATE completion_status SET is_completed = ?1, timestamp = ?2 
                 WHERE roster_id = ?3 AND char_id = ?4 AND content_id = ?5",
                params![completed_value, timestamp, roster_id, char_id, task_id],
            )?;
        } else {
            // If no: INSERT
            conn.execute(
                "INSERT INTO completion_status (roster_id, char_id, content_id, is_completed, timestamp, completion_source) 
                 VALUES (?1, ?2, ?3, ?4, ?5, 'manual')",
                params![roster_id, char_id, task_id, completed_value, timestamp],
            )?;
        }
        
        Ok(())
    }
}
