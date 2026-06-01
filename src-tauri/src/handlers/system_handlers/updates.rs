use std::{collections::HashMap, time::Duration};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

use crate::services::logging_service;

const UPDATE_METADATA_URL: &str = "https://raw.githubusercontent.com/nobodyeuw/MeowGang/main/latest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub body: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateMetadata {
    version: String,
    platforms: Option<HashMap<String, UpdatePlatformMetadata>>,
}

#[derive(Debug, Deserialize)]
struct UpdatePlatformMetadata {
    url: Option<String>,
}

enum UpdateReleaseDelay {
    Remaining(i64),
    Ready,
    Unknown,
}

/// Checks whether the latest.json assets are already reachable before showing an update.
async fn update_release_delay_status(version: &str) -> UpdateReleaseDelay {
    let client = match reqwest::Client::builder().timeout(Duration::from_secs(8)).build() {
        Ok(client) => client,
        Err(e) => {
            logging_service::warn(&format!("Failed to build update metadata client: {}", e));
            return UpdateReleaseDelay::Unknown;
        }
    };
    let metadata = match client
        .get(UPDATE_METADATA_URL)
        .header("User-Agent", "LOA Tracker")
        .send()
        .await
    {
        Ok(response) => match response.json::<UpdateMetadata>().await {
            Ok(metadata) => metadata,
            Err(e) => {
                logging_service::warn(&format!("Failed to parse update metadata for delay check: {}", e));
                return UpdateReleaseDelay::Unknown;
            }
        },
        Err(e) => {
            logging_service::warn(&format!("Failed to fetch update metadata for delay check: {}", e));
            return UpdateReleaseDelay::Unknown;
        }
    };

    if metadata.version.trim_start_matches('v') != version.trim_start_matches('v') {
        return UpdateReleaseDelay::Unknown;
    }

    if !update_asset_urls_ready(&client, &metadata).await {
        return UpdateReleaseDelay::Remaining(60);
    }

    UpdateReleaseDelay::Ready
}

/// Verifies updater asset URLs so users do not see a release before GitHub artifacts are available.
async fn update_asset_urls_ready(client: &reqwest::Client, metadata: &UpdateMetadata) -> bool {
    let Some(platforms) = metadata.platforms.as_ref() else {
        return true;
    };

    let mut urls: Vec<String> = Vec::new();
    for platform in platforms.values() {
        let Some(url) = platform.url.as_deref() else {
            continue;
        };
        if !urls.iter().any(|existing| existing == url) {
            urls.push(url.to_string());
        }
    }

    if urls.is_empty() {
        return true;
    }

    for url in urls {
        match client.head(&url).header("User-Agent", "LOA Tracker").send().await {
            Ok(response) if response.status().is_success() => {}
            Ok(response) => {
                logging_service::warn(&format!(
                    "Update asset is not reachable yet: {} returned {}",
                    url,
                    response.status()
                ));
                return false;
            }
            Err(e) => {
                logging_service::warn(&format!(
                    "Failed to verify update asset availability for {}: {}",
                    url, e
                ));
                return false;
            }
        }
    }

    true
}

/// Checks the configured Tauri updater endpoint and hides releases while artifacts settle.
#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateInfo, String> {
    logging_service::info("Checking for updates");

    let current_version = app.package_info().version.to_string();

    match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => {
                let remaining_delay = match update_release_delay_status(&update.version).await {
                    UpdateReleaseDelay::Remaining(seconds) => Some(seconds),
                    UpdateReleaseDelay::Ready => None,
                    UpdateReleaseDelay::Unknown => None,
                };

                if let Some(remaining_seconds) = remaining_delay {
                    logging_service::info(&format!(
                        "Update {} detected but hidden for {} more seconds while release artifacts settle",
                        update.version, remaining_seconds
                    ));
                    return Ok(UpdateInfo {
                        current_version,
                        latest_version: None,
                        update_available: false,
                        body: None,
                    });
                }

                logging_service::info(&format!("Update available: {}", update.version));
                Ok(UpdateInfo {
                    current_version,
                    latest_version: Some(update.version),
                    update_available: true,
                    body: update.body,
                })
            }
            Ok(None) => {
                logging_service::info("No update available");
                Ok(UpdateInfo {
                    current_version,
                    latest_version: None,
                    update_available: false,
                    body: None,
                })
            }
            Err(e) => {
                logging_service::error(&format!("Update check failed: {}", e));
                Err(e.to_string())
            }
        },
        Err(e) => {
            logging_service::error(&format!("Failed to get updater: {}", e));
            Err(e.to_string())
        }
    }
}

/// Downloads and installs the currently available Tauri updater release.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<String, String> {
    logging_service::info("Starting update installation");

    match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => {
                if let UpdateReleaseDelay::Remaining(remaining_seconds) =
                    update_release_delay_status(&update.version).await
                {
                    let remaining_minutes = ((remaining_seconds as f64) / 60.0).ceil() as i64;
                    logging_service::info(&format!(
                        "Install blocked because update {} is still in release grace period",
                        update.version
                    ));
                    return Err(format!(
                        "Update is still being prepared. Please try again in about {} minute{}.",
                        remaining_minutes,
                        if remaining_minutes == 1 { "" } else { "s" }
                    ));
                }

                logging_service::info(&format!("Downloading and installing update: {}", update.version));

                match update.download_and_install(|_, _| {}, || {}).await {
                    Ok(_) => {
                        logging_service::info("Update installed successfully");
                        Ok("Update installed successfully! The app will restart.".to_string())
                    }
                    Err(e) => {
                        logging_service::error(&format!("Update installation failed: {}", e));
                        Err(format!("Failed to install update: {}", e))
                    }
                }
            }
            Ok(None) => Err("No update available".to_string()),
            Err(e) => {
                logging_service::error(&format!("Failed to check for updates: {}", e));
                Err(format!("Failed to check for updates: {}", e))
            }
        },
        Err(e) => {
            logging_service::error(&format!("Failed to get updater: {}", e));
            Err(format!("Failed to get updater: {}", e))
        }
    }
}
