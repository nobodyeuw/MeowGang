use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    time::{Duration, Instant},
};
use tauri::Manager;

const DEFAULT_AUTH_PORT: u16 = 53682;
const AUTH_TIMEOUT_SECONDS: u64 = 120;
const DISCORD_AUTHORIZE_URL: &str = "https://discord.com/oauth2/authorize";
const DISCORD_TOKEN_URL: &str = "https://discord.com/api/v10/oauth2/token";
const DISCORD_USER_URL: &str = "https://discord.com/api/v10/users/@me";
const DEFAULT_DISCORD_CLIENT_ID: &str = "1506247060142166076";
const LEGACY_AUTH_FILE_NAME: &str = "meowgang_auth.txt";
const LEGACY_UPDATE_FIRST_SEEN_FILE_NAME: &str = "update_first_seen.json";
const LEGACY_PARTY_PLANS_FILE_NAME: &str = "party_plans.json";

#[derive(Debug, Serialize)]
pub struct DiscordAuthResult {
    pub approved: bool,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct WhitelistMember {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct SupabaseOAuthCodeResult {
    pub code: String,
}

#[derive(Debug, Deserialize)]
struct DiscordCodePayload {
    code: String,
    state: String,
}

#[derive(Debug, Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    global_name: Option<String>,
}

#[tauri::command]
pub async fn authenticate_discord(app: tauri::AppHandle) -> Result<DiscordAuthResult, String> {
    let client_id = discord_client_id()?;
    let whitelist_url = discord_whitelist_url()?;
    let port = discord_redirect_port();
    let redirect_uri = format!("http://127.0.0.1:{}/discord/callback", port);
    let state = random_state();
    let code_verifier = pkce_code_verifier();
    let code_challenge = pkce_code_challenge(&code_verifier);
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("Failed to start Discord auth callback server on port {}: {}", port, e))?;

    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope=identify&state={}&code_challenge={}&code_challenge_method=S256&prompt=consent",
        DISCORD_AUTHORIZE_URL,
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&state),
        urlencoding::encode(&code_challenge),
    );

    open_external_url(&auth_url)?;

    let expected_state = state.clone();
    let code_payload = tokio::task::spawn_blocking(move || wait_for_discord_code(listener, &expected_state))
        .await
        .map_err(|e| format!("Discord auth task failed: {}", e))??;

    let access_token =
        exchange_discord_code_for_token(&client_id, &redirect_uri, &code_payload.code, &code_verifier).await?;

    verify_token_against_whitelist(&app, &access_token, &whitelist_url).await
}

#[tauri::command]
pub async fn authenticate_supabase_discord(authUrl: String) -> Result<SupabaseOAuthCodeResult, String> {
    if !authUrl.starts_with("https://") {
        return Err("Supabase auth URL must use HTTPS.".to_string());
    }

    let port = discord_redirect_port();
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("Failed to start Supabase auth callback server on port {}: {}", port, e))?;

    open_external_url(&authUrl)?;

    let code = tokio::task::spawn_blocking(move || wait_for_supabase_code(listener))
        .await
        .map_err(|e| format!("Supabase auth task failed: {}", e))??;

    Ok(SupabaseOAuthCodeResult { code })
}

#[tauri::command]
pub async fn verify_discord_profile_auth(
    _app: tauri::AppHandle,
    discordId: String,
    username: Option<String>,
) -> Result<DiscordAuthResult, String> {
    let whitelist_url = discord_whitelist_url()?;
    let whitelist = fetch_whitelist(&whitelist_url).await?;
    let discord_id = discordId.trim();

    if discord_id.is_empty() {
        return Ok(DiscordAuthResult {
            approved: false,
            user_id: None,
            username,
            message: "Discord auth did not include a Discord user id.".to_string(),
        });
    }

    if let Some(whitelist_name) = whitelist.get(discord_id) {
        let display_name = whitelist_name
            .clone()
            .or(username)
            .unwrap_or_else(|| "MeowGang member".to_string());
        Ok(DiscordAuthResult {
            approved: true,
            user_id: Some(discord_id.to_string()),
            username: Some(display_name.clone()),
            message: format!("Welcome, {}", display_name),
        })
    } else {
        Ok(DiscordAuthResult {
            approved: false,
            user_id: Some(discord_id.to_string()),
            username,
            message: "Not approved by our Meowtator".to_string(),
        })
    }
}

