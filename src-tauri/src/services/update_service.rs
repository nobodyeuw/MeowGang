use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct LatestRelease {
    pub version: String,
    pub notes: String,
    pub pub_date: String,
    pub platforms: PlatformInfo,
}

#[derive(Debug, Deserialize)]
pub struct PlatformInfo {
    #[serde(rename = "windows-x86_64")]
    pub windows_x86_64: WindowsInfo,
}

#[derive(Debug, Deserialize)]
pub struct WindowsInfo {
    pub signature: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
}

pub struct UpdateService;

impl UpdateService {
    pub async fn check_for_updates(current_version: &str) -> Result<UpdateInfo> {
        use semver::Version;
        use std::time::Duration;
        
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        // Get latest release from custom endpoint
        let response = client.get("https://raw.githubusercontent.com/nobodyeuw/MeowGang-Tracker/main/latest.json")
            .header("User-Agent", "LOA Tracker")
            .send()
            .await?;

        let release: LatestRelease = response.json().await?;
        
        // Use proper semantic version comparison
        // Handle both "1.1.8" and "v1.1.8" formats
        let latest_version_str = release.version.trim_start_matches('v');
        let current_version_str = current_version.trim_start_matches('v');
        
        let update_available = match (
            Version::parse(current_version_str),
            Version::parse(latest_version_str),
        ) {
            (Ok(current), Ok(latest)) => latest > current,
            _ => {
                // Fallback to string comparison if semantic version parsing fails
                latest_version_str != current_version_str
            }
        };
        
        Ok(UpdateInfo {
            current_version: current_version.to_string(),
            latest_version: release.version,
            update_available,
            release_notes: Some(release.notes),
            download_url: Some(release.platforms.windows_x86_64.url),
        })
    }

    pub async fn download_and_install(download_url: String) -> Result<()> {
        const MAX_RETRIES: u32 = 3;
        let mut retry_count = 0;
        
        loop {
            match Self::try_download_and_install(&download_url).await {
                Ok(_) => return Ok(()),
                Err(e) if retry_count < MAX_RETRIES => {
                    crate::log_warn!("Download failed, retrying ({}/{}): {}", 
                        retry_count + 1, MAX_RETRIES, e);
                    retry_count += 1;
                    // Exponential backoff: 1s, 2s, 4s
                    let delay = std::time::Duration::from_secs(2_u64.pow(retry_count));
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn try_download_and_install(download_url: &str) -> Result<()> {
        use std::time::Duration;
        
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        
        // 1. Download installer (.exe)
        let response = client.get(download_url)
            .header("User-Agent", "tauri-app")
            .header("Accept", "application/octet-stream")
            .send()
            .await?;

        let bytes = response.bytes().await?;
        
        // Validate downloaded file size (min 1MB)
        if bytes.len() < 1024 * 1024 {
            return Err(anyhow::anyhow!("Downloaded file too small ({}MB), likely corrupted", bytes.len() / (1024 * 1024)));
        }
        
        // 2. Save to temporary directory with unique name
        let temp_dir = std::env::temp_dir();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let file_path = temp_dir.join(format!("LOA_Tracker_Updater_{}.exe", timestamp));
        
        // Remove existing file if it exists
        if file_path.exists() {
            std::fs::remove_file(&file_path)?;
        }
        
        let mut file = File::create(&file_path)?;
        file.write_all(&bytes)?;
        file.flush()?;
        drop(file);

        // 3. Small delay to ensure file is fully written
        std::thread::sleep(Duration::from_millis(500));
        
        // 4. Launch installer and exit app
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            match Command::new(&file_path)
                .arg("/S") // Silent install
                .creation_flags(0x08000000) // CREATE_NEW_PROCESS_GROUP
                .spawn() {
                Ok(_) => {
                    crate::log_info!("Installer launched successfully from: {}", file_path.display());
                    std::process::exit(0);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to launch installer: {}", e));
                }
            }
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            match Command::new(&file_path)
                .arg("/S")
                .spawn() {
                Ok(_) => {
                    crate::log_info!("Installer launched successfully");
                    std::process::exit(0);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to launch installer: {}", e));
                }
            }
        }
    }
}
