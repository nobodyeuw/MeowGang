use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};
use std::time::Duration;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyPlanMember {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub member_type: String,
    pub test_roster_id: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyPlanCharacter {
    pub char_id: i64,
    pub discord_id: String,
    pub roster_id: String,
    pub roster_name: String,
    pub char_name: String,
    pub class_id: String,
    pub icon_id: Option<String>,
    pub item_level: f64,
    pub combat_power: f64,
    pub included: bool,
    pub display_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyPlanRaid {
    pub raid_id: String,
    pub raid_name: String,
    pub min_ilvl: f64,
    pub max_ilvl: f64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyPlanAssignment {
    pub raid_id: String,
    pub assignment_type: String,
    pub target_id: String,
    pub slot_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyPlanCompletionSnapshot {
    pub discord_id: String,
    pub roster_id: String,
    pub char_id: i64,
    pub char_name: String,
    pub content_id: String,
    pub difficulty: Option<String>,
    pub is_completed: bool,
    pub session_id: Option<String>,
    pub completed_at: i64,
    pub reset_cycle: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyPlanEncounterSnapshot {
    pub discord_id: String,
    pub local_player: String,
    pub content_id: String,
    pub raid_name: String,
    pub difficulty: String,
    pub gate: Option<String>,
    pub cleared: bool,
    pub fight_start: i64,
    pub players: Vec<String>,
    pub matched_character_ids: Vec<i64>,
    pub reset_cycle: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyPlanData {
    pub group_id: String,
    #[serde(default)]
    pub group_secret: String,
    pub group_name: String,
    pub group_mode: Option<String>,
    pub owner_discord_id: Option<String>,
    pub sheet_url: String,
    pub sheet_version: i64,
    pub members: Vec<PartyPlanMember>,
    pub characters: Vec<PartyPlanCharacter>,
    pub planned_raids: Vec<PartyPlanRaid>,
    pub assignments: Vec<PartyPlanAssignment>,
    #[serde(default)]
    pub completion_snapshots: Vec<PartyPlanCompletionSnapshot>,
    #[serde(default)]
    pub encounter_snapshots: Vec<PartyPlanEncounterSnapshot>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyPlanRemoteSyncRequest {
    pub endpoint_url: String,
    pub action: String,
    pub group_id: String,
    pub group_secret: String,
    pub plan: Option<PartyPlanData>,
    #[serde(default)]
    pub merge_owner_ids: Vec<String>,
    #[serde(default)]
    pub member_discord_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyPlanRemoteStatus {
    pub group_id: String,
    pub updated_at: String,
    pub sheet_version: i64,
    pub group_config_updated_at: Option<String>,
    pub roster_updated_at: Option<String>,
    pub assignment_updated_at: Option<String>,
    pub snapshot_updated_at: Option<String>,
    pub related_members_updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyPlanMemberClear {
    pub group_id: String,
    pub group_name: String,
    pub group_mode: String,
    pub discord_id: String,
    pub char_id: i64,
    pub content_id: String,
    pub difficulty: Option<String>,
    pub session_id: Option<String>,
    pub gate: Option<String>,
    pub reset_cycle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyPlanStaticReservation {
    pub group_id: String,
    pub group_name: String,
    pub discord_id: String,
    pub char_id: i64,
    pub raid_id: String,
    pub raid_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyPlanRemoteSyncResponse {
    #[serde(default)]
    pub ok: bool,
    pub plan: Option<PartyPlanData>,
    pub status: Option<PartyPlanRemoteStatus>,
    #[serde(default)]
    pub member_clears: Vec<PartyPlanMemberClear>,
    #[serde(default)]
    pub static_reservations: Vec<PartyPlanStaticReservation>,
    #[serde(default)]
    pub message: String,
    pub error: Option<String>,
}

fn party_plans_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data directory: {}", e))?;

    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;

    Ok(app_data_dir.join("party_plans.json"))
}

fn read_party_plan_map(app: &AppHandle) -> Result<HashMap<String, PartyPlanData>, String> {
    let path = party_plans_path(app)?;
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read party plans file: {}", e))?;

    if content.trim().is_empty() {
        return Ok(HashMap::new());
    }

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse party plans file: {}", e))
}

fn write_party_plan_map(app: &AppHandle, plans: &HashMap<String, PartyPlanData>) -> Result<(), String> {
    let path = party_plans_path(app)?;
    let content = serde_json::to_string_pretty(plans)
        .map_err(|e| format!("Failed to serialize party plans: {}", e))?;

    fs::write(&path, content).map_err(|e| format!("Failed to write party plans file: {}", e))
}

#[tauri::command]
pub fn save_party_plan(app: AppHandle, mut plan: PartyPlanData) -> Result<PartyPlanData, String> {
    if plan.group_id.trim().is_empty() {
        return Err("Party plan group_id is required".to_string());
    }

    let mut plans = read_party_plan_map(&app)?;
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(existing_plan) = plans.get(&plan.group_id) {
        plan.created_at = existing_plan.created_at.clone();
    } else if plan.created_at.trim().is_empty() {
        plan.created_at = now.clone();
    }
    plan.updated_at = now;

    plans.insert(plan.group_id.clone(), plan.clone());
    write_party_plan_map(&app, &plans)?;

    Ok(plan)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn load_party_plan(app: AppHandle, groupId: String) -> Result<Option<PartyPlanData>, String> {
    let plans = read_party_plan_map(&app)?;
    Ok(plans.get(&groupId).cloned())
}

#[tauri::command]
pub fn list_party_plans(app: AppHandle) -> Result<Vec<PartyPlanData>, String> {
    let mut plans: Vec<PartyPlanData> = read_party_plan_map(&app)?.into_values().collect();
    plans.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(plans)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn delete_party_plan(app: AppHandle, groupId: String) -> Result<bool, String> {
    let mut plans = read_party_plan_map(&app)?;
    let removed = plans.remove(&groupId).is_some();
    write_party_plan_map(&app, &plans)?;
    Ok(removed)
}

#[tauri::command]
pub fn get_party_plan_endpoint_url() -> Result<Option<String>, String> {
    Ok(configured_value(
        "PARTY_PLAN_ENDPOINT_URL",
        option_env!("PARTY_PLAN_ENDPOINT_URL"),
    ))
}

#[tauri::command]
pub async fn sync_party_plan_remote(request: PartyPlanRemoteSyncRequest) -> Result<PartyPlanRemoteSyncResponse, String> {
    let endpoint_url = request.endpoint_url.trim();
    if endpoint_url.is_empty() {
        return Err("Party Plan remote sync endpoint is not configured".to_string());
    }
    if !endpoint_url.starts_with("https://") {
        return Err("Party Plan remote sync endpoint must use HTTPS".to_string());
    }
    if endpoint_url.contains("docs.google.com") || endpoint_url.contains("drive.google.com") {
        return Err("Party Plan remote sync endpoint must be the Apps Script web app URL, not a Google Sheet or Drive URL".to_string());
    }
    if request.group_id.trim().is_empty() || request.group_secret.trim().is_empty() {
        return Err("Party Plan remote sync requires group id and group secret".to_string());
    }
    if request.action != "load" && request.action != "status" && request.action != "loadMemberClears" && request.action != "loadStaticReservations" && request.action != "save" && request.action != "saveMerged" && request.action != "saveSnapshots" && request.action != "delete" {
        return Err("Party Plan remote sync action must be load, status, loadMemberClears, loadStaticReservations, save, saveMerged, saveSnapshots, or delete".to_string());
    }
    if (request.action == "save" || request.action == "saveMerged" || request.action == "saveSnapshots") && request.plan.is_none() {
        return Err("Party Plan remote save requires a plan payload".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| {
            let message = format!("Failed to create Party Plan sync client: {}", e);
            crate::log_error!("{}", message);
            message
        })?;

    let mut next_url = endpoint_url.to_string();
    let mut next_method_is_get = false;
    let mut response = None;

    for redirect_count in 0..=5 {
        let request_builder = if next_method_is_get {
            client.get(&next_url)
        } else {
            client.post(&next_url).json(&request)
        };

        let current_response = request_builder.send().await.map_err(|e| {
            let message = format!("Party Plan remote sync request failed: {}", e);
            crate::log_error!("{}", message);
            message
        })?;

        if current_response.status().is_redirection() {
            let status = current_response.status();
            let location = current_response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| {
                    let message = "Party Plan remote sync redirected without a Location header".to_string();
                    crate::log_error!("{}", message);
                    message
                })?;

            next_url = current_response
                .url()
                .join(location)
                .map_err(|e| {
                    let message = format!("Party Plan remote sync redirect URL is invalid: {}", e);
                    crate::log_error!("{}", message);
                    message
                })?
                .to_string();

            next_method_is_get = matches!(
                status,
                reqwest::StatusCode::MOVED_PERMANENTLY
                    | reqwest::StatusCode::FOUND
                    | reqwest::StatusCode::SEE_OTHER
            );

            if redirect_count == 5 {
                let message = "Party Plan remote sync redirected too many times".to_string();
                crate::log_error!("{}", message);
                return Err(message);
            }

            continue;
        }

        response = Some(current_response);
        break;
    }

    let response = response.ok_or_else(|| {
        let message = "Party Plan remote sync did not return a response".to_string();
        crate::log_error!("{}", message);
        message
    })?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| {
            let message = format!("Failed to read Party Plan remote sync response: {}", e);
            crate::log_error!("{}", message);
            message
        })?;

    if !status.is_success() {
        let message = format!(
            "Party Plan remote sync failed with HTTP {}: {}",
            status,
            truncate_response_body(&body)
        );
        crate::log_error!("{}", message);
        return Err(message);
    }

    let parsed: PartyPlanRemoteSyncResponse = serde_json::from_str(&body)
        .map_err(|e| {
            let message = format!(
                "Failed to parse Party Plan remote sync response: {} ({})",
                e,
                truncate_response_body(&body)
            );
            crate::log_error!("{}", message);
            message
        })?;

    if !parsed.ok {
        let message = parsed.error.clone().unwrap_or_else(|| parsed.message.clone());
        crate::log_error!("Party Plan remote sync rejected: {}", message);
        return Err(message);
    }

    Ok(parsed)
}

fn configured_value(key: &str, build_value: Option<&'static str>) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| build_value.map(str::to_string).filter(|value| !value.trim().is_empty()))
}

fn truncate_response_body(body: &str) -> String {
    const MAX_RESPONSE_BODY_LENGTH: usize = 800;
    if body.chars().count() <= MAX_RESPONSE_BODY_LENGTH {
        return body.to_string();
    }

    format!("{}...", body.chars().take(MAX_RESPONSE_BODY_LENGTH).collect::<String>())
}