#[tauri::command]
pub async fn verify_stored_discord_auth(app: tauri::AppHandle) -> Result<DiscordAuthResult, String> {
    remove_stored_discord_id(&app);
    Ok(DiscordAuthResult {
        approved: false,
        user_id: None,
        username: None,
        message: "Sign in with Discord to access LOA Tracker.".to_string(),
    })
}

#[tauri::command]
pub async fn get_discord_whitelist_members() -> Result<Vec<WhitelistMember>, String> {
    let whitelist_url = discord_whitelist_url()?;
    let whitelist = fetch_whitelist(&whitelist_url).await?;
    let mut members: Vec<WhitelistMember> = whitelist
        .into_iter()
        .map(|(id, name)| WhitelistMember {
            name: name.unwrap_or_else(|| id.clone()),
            id,
        })
        .collect();

    members.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(members)
}

async fn verify_token_against_whitelist(
    _app: &tauri::AppHandle,
    access_token: &str,
    whitelist_url: &str,
) -> Result<DiscordAuthResult, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create auth client: {}", e))?;

    let user = client
        .get(DISCORD_USER_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to contact Discord: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Discord token is not valid: {}", e))?
        .json::<DiscordUser>()
        .await
        .map_err(|e| format!("Failed to parse Discord user: {}", e))?;

    let whitelist = fetch_whitelist(whitelist_url).await?;
    let display_name = user.global_name.clone().unwrap_or_else(|| user.username.clone());

    if let Some(whitelist_name) = whitelist.get(&user.id) {
        Ok(DiscordAuthResult {
            approved: true,
            user_id: Some(user.id),
            username: Some(whitelist_name.clone().unwrap_or(display_name)),
            message: format!(
                "Welcome, {}",
                whitelist_name.clone().unwrap_or_else(|| "MeowGang member".to_string())
            ),
        })
    } else {
        Ok(DiscordAuthResult {
            approved: false,
            user_id: Some(user.id),
            username: Some(display_name),
            message: "Not approved by our Meowtator".to_string(),
        })
    }
}

async fn fetch_whitelist(whitelist_url: &str) -> Result<HashMap<String, Option<String>>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create whitelist client: {}", e))?;

    let whitelist = client
        .get(whitelist_url)
        .header("User-Agent", "LOA Tracker")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Discord whitelist: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Discord whitelist request failed: {}", e))?
        .json::<Value>()
        .await
        .map_err(|e| format!("Failed to parse Discord whitelist JSON: {}", e))?;

    Ok(extract_whitelisted_users(&whitelist))
}

