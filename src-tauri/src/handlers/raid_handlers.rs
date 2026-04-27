use serde::{Deserialize, Serialize};
use tauri::State;
use crate::database::repositories::{RaidRepository, CharacterRepository};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Raid {
    pub id: String,
    pub name: String,
    pub difficulty: String,
    pub gates: Vec<RaidGate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidGate {
    pub gate: String,
    #[serde(rename = "minIlvl")]
    pub min_ilvl: i32,
    #[serde(rename = "tradableGold")]
    pub tradable_gold: i32,
    #[serde(rename = "boundGold")]
    pub bound_gold: i32,
    #[serde(rename = "boxPrice")]
    pub box_price: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidMatrixItem {
    pub raid_id: String,
    pub raid_name: String,
    pub min_ilvl: i64,
    pub character_states: Vec<CharacterRaidState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRaidState {
    pub char_id: i64,
    pub tracked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub char_id: i64,
    pub char_name: String,
    pub item_level: f64,
    pub combat_power: f64,
    pub class_id: String,
    pub earns_gold: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidConfig {
    pub content_id: String,
    pub gates: Vec<RaidGateConfig>,
    pub take_gold: bool,
    pub buy_box: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidGateConfig {
    pub gate: String,
    pub difficulty: String,
    pub take_gold: bool,
    pub buy_box: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidMatrixResponse {
    pub characters: Vec<CharacterInfo>,
    pub raid_configs: Vec<CharacterRaidConfigs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRaidConfigs {
    pub char_id: i64,
    pub char_name: String,
    pub item_level: f64,
    pub class_id: String,
    pub earns_gold: bool,
    pub raid_configs: Vec<RaidConfig>,
}

#[tauri::command]
pub async fn get_game_raids(
    raids: Vec<Raid>
) -> Result<Vec<crate::models::Raid>, String> {
    // Convert frontend raids to backend model
    let backend_raids = raids.into_iter().map(|raid| {
        crate::models::Raid {
            id: raid.id,
            name: raid.name,
            difficulty: raid.difficulty,
            min_ilvl: raid.gates.first().map(|g| g.min_ilvl as i64).unwrap_or(0),
            max_players: 4,
            gates: raid.gates.into_iter().map(|g| {
                let gate_name = g.gate.clone();
                crate::models::RaidGate {
                    gate: g.gate,
                    name: gate_name,
                    min_ilvl: g.min_ilvl as i64,
                    tradable_gold: Some(g.tradable_gold),
                    bound_gold: Some(g.bound_gold),
                    box_price: Some(g.box_price),
                }
            }).collect(),
        }
    }).collect();
    
    Ok(backend_raids)
}

#[tauri::command]
pub async fn get_character_raid_config(
    character_id: i64,
    raid_repo: State<'_, RaidRepository>
) -> Result<Vec<crate::models::CharacterRaidState>, String> {
    raid_repo.get_character_raid_config(character_id)
        .map_err(|e| format!("Failed to get character raid config: {}", e))
}

#[tauri::command]
pub async fn update_raid_config(
    character_id: i64,
    raid_id: String,
    tracked: bool,
    raid_repo: State<'_, RaidRepository>
) -> Result<(), String> {
    raid_repo.update_raid_config(character_id, &raid_id, tracked)
        .map_err(|e| format!("Failed to update raid config: {}", e))
}

#[tauri::command]
pub async fn get_raid_gate_matrix(
    _roster_id: String,
    _raid_repo: State<'_, RaidRepository>,
    _character_repo: State<'_, CharacterRepository>
) -> Result<Vec<crate::models::RaidMatrixItem>, String> {
    // Placeholder implementation
    Ok(vec![])
}

#[tauri::command]
pub async fn get_raid_matrix_data(
    rosterId: String,
    raid_repo: State<'_, RaidRepository>,
    character_repo: State<'_, CharacterRepository>
) -> Result<RaidMatrixResponse, String> {
    // Get characters for the roster
    let characters = character_repo.get_characters_by_roster(&rosterId)
        .map_err(|e| format!("Failed to get characters: {}", e))?;
    
    // Get all raid configurations for these characters
    let mut raid_configs = Vec::new();
    for character in &characters {
        let char_raid_configs = raid_repo.get_character_raid_configs(character.char_id)
            .map_err(|e| format!("Failed to get raid configs: {}", e))?;
        
        raid_configs.push(CharacterRaidConfigs {
            char_id: character.char_id,
            char_name: character.char_name.clone(),
            item_level: character.item_level,
            class_id: character.class_id.clone(),
            earns_gold: character.earns_gold,
            raid_configs: char_raid_configs,
        });
    }
    
    // Convert characters to CharacterInfo
    let character_infos: Vec<CharacterInfo> = characters.into_iter().map(|char| {
        CharacterInfo {
            char_id: char.char_id,
            char_name: char.char_name,
            item_level: char.item_level,
            combat_power: char.combat_power,
            class_id: char.class_id,
            earns_gold: char.earns_gold,
        }
    }).collect();
    
    Ok(RaidMatrixResponse {
        characters: character_infos,
        raid_configs,
    })
}

#[tauri::command]
pub async fn update_raid_master_config(
    charId: i64,
    contentId: String,
    takeGold: Option<bool>,
    buyBox: Option<bool>,
    raid_repo: State<'_, RaidRepository>
) -> Result<(), String> {
    // Get current configuration for this character and raid
    let mut configs = raid_repo.get_character_raid_configs(charId)
        .map_err(|e| format!("Failed to get current raid config: {}", e))?;
    
    // Find the raid config
    if let Some(raid_config) = configs.iter_mut().find(|c| c.content_id == contentId) {
        // Update master settings
        if let Some(take_gold) = takeGold {
            raid_config.take_gold = take_gold;
        }
        if let Some(buy_box) = buyBox {
            raid_config.buy_box = buy_box;
        }
        
        // Save the updated configuration
        for gate_config in &mut raid_config.gates {
            if let Some(take_gold) = takeGold {
                gate_config.take_gold = take_gold;
            }
            if let Some(buy_box) = buyBox {
                gate_config.buy_box = buy_box;
            }
        }
        
        raid_repo.save_character_raid_configs(charId, &configs)
            .map_err(|e| format!("Failed to save raid config: {}", e))?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn update_raid_gate_config(
    rosterId: String,
    charId: i64,
    contentId: String,
    gate: String,
    difficulty: String,
    takeGold: Option<bool>,
    buyBox: Option<bool>,
    raid_repo: State<'_, RaidRepository>
) -> Result<(), String> {
    // Get current configuration for this character and raid
    let mut configs = raid_repo.get_character_raid_configs(charId)
        .map_err(|e| format!("Failed to get current raid config: {}", e))?;
    
    // Find the raid config
    if let Some(raid_config) = configs.iter_mut().find(|c| c.content_id == contentId) {
        // Find the specific gate
        if let Some(gate_config) = raid_config.gates.iter_mut().find(|g| g.gate == gate) {
            // Update gate settings
            gate_config.difficulty = difficulty;
            if let Some(take_gold) = takeGold {
                gate_config.take_gold = take_gold;
            }
            if let Some(buy_box) = buyBox {
                gate_config.buy_box = buy_box;
            }
            
            // Save the updated configuration with roster_id
            raid_repo.save_character_raid_configs_with_roster_id(rosterId, charId, &configs)
                .map_err(|e| format!("Failed to save raid config: {}", e))?;
        }
    }
    
    Ok(())
}
