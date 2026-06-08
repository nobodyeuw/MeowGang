use std::{fs, path::PathBuf, time::Duration};

use tauri::Manager;

const LIVE_KNOWN_BUGS_URL: &str = "https://gist.githubusercontent.com/nobodyeuw/b974e79d199fdee371cc3db18151b22a/raw";

/// Reads bundled/resource JSON used by the Update tab.
///
/// Development, installed, and local-app-data layouts all place resources in
/// slightly different locations, so both changelogs and known bugs use this
/// shared lookup order.
fn read_resource_json(app: &tauri::AppHandle, filename: &str, label: &str) -> Result<serde_json::Value, String> {
    let exe_resource_path = {
        let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;
        let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
        exe_dir.join("../resources").join(filename)
    };

    let possible_paths: Vec<PathBuf> = vec![
        crate::app::resources_dir(app).join(filename),
        exe_resource_path,
        app.path()
            .resource_dir()
            .map_err(|e| format!("Failed to get resource directory: {}", e))?
            .join(filename),
        PathBuf::from("src-tauri/resources").join(filename),
    ];

    let path = possible_paths
        .iter()
        .find(|path| path.exists())
        .cloned()
        .ok_or_else(|| {
            format!(
                "{} file not found. Tried: {}",
                label,
                possible_paths
                    .iter()
                    .map(|path| format!("{:?}", path))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read {} file from {:?}: {}", label, path, e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse {} JSON: {}", label, e))
}

/// Fetches remote resource JSON with a short timeout.
///
/// Known bugs are allowed to be live-updated from a gist, but the app must keep
/// working offline and on invalid JSON, so callers should fall back to bundled
/// resources on any error.
async fn fetch_remote_resource_json(url: &str, label: &str) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| format!("Failed to build {} HTTP client: {}", label, e))?;

    let content = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch live {} JSON: {}", label, e))?
        .error_for_status()
        .map_err(|e| format!("Live {} JSON request failed: {}", label, e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read live {} JSON: {}", label, e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse live {} JSON: {}", label, e))
}

/// Loads the bundled changelog JSON shown in the Update tab.
#[tauri::command]
pub async fn get_changelogs(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    read_resource_json(&app, "changelogs.json", "Changelogs")
}

/// Loads the bundled known-bugs JSON shown in the Update tab.
#[tauri::command]
pub async fn get_known_bugs(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    match fetch_remote_resource_json(LIVE_KNOWN_BUGS_URL, "known bugs").await {
        Ok(remote_known_bugs) => return Ok(remote_known_bugs),
        Err(error) => {
            crate::log_warn!(
                "Failed to load live known bugs from gist; falling back to bundled resource: {}",
                error
            );
        }
    }

    read_resource_json(&app, "known_bugs.json", "Known bugs")
}