async fn exchange_discord_code_for_token(
    client_id: &str,
    redirect_uri: &str,
    code: &str,
    code_verifier: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create Discord token client: {}", e))?;

    let body = format!(
        "client_id={}&grant_type=authorization_code&code={}&redirect_uri={}&code_verifier={}",
        urlencoding::encode(client_id),
        urlencoding::encode(code),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(code_verifier),
    );

    let response = client
        .post(DISCORD_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Failed to exchange Discord auth code: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Discord auth code exchange failed: {}", e))?
        .json::<DiscordTokenResponse>()
        .await
        .map_err(|e| format!("Failed to parse Discord token response: {}", e))?;

    Ok(response.access_token)
}

fn wait_for_discord_code(listener: TcpListener, expected_state: &str) -> Result<DiscordCodePayload, String> {
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("Failed to configure Discord auth callback server: {}", e))?;

    let deadline = Instant::now() + Duration::from_secs(AUTH_TIMEOUT_SECONDS);

    while Instant::now() < deadline {
        match listener.accept() {
            Ok((mut stream, _)) => {
                if let Some(payload) = handle_auth_callback_request(&mut stream)? {
                    if payload.state != expected_state {
                        return Err("Discord auth state did not match. Please try again.".to_string());
                    }
                    return Ok(payload);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(format!("Discord auth callback failed: {}", e)),
        }
    }

    Err("Discord login timed out. Please try again.".to_string())
}

fn wait_for_supabase_code(listener: TcpListener) -> Result<String, String> {
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("Failed to configure Supabase auth callback server: {}", e))?;

    let deadline = Instant::now() + Duration::from_secs(AUTH_TIMEOUT_SECONDS);

    while Instant::now() < deadline {
        match listener.accept() {
            Ok((mut stream, _)) => {
                if let Some(code) = handle_supabase_callback_request(&mut stream)? {
                    return Ok(code);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(format!("Supabase auth callback failed: {}", e)),
        }
    }

    Err("Supabase login timed out. Please try again.".to_string())
}

fn handle_auth_callback_request(stream: &mut TcpStream) -> Result<Option<DiscordCodePayload>, String> {
    let request = read_http_request(stream)?;

    if request.starts_with("GET /discord/callback") {
        let payload = parse_discord_code_payload(&request)?;
        write_http_response(stream, "200 OK", "text/html; charset=utf-8", discord_callback_html())?;
        return Ok(Some(payload));
    }

    write_http_response(stream, "404 Not Found", "text/plain; charset=utf-8", "Not found")?;
    Ok(None)
}

fn handle_supabase_callback_request(stream: &mut TcpStream) -> Result<Option<String>, String> {
    let request = read_http_request(stream)?;

    if request.starts_with("GET /supabase/callback") {
        let code = parse_supabase_code_payload(&request)?;
        write_http_response(stream, "200 OK", "text/html; charset=utf-8", supabase_callback_html())?;
        return Ok(Some(code));
    }

    write_http_response(stream, "404 Not Found", "text/plain; charset=utf-8", "Not found")?;
    Ok(None)
}

fn parse_discord_code_payload(request: &str) -> Result<DiscordCodePayload, String> {
    let request_target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| "Discord callback request was malformed.".to_string())?;
    let query = request_target
        .split_once('?')
        .map(|(_, query)| query)
        .unwrap_or_default();
    let mut code = None;
    let mut state = None;

    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        let decoded_value = urlencoding::decode(value)
            .map_err(|e| format!("Failed to decode Discord callback value: {}", e))?
            .into_owned();

        match key {
            "code" => code = Some(decoded_value),
            "state" => state = Some(decoded_value),
            "error" => return Err(format!("Discord login failed: {}", decoded_value)),
            _ => {}
        }
    }

    Ok(DiscordCodePayload {
        code: code.ok_or_else(|| "Discord did not return an auth code.".to_string())?,
        state: state.ok_or_else(|| "Discord did not return an auth state.".to_string())?,
    })
}

fn parse_supabase_code_payload(request: &str) -> Result<String, String> {
    let request_target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| "Supabase callback request was malformed.".to_string())?;
    let query = request_target
        .split_once('?')
        .map(|(_, query)| query)
        .unwrap_or_default();

    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        let decoded_value = urlencoding::decode(value)
            .map_err(|e| format!("Failed to decode Supabase callback value: {}", e))?
            .into_owned();

        match key {
            "code" => return Ok(decoded_value),
            "error" => return Err(format!("Supabase login failed: {}", decoded_value)),
            _ => {}
        }
    }

    Err("Supabase did not return an auth code.".to_string())
}

fn read_http_request(stream: &mut TcpStream) -> Result<String, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("Failed to set callback read timeout: {}", e))?;

    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 2048];

    loop {
        let bytes_read = stream
            .read(&mut chunk)
            .map_err(|e| format!("Failed to read auth callback request: {}", e))?;
        if bytes_read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..bytes_read]);

        if let Some(header_end) = find_header_end(&buffer) {
            let header = String::from_utf8_lossy(&buffer[..header_end]).to_string();
            let content_length = parse_content_length(&header).unwrap_or(0);
            let body_bytes_read = buffer.len().saturating_sub(header_end + 4);

            while body_bytes_read < content_length && buffer.len() < header_end + 4 + content_length {
                let bytes_read = stream
                    .read(&mut chunk)
                    .map_err(|e| format!("Failed to read auth callback body: {}", e))?;
                if bytes_read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..bytes_read]);
            }
            break;
        }
    }

    String::from_utf8(buffer).map_err(|e| format!("Auth callback request was not valid UTF-8: {}", e))
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_content_length(header: &str) -> Option<usize> {
    header.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("content-length") {
            value.trim().parse::<usize>().ok()
        } else {
            None
        }
    })
}

