use tauri::State;
use crate::database::DatabaseManager;

#[derive(serde::Serialize)]
pub struct RaidConfigResponse {
    char_id: i64,
    content_id: String,
    difficulty: String,
}

#[tauri::command]
pub async fn get_raid_configs_for_roster(
    roster_id: String,
    db: State<'_, DatabaseManager>
) -> Result<Vec<RaidConfigResponse>, String> {
    let conn = db.pool.get().map_err(|e: r2d2::Error| e.to_string())?;
    
    // Get all characters in this roster
    let mut chars_stmt = conn.prepare(
        "SELECT char_id FROM conf_character WHERE roster_id = ?1"
    ).map_err(|e: rusqlite::Error| e.to_string())?;
    
    let char_ids: Vec<i64> = chars_stmt.query_map([roster_id.clone()], |row: &rusqlite::Row| {
        Ok(row.get::<_, i64>(0)?)
    }).map_err(|e: rusqlite::Error| e.to_string())?
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e: rusqlite::Error| e.to_string())?;
    
    // Get all raid configs for these characters
    let mut configs = Vec::new();
    for char_id in char_ids {
        let mut raid_stmt = conn.prepare(
            "SELECT content_id, difficulty FROM conf_raid WHERE char_id = ?1"
        ).map_err(|e: rusqlite::Error| e.to_string())?;
        
        let raid_iter = raid_stmt.query_map([char_id], |row: &rusqlite::Row| {
            Ok(RaidConfigResponse {
                char_id,
                content_id: row.get::<_, String>(0)?,
                difficulty: row.get::<_, String>(1)?,
            })
        }).map_err(|e: rusqlite::Error| e.to_string())?;
        
        for raid_result in raid_iter {
            let config: Result<RaidConfigResponse, rusqlite::Error> = raid_result;
            configs.push(config.map_err(|e: rusqlite::Error| e.to_string())?);
        }
    }
    
    Ok(configs)
}