fn write_http_response(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) -> Result<(), String> {
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        content_type,
        body.as_bytes().len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|e| format!("Failed to write auth callback response: {}", e))
}

fn discord_callback_html() -> &'static str {
    r#"<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>LOA Tracker Discord Login</title>
    <style>
      body { margin: 0; min-height: 100vh; display: grid; place-items: center; font-family: system-ui, sans-serif; background: #161618; color: #f4f0ea; }
      main { max-width: 420px; padding: 28px; text-align: center; }
      h1 { font-size: 22px; margin: 0 0 10px; }
      p { color: #c9c1b7; line-height: 1.5; }
    </style>
  </head>
  <body>
    <main>
      <h1>Discord login received</h1>
      <p>Login complete. You can close this tab and return to LOA Tracker.</p>
    </main>
  </body>
</html>"#
}

fn supabase_callback_html() -> &'static str {
    r#"<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>LOA Tracker MeowConnect Login</title>
    <style>
      body { margin: 0; min-height: 100vh; display: grid; place-items: center; font-family: system-ui, sans-serif; background: #161618; color: #f4f0ea; }
      main { max-width: 420px; padding: 28px; text-align: center; }
      h1 { font-size: 22px; margin: 0 0 10px; }
      p { color: #c9c1b7; line-height: 1.5; }
    </style>
  </head>
  <body>
    <main>
      <h1>MeowConnect login received</h1>
      <p>Login complete. You can close this tab and return to LOA Tracker.</p>
    </main>
  </body>
</html>"#
}

fn extract_whitelisted_users(value: &Value) -> HashMap<String, Option<String>> {
    let mut users = HashMap::new();

    match value {
        Value::Array(entries) => collect_users_from_array(entries, &mut users),
        Value::Object(map) => {
            for key in ["discord_ids", "allowed_discord_ids", "whitelist", "allowed", "users"] {
                if let Some(entry) = map.get(key) {
                    collect_users(entry, &mut users);
                }
            }
        }
        _ => {}
    }

    users
}

fn collect_users(value: &Value, users: &mut HashMap<String, Option<String>>) {
    match value {
        Value::String(id) => {
            users.insert(id.trim().to_string(), None);
        }
        Value::Array(entries) => collect_users_from_array(entries, users),
        Value::Object(map) => {
            let id = ["id", "discord_id", "discordId"]
                .iter()
                .find_map(|key| map.get(*key).and_then(Value::as_str));

            if let Some(id) = id {
                let name = ["name", "username", "display_name", "displayName"]
                    .iter()
                    .find_map(|key| map.get(*key).and_then(Value::as_str))
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_string);

                users.insert(id.trim().to_string(), name);
            }
        }
        _ => {}
    }
}

fn collect_users_from_array(entries: &[Value], users: &mut HashMap<String, Option<String>>) {
    for entry in entries {
        collect_users(entry, users);
    }
}

pub fn migrate_legacy_roaming_files(app: &tauri::AppHandle) {
    if let Err(e) = remove_legacy_auth_files(app) {
        crate::log_warn!("Failed to remove legacy Discord auth file: {}", e);
    }

    if let Some(path) = legacy_roaming_file_path(app, LEGACY_UPDATE_FIRST_SEEN_FILE_NAME) {
        if path.exists() {
            match fs::remove_file(&path) {
                Ok(_) => crate::log_info!("Removed legacy update delay state from {:?}", path),
                Err(e) => crate::log_warn!("Failed to remove legacy update delay state {:?}: {}", path, e),
            }
        }
    }

    if let Some(path) = legacy_roaming_file_path(app, LEGACY_PARTY_PLANS_FILE_NAME) {
        if path.exists() {
            match fs::remove_file(&path) {
                Ok(_) => crate::log_info!("Removed legacy party plans cache from {:?}", path),
                Err(e) => crate::log_warn!("Failed to remove legacy party plans cache {:?}: {}", path, e),
            }
        }
    }
}

fn legacy_roaming_file_path(app: &tauri::AppHandle, file_name: &str) -> Option<PathBuf> {
    app
        .path()
        .app_data_dir()
        .ok()
        .map(|path| path.join(file_name))
}

fn remove_legacy_auth_files(app: &tauri::AppHandle) -> Result<(), String> {
    let local_path = crate::app::data_dir(app).join(LEGACY_AUTH_FILE_NAME);
    if local_path.exists() {
        fs::remove_file(&local_path)
            .map_err(|e| format!("Failed to remove legacy Discord auth file {:?}: {}", local_path, e))?;
        crate::log_info!("Removed legacy Discord auth file from {:?}", local_path);
    }

    if let Some(legacy_path) = legacy_roaming_file_path(app, LEGACY_AUTH_FILE_NAME) {
        if legacy_path.exists() {
            fs::remove_file(&legacy_path)
                .map_err(|e| format!("Failed to remove legacy Discord auth file {:?}: {}", legacy_path, e))?;
            crate::log_info!("Removed legacy Discord auth file from {:?}", legacy_path);
        }
    }

    Ok(())
}

fn remove_stored_discord_id(app: &tauri::AppHandle) {
    let path = crate::app::data_dir(app).join(LEGACY_AUTH_FILE_NAME);
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    if let Some(path) = legacy_roaming_file_path(app, LEGACY_AUTH_FILE_NAME) {
        let _ = fs::remove_file(path);
    }
}

fn discord_client_id() -> Result<String, String> {
    Ok(configured_value("DISCORD_CLIENT_ID", option_env!("DISCORD_CLIENT_ID"))
        .unwrap_or_else(|| DEFAULT_DISCORD_CLIENT_ID.to_string()))
}

fn discord_whitelist_url() -> Result<String, String> {
    configured_value("DISCORD_WHITELIST_URL", option_env!("DISCORD_WHITELIST_URL"))
        .map(|url| normalize_gist_url(&url))
        .ok_or_else(|| "Discord auth is not configured: DISCORD_WHITELIST_URL is missing.".to_string())
}

fn discord_redirect_port() -> u16 {
    configured_value("DISCORD_REDIRECT_PORT", option_env!("DISCORD_REDIRECT_PORT"))
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_AUTH_PORT)
}

fn configured_value(key: &str, build_value: Option<&'static str>) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| build_value.map(str::to_string).filter(|value| !value.trim().is_empty()))
}

fn normalize_gist_url(url: &str) -> String {
    let trimmed_url = url.trim();

    if trimmed_url.contains("gist.githubusercontent.com") || !trimmed_url.contains("gist.github.com") {
        return trimmed_url.to_string();
    }

    let path = trimmed_url
        .trim_start_matches("https://gist.github.com/")
        .trim_start_matches("http://gist.github.com/")
        .split(['#', '?'])
        .next()
        .unwrap_or_default();
    let mut parts = path.split('/').filter(|part| !part.is_empty());

    match (parts.next(), parts.next()) {
        (Some(owner), Some(gist_id)) => {
            format!(
                "https://gist.githubusercontent.com/{}/{}/raw/whitelist.json",
                owner, gist_id
            )
        }
        _ => trimmed_url.to_string(),
    }
}

fn random_state() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

fn pkce_code_verifier() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

fn pkce_code_challenge(code_verifier: &str) -> String {
    let digest = Sha256::digest(code_verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

fn open_external_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .map_err(|e| format!("Failed to open Discord login in browser: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open Discord login in browser: {}", e))?;
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open Discord login in browser: {}", e))?;
    }

    Ok(())
}
