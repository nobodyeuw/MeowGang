use rand::Rng;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tokio::time::sleep;
use urlencoding;
use json5;
use super::item_mapping::{get_engraving_name, get_gem_name};

#[derive(Debug, Error)]
pub enum ScraperError {
    #[error("HTTP Error: {0}")]
    HttpError(u16),
    #[error("Private profile or not found")]
    PrivateProfile,
    #[error("No characters found for this roster")]
    NoCharactersFound,
    #[error("Could not find roster data in HTML")]
    RosterDataNotFound,
    #[error("Cloudflare protection blocked the request. Please try again in a few minutes.")]
    CloudflareBlocked,
    #[error("JSON parsing error: {0}")]
    JsonParsingError(#[from] serde_json::Error),
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Rate limited. Next request in: {0:?}")]
    RateLimited(Duration),
    #[error("Generic error: {0}")]
    Generic(String),
}

const MAX_RETRIES: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub char_id: i64,
    pub char_name: String,
    pub roster_id: String,
    pub roster_name: String,
    pub class_id: String,
    pub item_level: f64,
    pub combat_power: f64,
    pub display_order: i64,
    pub earns_gold: bool,
    pub hide_from_dashboard: bool,
    pub class_display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperData {
    pub roster_name: String,
    pub main_character: String,
    pub timestamp: String,
    pub source: String,
    pub characters: Vec<Character>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperResult {
    pub scraper_data: ScraperData,
    pub mapped_for_models: MappedData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappedData {
    pub roster: RosterData,
    pub character_tasks: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosterData {
    pub roster_id: String,
    pub roster_name: String,
    pub characters: Vec<Character>,
}

#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub next_request_time: Option<SystemTime>,
    pub time_until_next: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterEngraving {
    pub engraving_name: String,
    pub books_read: f64,
    pub max_books: f64,
    pub stone_bonus: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterEquipment {
    pub slot: String,
    pub enhancement_level: Option<f64>,
    pub tier: Option<String>,
    pub quality: Option<f64>,
    pub item_level: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterGem {
    pub slot_index: i64,
    pub gem_name: String,
    pub skill_name: String,
    pub gem_type: String,
    pub gem_level: i64,
    pub is_bound: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterDetailData {
    pub character_name: String,
    pub engravings: Vec<CharacterEngraving>,
    pub equipment: Vec<CharacterEquipment>,
    pub gems: Vec<CharacterGem>,
}

#[derive(Debug, Clone)]
pub enum ScraperStatus {
    Starting(String),
    Progress(String),
    Success(String),
    Error(String),
    RateLimited(RateLimitInfo),
    Finished,
}

#[derive(Debug, Clone)]
pub struct HumanizedScraper {
    main_character: String,
    roster_name: String,
    client: Client,
    last_request_time: Option<SystemTime>,
    user_agents: Vec<String>,
    referers: Vec<String>,
}

impl HumanizedScraper {
    pub fn new(main_character: String, roster_name: String) -> Self {
        crate::log_info!(
            "Starting new scraper-session for character '{}' in roster '{}'",
            main_character,
            roster_name
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .expect("Failed to create HTTP client");

        let user_agents = vec![
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/120.0",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:109.0) Gecko/20100101 Firefox/120.0",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/118.0.0.0 Safari/537.36",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edge/120.0.0.0",
        ].iter().map(|s| s.to_string()).collect();

        let referers = vec![
            "https://www.google.com/",
            "https://www.google.com/search?q=lostark.bible",
            "https://duckduckgo.com/?q=lostark.bible",
            "https://www.bing.com/search?q=lostark.bible",
            "https://www.reddit.com/r/lostarkgame/",
            "https://lost-ark.fandom.com/",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let scraper = Self {
            main_character,
            roster_name,
            client,
            last_request_time: None,
            user_agents,
            referers,
        };

        crate::log_debug!(
            "Scraper session successfully initialized with {} user agents and {} referers",
            scraper.user_agents.len(),
            scraper.referers.len()
        );
        scraper
    }

    pub fn set_last_request_time(&mut self, time: SystemTime) {
        self.last_request_time = Some(time);
    }

    pub fn get_last_request_time(&self) -> Option<SystemTime> {
        self.last_request_time.clone()
    }

    async fn humanized_delay(&self, min_seconds: f64, max_seconds: f64) {
        let delay = {
            let mut rng = rand::thread_rng();
            rng.gen_range(min_seconds..=max_seconds)
        };
        crate::log_debug!("Applying humanized delay: {:.2} seconds", delay);
        sleep(Duration::from_secs_f64(delay)).await;
    }

    pub fn can_make_request(&self) -> bool {
        if let Some(last_time) = self.last_request_time {
            let elapsed = SystemTime::now().duration_since(last_time).unwrap_or_default();
            elapsed >= Duration::from_secs(24 * 60 * 60) // 24 hours
        } else {
            true
        }
    }

    fn get_rate_limit_info(&self) -> RateLimitInfo {
        if let Some(last_time) = self.last_request_time {
            let next_request = last_time + Duration::from_secs(24 * 60 * 60);
            let now = SystemTime::now();

            if now < next_request {
                let time_until = next_request.duration_since(now).unwrap_or_default();
                RateLimitInfo {
                    next_request_time: Some(next_request),
                    time_until_next: Some(time_until),
                }
            } else {
                RateLimitInfo {
                    next_request_time: None,
                    time_until_next: None,
                }
            }
        } else {
            RateLimitInfo {
                next_request_time: None,
                time_until_next: None,
            }
        }
    }

    async fn setup_session(&self) -> Result<reqwest::RequestBuilder, Box<dyn std::error::Error + Send + Sync>> {
        crate::log_debug!("Setting up HTTP session for roster scraping");

        let (user_agent, referer) = {
            let mut rng = rand::thread_rng();
            let ua = self.user_agents[rng.gen_range(0..self.user_agents.len())].clone();
            let referrer = self.referers[rng.gen_range(0..self.referers.len())].clone();
            crate::log_debug!("Selected user agent and referer for request");
            (ua, referrer)
        };

        let encoded_character = urlencoding::encode(&self.main_character);
        let url = format!("https://lostark.bible/character/CE/{}/roster", encoded_character);
        crate::log_debug!("Constructed roster URL: {}", url);

        let request = self
            .client
            .get(&url)
            .header("User-Agent", user_agent)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
            )
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Accept-Encoding", "identity")
            .header("DNT", "1")
            .header("Connection", "keep-alive")
            .header("Upgrade-Insecure-Requests", "1")
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "cross-site")
            .header("Pragma", "no-cache")
            .header("Cache-Control", "no-cache")
            .header("Referer", referer);

        crate::log_debug!("HTTP request configured with headers");
        Ok(request)
    }

    fn normalize_class_name(&self, class_name: &str) -> String {
        class_name.to_lowercase().replace(" ", "_")
    }

    fn parse_item_level(&self, ilvl_str: &str) -> f64 {
        // Remove commas and parse as f64
        let cleaned = ilvl_str.replace(",", "");
        cleaned.parse().unwrap_or(0.0)
    }

    fn parse_roster_data(&self, html: &str) -> Result<Vec<Character>, ScraperError> {
        crate::log_debug!("Parsing HTML content, length: {}", html.len());
        crate::log_debug!("HTML starts with: {}", &html[..200.min(html.len())]);

        let patterns = vec![
            r"data[:=]\{roster:(\[[^\]]+\])",
            r"data\s*[:=]\s*\{roster:\s*(\[[^\]]+\])",
            r"window\.__NUXT__[^{]*roster[^{]*(\[[^\]]+\])",
        ];

        for (i, pattern) in patterns.iter().enumerate() {
            crate::log_debug!("Trying pattern {}: {}", i + 1, pattern);
            let regex = Regex::new(pattern)?;
            if let Some(captures) = regex.captures(html) {
                if let Some(roster_js) = captures.get(1) {
                    crate::log_info!("Found roster data with pattern {}", i + 1);
                    return self.parse_roster_json(roster_js.as_str());
                }
            }
        }

        crate::log_warn!("No roster data found in HTML");
        Err(ScraperError::RosterDataNotFound)
    }

    fn parse_roster_json(&self, roster_js: &str) -> Result<Vec<Character>, ScraperError> {
        crate::log_debug!("Raw JavaScript JSON: {}", roster_js);

        // Clean up JavaScript JSON to valid JSON using a Rust-compatible approach
        let mut cleaned = roster_js.to_string();

        // First, replace property names with quoted versions
        let re1 = Regex::new(r#"(\w+):"#)?;
        cleaned = re1
            .replace_all(&cleaned, |caps: &regex::Captures| format!("\"{}\":", &caps[1]))
            .to_string();

        crate::log_debug!("After property name quoting: {}", cleaned);

        // Then, handle unquoted string values more carefully
        // Only quote actual string values, not booleans, numbers, or null
        let re2 = Regex::new(r#":\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*([,}])"#)?;
        cleaned = re2
            .replace_all(&cleaned, |caps: &regex::Captures| {
                let value = &caps[1];
                let separator = &caps[2];

                // Don't quote boolean values, numbers, or null
                if value == "true" || value == "false" || value == "null" || value.parse::<f64>().is_ok() {
                    format!(":{}{}", value, separator)
                } else {
                    format!(":\"{}\"{}", value, separator)
                }
            })
            .to_string();

        // Normalize shorthand decimals: .59 -> 0.59 (valid JS but invalid JSON)
        let re3 = Regex::new(r"(?P<pre>[:\[,])(?P<dot>\.\d)")?;
        cleaned = re3
            .replace_all(&cleaned, |caps: &regex::Captures| {
                format!("{}0{}", &caps["pre"], &caps["dot"])
            })
            .to_string();

        crate::log_debug!("After cleanup: {}", cleaned);

        let parsed: Vec<serde_json::Value> = serde_json::from_str(&cleaned)?;
        let roster_id = self.generate_roster_id();

        let mut characters = Vec::new();
        for (i, char) in parsed.iter().enumerate() {
            // Extract ID as number
            let char_id = char.get("id").and_then(|v| v.as_i64()).unwrap_or(0);

            if let (Some(name), Some(class)) = (
                char.get("name").and_then(|v| v.as_str()),
                char.get("class").and_then(|v| v.as_str()),
            ) {
                // Extract item level as number or string
                let item_level = char
                    .get("ilvl")
                    .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
                    .unwrap_or(0.0);

                // Extract combat power from nested object
                let combat_power = char
                    .get("combatPower")
                    .and_then(|cp| cp.get("score"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                characters.push(Character {
                    char_id,
                    char_name: name.to_string(),
                    roster_id: roster_id.clone(),
                    roster_name: self.roster_name.clone(),
                    class_id: self.normalize_class_name(class),
                    item_level,
                    combat_power,
                    display_order: (i + 1) as i64,
                    earns_gold: false,
                    hide_from_dashboard: false,
                    class_display_name: None,
                });
            }
        }

        crate::log_debug!("Successfully parsed {} characters", characters.len());
        Ok(characters)
    }

    fn generate_roster_id(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.roster_name.hash(&mut hasher);
        format!("roster_{}", hasher.finish())
    }

    fn map_to_service_layer_format(&self, characters: Vec<Character>) -> ScraperResult {
        let scraper_data = ScraperData {
            roster_name: self.roster_name.clone(),
            main_character: self.main_character.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            source: "lostark.bible".to_string(),
            characters: characters.clone(),
        };

        let mapped_for_models = MappedData {
            roster: RosterData {
                roster_id: self.roster_name.clone(),
                roster_name: self.roster_name.clone(),
                characters,
            },
            character_tasks: Vec::new(),
        };

        ScraperResult {
            scraper_data,
            mapped_for_models,
        }
    }

    fn is_cloudflare_challenge(status: u16, content: &str) -> bool {
        if status == 403 || status == 503 {
            return true;
        }
        content.contains("Just a moment...")
            || content.contains("challenges.cloudflare.com")
            || content.contains("cf-browser-verification")
            || content.contains("cf_clearance")
    }

    async fn attempt_scrape(&self) -> Result<String, ScraperError> {
        let request = self
            .setup_session()
            .await
            .map_err(|e| ScraperError::Generic(e.to_string()))?;

        let response = request.send().await?;
        let status = response.status().as_u16();

        if status == 404 {
            return Err(ScraperError::PrivateProfile);
        }

        let content = response.text().await?;

        if Self::is_cloudflare_challenge(status, &content) {
            crate::log_warn!(
                "Cloudflare challenge detected (HTTP {}) for '{}'",
                status,
                self.main_character
            );
            return Err(ScraperError::CloudflareBlocked);
        }

        if status != 200 {
            return Err(ScraperError::HttpError(status));
        }

        Ok(content)
    }

    pub async fn scrape_roster(&mut self) -> Result<ScraperResult, ScraperError> {
        // Check rate limiting
        if !self.can_make_request() {
            let rate_info = self.get_rate_limit_info();
            return Err(ScraperError::RateLimited(rate_info.time_until_next.unwrap_or_default()));
        }

        // Retry loop with exponential backoff for Cloudflare blocks
        let mut last_error = ScraperError::Generic("No attempts made".to_string());
        for attempt in 1..=MAX_RETRIES {
            crate::log_info!(
                "Scrape attempt {}/{} for '{}'",
                attempt,
                MAX_RETRIES,
                self.main_character
            );

            // Humanized delay before request (longer on retries)
            let base_delay = if attempt == 1 { 1.0 } else { 3.0 * attempt as f64 };
            self.humanized_delay(base_delay, base_delay + 3.0).await;

            match self.attempt_scrape().await {
                Ok(content) => {
                    crate::log_debug!("Received HTML response, length: {}", content.len());

                    // Check for private profile indicators
                    let is_private = content.contains("This profile is private")
                        || content.contains("Profile not found")
                        || (content.contains("Private") && content.contains("profile"));

                    if is_private {
                        crate::log_warn!("Private profile detected for character '{}'", self.main_character);
                        return Err(ScraperError::PrivateProfile);
                    }

                    // Brief processing delay
                    self.humanized_delay(0.5, 1.5).await;

                    let characters = self.parse_roster_data(&content)?;

                    if characters.is_empty() {
                        crate::log_warn!("No characters found in roster for '{}'", self.main_character);
                        return Err(ScraperError::NoCharactersFound);
                    }

                    crate::log_info!(
                        "Successfully parsed {} characters from roster '{}'",
                        characters.len(),
                        self.roster_name
                    );

                    // Update last request time
                    self.last_request_time = Some(SystemTime::now());

                    return Ok(self.map_to_service_layer_format(characters));
                }
                Err(ScraperError::CloudflareBlocked) if attempt < MAX_RETRIES => {
                    crate::log_warn!(
                        "Cloudflare blocked attempt {}/{}, retrying after delay...",
                        attempt,
                        MAX_RETRIES
                    );
                    last_error = ScraperError::CloudflareBlocked;
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Err(last_error)
    }

    async fn setup_character_session(&self, character_name: &str) -> Result<reqwest::RequestBuilder, Box<dyn std::error::Error + Send + Sync>> {
        crate::log_debug!("Setting up HTTP session for character detail scraping");

        let (user_agent, referer) = {
            let mut rng = rand::thread_rng();
            let ua = self.user_agents[rng.gen_range(0..self.user_agents.len())].clone();
            let referrer = self.referers[rng.gen_range(0..self.referers.len())].clone();
            crate::log_debug!("Selected user agent and referer for character request");
            (ua, referrer)
        };

        let encoded_character = urlencoding::encode(character_name);
        let url = format!("https://lostark.bible/character/CE/{}", encoded_character);
        crate::log_debug!("Constructed character detail URL: {}", url);

        let request = self
            .client
            .get(&url)
            .header("User-Agent", user_agent)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
            )
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Accept-Encoding", "identity")
            .header("DNT", "1")
            .header("Connection", "keep-alive")
            .header("Upgrade-Insecure-Requests", "1")
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "cross-site")
            .header("Pragma", "no-cache")
            .header("Cache-Control", "no-cache")
            .header("Referer", referer);

        crate::log_debug!("HTTP request configured with headers");
        Ok(request)
    }

    fn parse_character_loadouts(&self, html: &str) -> Result<Vec<serde_json::Value>, ScraperError> {
        crate::log_debug!("Parsing character loadouts from HTML, length: {}", html.len());

        // Find the loadouts data by looking for the array that contains loadout objects
        // Loadout objects have a "classification" field
        let loadouts_start = html.find("loadouts:[")
            .ok_or_else(|| ScraperError::RosterDataNotFound)?;

        // Extract from the loadouts:[ position onwards
        let html_slice = &html[loadouts_start..];
        
        // Find the opening bracket after loadouts:[
        let bracket_pos = html_slice.find('[')
            .ok_or_else(|| ScraperError::RosterDataNotFound)?;
        
        let array_start = loadouts_start + bracket_pos;
        
        // Use bracket counting to extract the entire array
        let loadouts_js = self.extract_array_by_bracket_counting_from_position(html, array_start)
            .ok_or_else(|| ScraperError::RosterDataNotFound)?;

        crate::log_info!("Found loadouts data using bracket counting");
        crate::log_debug!("Extracted loadouts length: {}", loadouts_js.len());
        crate::log_debug!("Extracted loadouts (first 1000 chars): {}", &loadouts_js.chars().take(1000).collect::<String>());
        crate::log_debug!("Extracted loadouts (last 1000 chars): {}", &loadouts_js.chars().rev().take(1000).collect::<String>());

        self.parse_loadouts_json(&loadouts_js)
    }

    fn extract_array_by_bracket_counting_from_position(&self, html: &str, start_pos: usize) -> Option<String> {
        crate::log_debug!("Extracting array from position: {}", start_pos);
        
        // Count brackets to find the matching closing bracket
        let mut bracket_count = 0;
        let mut in_string = false;
        let mut escape_next = false;
        
        for (i, c) in html[start_pos..].char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }
            
            match c {
                '\\' => escape_next = true,
                '"' => in_string = !in_string,
                '[' if !in_string => bracket_count += 1,
                ']' if !in_string => {
                    bracket_count -= 1;
                    if bracket_count == 0 {
                        // Found the matching closing bracket
                        let array_end = start_pos + i + 1;
                        crate::log_debug!("Found closing bracket at position: {}", array_end);
                        crate::log_debug!("Extracted array length: {}", array_end - start_pos);
                        return Some(html[start_pos..array_end].to_string());
                    }
                }
                _ => {}
            }
        }
        
        crate::log_debug!("Failed to find matching closing bracket, final bracket_count: {}", bracket_count);
        None
    }

    fn extract_array_by_bracket_counting(&self, html: &str, key: &str) -> Option<String> {
        // Find the key (with or without colon)
        let key_with_colon = format!("{}:", key);
        let key_without_colon = key;
        
        crate::log_debug!("Looking for key pattern: {}", key_with_colon);
        
        // Try with colon first
        let (start_pos, key_len) = if let Some(pos) = html.find(&key_with_colon) {
            crate::log_debug!("Found key pattern with colon at position: {}", pos);
            (pos, key_with_colon.len())
        } else if let Some(pos) = html.find(key_without_colon) {
            crate::log_debug!("Found key pattern without colon at position: {}", pos);
            (pos, key_without_colon.len())
        } else {
            crate::log_debug!("Could not find key pattern in HTML");
            return None;
        };
        
        // Find the opening bracket after the key
        let after_key = &html[start_pos + key_len..];
        if let Some(bracket_pos) = after_key.find('[') {
            let array_start = start_pos + key_len + bracket_pos;
            crate::log_debug!("Found opening bracket at position: {}", array_start);
            
            // Count brackets to find the matching closing bracket
            let mut bracket_count = 0;
            let mut in_string = false;
            let mut escape_next = false;
            
            for (i, c) in html[array_start..].char_indices() {
                if escape_next {
                    escape_next = false;
                    continue;
                }
                
                match c {
                    '\\' => escape_next = true,
                    '"' => in_string = !in_string,
                    '[' if !in_string => bracket_count += 1,
                    ']' if !in_string => {
                        bracket_count -= 1;
                        if bracket_count == 0 {
                            // Found the matching closing bracket
                            let array_end = array_start + i + 1;
                            crate::log_debug!("Found closing bracket at position: {}", array_end);
                            crate::log_debug!("Extracted array length: {}", array_end - array_start);
                            return Some(html[array_start..array_end].to_string());
                        }
                    }
                    _ => {}
                }
            }
            
            crate::log_debug!("Failed to find matching closing bracket, final bracket_count: {}", bracket_count);
        } else {
            crate::log_debug!("Could not find opening bracket after key");
        }
        None
    }

    fn parse_loadouts_json(&self, loadouts_js: &str) -> Result<Vec<serde_json::Value>, ScraperError> {
        crate::log_debug!("Raw loadouts JSON length: {}", loadouts_js.len());

        // lostark.bible embeds JavaScript literals that are invalid in JSON5:
        //   `void 0` and `void(0)` mean `undefined` in JS — replace with `null`.
        let sanitized = loadouts_js
            .replace("void 0", "null")
            .replace("void(0)", "null");

        // Use json5 to parse JavaScript-style JSON directly.
        // json5 handles unquoted property names, trailing commas, etc.
        let parsed: Vec<serde_json::Value> = json5::from_str(&sanitized)
            .map_err(|e| {
                crate::log_error!("JSON5 parsing error: {}", e);
                crate::log_error!("Sanitized JSON (first 500 chars): {}", &sanitized.chars().take(500).collect::<String>());
                ScraperError::Generic(format!("JSON5 parsing error: {}", e))
            })?;

        crate::log_debug!("Successfully parsed {} loadouts using json5", parsed.len());
        Ok(parsed)
    }

    fn extract_engravings_from_html(&self, html: &str) -> Vec<CharacterEngraving> {
        let mut engravings = Vec::new();
        
        // Try to find engraving data in the HTML
        // Look for patterns like engraving names and levels
        // This is a simplified approach - in production, you'd want more robust parsing
        
        // For now, return empty since we don't have the exact HTML structure
        // The actual implementation would need to inspect the HTML structure
        crate::log_debug!("Attempting to extract engravings from HTML");
        
        // Try to find engraving data in HTML structure
        
        // Pattern 1: Look for engraving data in script tags or JSON data
        if let Some(script_start) = html.find("engravings") {
            let section = &html[script_start..];
            // Try to find patterns like: {"name":"Adrenaline","points":20,"max":20,"stoneBonus":1}
            let json_pattern = Regex::new(r#"\{[^}]*"name"\s*:\s*"([^"]+)"[^}]*"points"\s*:\s*(\d+)[^}]*"max"\s*:\s*(\d+)[^}]*"stoneBonus"\s*:\s*(\d+)[^}]*\}"#).unwrap_or_else(|_| {
                Regex::new(r#"\{[^}]*"name"\s*:\s*"([^"]+)"[^}]*\}"#).unwrap()
            });
            
            for cap in json_pattern.captures_iter(section) {
                if let (Some(name), Some(points)) = (cap.get(1), cap.get(2)) {
                    let engraving_name = name.as_str().trim().to_string();
                    let books_read: f64 = points.as_str().parse().unwrap_or(0.0);
                    let max_books: f64 = cap.get(3).map(|m| m.as_str().parse().unwrap_or(20.0)).unwrap_or(20.0);
                    let stone_bonus: f64 = cap.get(4).map(|m| m.as_str().parse().unwrap_or(0.0)).unwrap_or(0.0);
                    
                    engravings.push(CharacterEngraving {
                        engraving_name,
                        books_read,
                        max_books,
                        stone_bonus,
                    });
                }
            }
        }
        
        // Pattern 2: Look for text-based patterns in the HTML
        // Search for common engraving names followed by numbers
        let engraving_pattern = Regex::new(r"(\d+)/(\d+)\s*\+*(\d*)").unwrap_or_else(|_| {
            Regex::new(r"(\d+)/(\d+)").unwrap()
        });
        
        let common_engravings = vec![
            "Adrenaline", "Grudge", "Keen Blunt Weapon", "Raid Captain", "Cursed Doll",
            "Spirit Absorption", "Hit Master", "Awakening", "Heavy Armor", "Drops of Ether"
        ];
        
        for engraving_name in &common_engravings {
            if let Some(pos) = html.find(engraving_name) {
                let after_name = &html[pos + engraving_name.len()..pos + engraving_name.len() + 50];
                if let Some(cap) = engraving_pattern.captures(after_name) {
                    let books_read: f64 = cap.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
                    let max_books: f64 = cap.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(20.0);
                    let stone_bonus: f64 = cap.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
                    
                    engravings.push(CharacterEngraving {
                        engraving_name: engraving_name.to_string(),
                        books_read,
                        max_books,
                        stone_bonus,
                    });
                }
            }
        }
        
        crate::log_debug!("Extracted {} engravings from HTML", engravings.len());
        engravings
    }

    fn extract_equipment_from_html(&self, _html: &str) -> Vec<CharacterEquipment> {
        // Superseded by extract_equipment_from_loadout — kept as a no-op stub.
        crate::log_debug!("extract_equipment_from_html: no-op stub, use loadout extractor");
        Vec::new()
    }

    fn extract_gems_from_html(&self, _html: &str) -> Vec<CharacterGem> {
        // Superseded by extract_gems_from_loadout — kept as a no-op stub.
        crate::log_debug!("extract_gems_from_html: no-op stub, use loadout extractor");
        Vec::new()
    }

    fn extract_engravings_from_loadout(&self, loadout: &serde_json::Value) -> Vec<CharacterEngraving> {
        let mut engravings = Vec::new();

        // Stone bonus from the ability_stone item:
        // items[slot=="ability_stone"].data.engravings[].{ id, nodes }
        let mut stone_bonus_map: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
        if let Some(items) = loadout.get("items").and_then(|i| i.as_array()) {
            for item in items {
                if item.get("slot").and_then(|s| s.as_str()) == Some("ability_stone") {
                    if let Some(stone_engravings) = item
                        .get("data")
                        .and_then(|d| d.get("engravings"))
                        .and_then(|e| e.as_array())
                    {
                        for se in stone_engravings {
                            if let (Some(id), Some(nodes)) = (
                                se.get("id").and_then(|i| i.as_i64()),
                                se.get("nodes").and_then(|n| n.as_i64()),
                            ) {
                                stone_bonus_map.insert(id, nodes);
                            }
                        }
                    }
                    break;
                }
            }
        }
        crate::log_debug!("Stone bonus map from ability_stone: {:?}", stone_bonus_map);

        // engravings[]: { grade: "engrave_grade04"|"engrave_grade05", id, progress }
        // progress = books read (0-20)
        if let Some(engraving_data) = loadout.get("engravings").and_then(|e| e.as_array()) {
            for engraving in engraving_data {
                let id = match engraving.get("id").and_then(|i| i.as_i64()) {
                    Some(v) => v,
                    None => continue,
                };
                // Skip penalty engravings
                if (id >= 800 && id <= 803) || (id >= 1800 && id <= 1803) {
                    continue;
                }
                let books_read = engraving.get("progress").and_then(|p| p.as_i64()).unwrap_or(0);
                let stone_bonus = *stone_bonus_map.get(&id).unwrap_or(&0);
                let engraving_name = get_engraving_name(id)
                    .unwrap_or_else(|| { crate::log_debug!("Unknown engraving id: {}", id); "Unknown Engraving" })
                    .to_string();
                engravings.push(CharacterEngraving {
                    engraving_name,
                    books_read: books_read as f64,
                    max_books: 20.0,
                    stone_bonus: stone_bonus as f64,
                });
            }
        }

        if engravings.is_empty() {
            crate::log_debug!("No engravings found in loadout");
        } else {
            crate::log_debug!("Extracted {} engravings", engravings.len());
        }
        engravings
    }

    fn extract_equipment_from_loadout(&self, loadout: &serde_json::Value) -> Vec<CharacterEquipment> {
        let mut equipment = Vec::new();

        let slot_map: std::collections::HashMap<&str, &str> = [
            ("weapon",        "weapon"),
            ("head",          "head"),
            ("upper_body",    "chest"),
            ("lower_body",    "pants"),
            ("hand",          "gloves"),
            ("shoulder",      "shoulder"),
            ("neck",          "neck"),
            ("ear1",          "earring1"),
            ("ear2",          "earring2"),
            ("finger1",       "ring1"),
            ("finger2",       "ring2"),
            ("bracelet",      "bracelet"),
            ("ability_stone", "ability_stone"),
        ].iter().cloned().collect();

        let armor_slots = ["weapon", "head", "upper_body", "lower_body", "hand", "shoulder"];

        if let Some(items_data) = loadout.get("items").and_then(|e| e.as_array()) {
            for item in items_data {
                let raw_slot = match item.get("slot").and_then(|s| s.as_str()) {
                    Some(s) => s,
                    None => continue,
                };
                let slot = match slot_map.get(raw_slot) {
                    Some(s) => s.to_string(),
                    None => continue,
                };
                let data = item.get("data");
                let enhancement_level = data
                    .and_then(|d| d.get("honing"))
                    .and_then(|h| h.as_i64().or_else(|| h.as_f64().map(|f| f as i64)));
                // Quality: stats array entry with type=57, index=1
                let quality = data
                    .and_then(|d| d.get("stats")).and_then(|s| s.as_array())
                    .and_then(|stats| stats.iter().find(|s| {
                        s.get("type").and_then(|t| t.as_i64()) == Some(57)
                        && s.get("index").and_then(|i| i.as_i64()) == Some(1)
                    }))
                    .and_then(|s| s.get("value").and_then(|v| v.as_i64()));
                let item_level = if armor_slots.contains(&raw_slot) {
                    loadout.get("itemLevel").and_then(|v| v.as_f64())
                } else { None };
                let tier = item_level.map(|ilvl| {
                    if ilvl >= 1600.0 { "T4".to_string() } else { "T3".to_string() }
                }).or_else(|| {
                    data.and_then(|d| d.get("type")).and_then(|t| t.as_str()).map(|t| {
                        if t.contains("tier4") { "T4".to_string() } else { "T3".to_string() }
                    })
                });
                equipment.push(CharacterEquipment {
                    slot,
                    enhancement_level: enhancement_level.map(|v| v as f64),
                    tier,
                    quality: quality.map(|v| v as f64),
                    item_level,
                });
            }
        }

        if equipment.is_empty() {
            crate::log_debug!("No equipment found in loadout");
        } else {
            crate::log_debug!("Extracted {} equipment pieces", equipment.len());
        }
        equipment
    }

    fn extract_gems_from_loadout(&self, loadout: &serde_json::Value) -> Vec<CharacterGem> {
        let mut gems = Vec::new();

        // gem: { id: <item_id>, slot: <0-based>, effects: [ { type, id: <skill_id>, value } ] }
        // Level encoded in item_id last 3 digits / 10: 65031080 -> 080/10 = 8
        // Type: 5 or 34 = attack, 27 = cooldown
        if let Some(gems_data) = loadout.get("gems").and_then(|g| g.as_array()) {
            for gem in gems_data {
                let gem_item_id = match gem.get("id").and_then(|i| i.as_i64()) {
                    Some(v) => v,
                    None => continue,
                };
                let slot_index = gem.get("slot").and_then(|s| s.as_i64()).unwrap_or(0);
                let last3 = gem_item_id % 1000;
                let gem_level = { let lv = last3 / 10; if lv == 0 { 10 } else { lv } };
                let gem_name = get_gem_name(gem_item_id)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("Gem (id {})", gem_item_id));
                let is_bound = gem.get("bound").and_then(|b| b.as_bool()).unwrap_or(false);
                let effects = gem.get("effects").and_then(|e| e.as_array());
                let primary = effects.and_then(|effs| effs.iter().find(|e| {
                    e.get("type").and_then(|t| t.as_i64()).map(|t| t != 2).unwrap_or(false)
                }));
                let gem_type = primary
                    .and_then(|e| e.get("type").and_then(|t| t.as_i64()))
                    .map(|t| match t { 5 | 34 => "attack".to_string(), 27 => "cooldown".to_string(), o => format!("type_{}", o) })
                    .unwrap_or_else(|| "unknown".to_string());
                let skill_name = primary
                    .and_then(|e| e.get("id").and_then(|i| i.as_i64()))
                    .and_then(|sid| get_gem_name(sid))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| gem_name.clone());
                gems.push(CharacterGem { slot_index, gem_name, skill_name, gem_type, gem_level, is_bound });
            }
        }

        if gems.is_empty() {
            crate::log_debug!("No gems found in loadout");
        } else {
            crate::log_debug!("Extracted {} gems", gems.len());
        }
        gems
    }

    pub async fn scrape_character_details(&mut self, character_name: String) -> Result<CharacterDetailData, ScraperError> {
        crate::log_info!("Starting character detail scrape for '{}'", character_name);

        // Humanized delay before request
        self.humanized_delay(1.0, 3.0).await;

        let request = self
            .setup_character_session(&character_name)
            .await
            .map_err(|e| ScraperError::Generic(e.to_string()))?;

        let response = request.send().await
            .map_err(|e| ScraperError::Generic(format!("Request error: {}", e)))?;
        let status = response.status().as_u16();

        if status == 404 {
            return Err(ScraperError::PrivateProfile);
        }

        let content = response.text().await
            .map_err(|e| ScraperError::Generic(format!("Failed to read response body: {}", e)))?;

        if Self::is_cloudflare_challenge(status, &content) {
            crate::log_warn!(
                "Cloudflare challenge detected (HTTP {}) for character '{}'",
                status,
                character_name
            );
            return Err(ScraperError::CloudflareBlocked);
        }

        if status != 200 {
            return Err(ScraperError::HttpError(status));
        }

        // Parse loadouts from HTML
        let loadouts = self.parse_character_loadouts(&content)?;

        if loadouts.is_empty() {
            crate::log_warn!("No loadouts found for character '{}'", character_name);
            return Err(ScraperError::NoCharactersFound);
        }

        // Use pick_preferred_raid_loadout to select the best loadout
        let preferred_loadout = crate::roster::pick_preferred_raid_loadout(&loadouts)
            .ok_or_else(|| ScraperError::Generic("No suitable raid loadout found".to_string()))?;

        crate::log_info!("Selected loadout with classification: {:?}", 
            preferred_loadout.get("classification").and_then(|c| c.as_str()));

        // Debug: log the structure of engravings and gems fields
        if let Some(engravings_val) = preferred_loadout.get("engravings") {
            crate::log_debug!("Engravings field value: {:?}", engravings_val);
        }
        if let Some(gems_val) = preferred_loadout.get("gems") {
            crate::log_debug!("Gems field value: {:?}", gems_val);
        }

        // Extract data from the selected loadout
        let engravings = self.extract_engravings_from_loadout(preferred_loadout);
        let equipment = self.extract_equipment_from_loadout(preferred_loadout);
        let gems = self.extract_gems_from_loadout(preferred_loadout);

        crate::log_info!(
            "Extracted {} engravings, {} equipment pieces, {} gems for character '{}'",
            engravings.len(),
            equipment.len(),
            gems.len(),
            character_name
        );

        Ok(CharacterDetailData {
            character_name,
            engravings,
            equipment,
            gems,
        })
    }
}

pub struct ScraperManager {
    scrapers: HashMap<String, HumanizedScraper>,
}

impl ScraperManager {
    pub fn new() -> Self {
        crate::log_info!("Creating new scraper manager");
        Self {
            scrapers: HashMap::new(),
        }
    }

    pub fn create_scraper(&mut self, main_character: String, roster_name: String) -> &mut HumanizedScraper {
        crate::log_info!(
            "Creating scraper for character '{}' in roster '{}'",
            main_character,
            roster_name
        );
        let scraper = HumanizedScraper::new(main_character, roster_name.clone());
        self.scrapers.insert(roster_name.clone(), scraper);
        self.scrapers.get_mut(&roster_name).unwrap()
    }

    pub fn get_scraper(&mut self, roster_name: &str) -> Option<&mut HumanizedScraper> {
        self.scrapers.get_mut(roster_name)
    }

    pub async fn scrape_roster_by_name(&mut self, roster_name: &str) -> Result<ScraperResult, ScraperError> {
        // Extract the scraper first, then release the lock
        let scraper_name = roster_name.to_string();
        let mut scraper = if let Some(scraper) = self.scrapers.remove(&scraper_name) {
            scraper
        } else {
            return Err(ScraperError::Generic(format!(
                "Scraper not found for roster: {}",
                roster_name
            )));
        };

        // Now scrape without holding the lock
        let result = scraper.scrape_roster().await;

        // Put the scraper back
        self.scrapers.insert(scraper_name, scraper);

        result
    }

    pub fn remove_scraper(&mut self, roster_name: &str) {
        self.scrapers.remove(roster_name);
    }

    pub fn take_scraper(&mut self, roster_name: &str) -> Option<HumanizedScraper> {
        self.scrapers.remove(roster_name)
    }

    pub fn return_scraper(&mut self, roster_name: String, scraper: HumanizedScraper) {
        self.scrapers.insert(roster_name, scraper);
    }

    pub fn get_active_scrapers(&self) -> Vec<String> {
        self.scrapers.keys().cloned().collect()
    }
}

impl Default for ScraperManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_vaanyar_scraper() {
        println!("Testing scraper with character: Vaanyar");

        let mut scraper = HumanizedScraper::new("Vaanyar".to_string(), "test_roster".to_string());

        match scraper.scrape_roster().await {
            Ok(result) => {
                println!("✅ SUCCESS: Scraping completed successfully!");
                println!("📊 Roster Name: {}", result.scraper_data.roster_name);
                println!("👤 Main Character: {}", result.scraper_data.main_character);
                println!("🕒 Timestamp: {}", result.scraper_data.timestamp);
                println!("🌐 Source: {}", result.scraper_data.source);
                println!("📈 Total Characters Found: {}", result.scraper_data.characters.len());
                println!();

                for (i, character) in result.scraper_data.characters.iter().enumerate() {
                    println!("🎮 Character {}: {}", i + 1, character.char_name);
                    println!("   🏷️  Class ID: {}", character.class_id);
                    println!("   ⚔️  Item Level: {:.2}", character.item_level);
                    println!("   💪 Combat Power: {}", character.combat_power);
                    println!("   📋 Display Order: {}", character.display_order);
                    println!("   🆔 Character ID: {}", character.char_id);
                    println!("   🏠 Roster ID: {}", character.roster_id);
                    println!();
                }

                assert_eq!(
                    result.scraper_data.characters.len(),
                    9,
                    "Expected 9 characters, but found {}",
                    result.scraper_data.characters.len()
                );

                println!("🎉 PERFECT: Found exactly 9 characters as expected!");
            }
            Err(e) => {
                println!("❌ ERROR: Scraping failed!");
                match e {
                    ScraperError::PrivateProfile => {
                        println!("🔒 Private profile detected");
                    }
                    ScraperError::CloudflareBlocked => {
                        println!("☁️ Cloudflare blocked");
                    }
                    ScraperError::NoCharactersFound => {
                        println!("📭 No characters found");
                    }
                    ScraperError::HttpError(code) => {
                        println!("🌐 HTTP error: {}", code);
                    }
                    ScraperError::JsonParsingError(err) => {
                        println!("📄 JSON parsing error: {}", err);
                    }
                    ScraperError::RosterDataNotFound => {
                        println!("🔍 Roster data not found in HTML");
                    }
                    ScraperError::RateLimited(duration) => {
                        println!("⏰ Rate limited. Wait: {:?}", duration);
                    }
                    ScraperError::Generic(msg) => {
                        println!("❓ Generic error: {}", msg);
                    }
                    ScraperError::RegexError(err) => {
                        println!("🔧 Regex error: {}", err);
                    }
                    ScraperError::RequestError(err) => {
                        println!("🌐 Request error: {}", err);
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn test_vaanyar_character_details() {
        println!("Testing character detail scraper with character: Vaanyar");

        let mut scraper = HumanizedScraper::new("Vaanyar".to_string(), "test_roster".to_string());

        match scraper.scrape_character_details("Vaanyar".to_string()).await {
            Ok(detail_data) => {
                println!("✅ SUCCESS: Character detail scraping completed successfully!");
                println!("👤 Character Name: {}", detail_data.character_name);
                println!("📚 Engravings Found: {}", detail_data.engravings.len());
                println!();

                for (i, engraving) in detail_data.engravings.iter().enumerate() {
                    println!("🎯 Engraving {}: {}", i + 1, engraving.engraving_name);
                    println!("   📖 Books Read: {}/{}", engraving.books_read, engraving.max_books);
                    println!("   💎 Stone Bonus: +{}", engraving.stone_bonus);
                    println!();
                }

                println!("⚔️ Equipment Found: {}", detail_data.equipment.len());
                println!();

                for (i, equip) in detail_data.equipment.iter().enumerate() {
                    println!("🛡️  Equipment {}: {}", i + 1, equip.slot);
                    if let Some(enh) = equip.enhancement_level {
                        println!("   ➕ Enhancement: +{}", enh);
                    }
                    if let Some(tier) = &equip.tier {
                        println!("   🏷️  Tier: {}", tier);
                    }
                    if let Some(quality) = equip.quality {
                        println!("   ⭐ Quality: {}", quality);
                    }
                    if let Some(ilvl) = equip.item_level {
                        println!("   📊 Item Level: {:.1}", ilvl);
                    }
                    println!();
                }

                println!("💎 Gems Found: {}", detail_data.gems.len());
                println!();

                for (i, gem) in detail_data.gems.iter().enumerate() {
                    println!("💎 Gem {}: {}", i + 1, gem.skill_name);
                    println!("   🔮 Type: {}", gem.gem_type);
                    println!("   ⭐ Level: {}", gem.gem_level);
                    println!();
                }

                // Basic validation
                assert!(!detail_data.equipment.is_empty(), "Expected at least one equipment piece");
                
                println!("🎉 Character detail scraping test passed!");
            }
            Err(e) => {
                println!("❌ ERROR: Character detail scraping failed!");
                match e {
                    ScraperError::PrivateProfile => {
                        println!("🔒 Private profile detected");
                    }
                    ScraperError::CloudflareBlocked => {
                        println!("☁️ Cloudflare blocked");
                    }
                    ScraperError::NoCharactersFound => {
                        println!("📭 No characters/loadouts found");
                    }
                    ScraperError::HttpError(code) => {
                        println!("🌐 HTTP error: {}", code);
                    }
                    ScraperError::JsonParsingError(err) => {
                        println!("📄 JSON parsing error: {}", err);
                    }
                    ScraperError::RosterDataNotFound => {
                        println!("🔍 Loadout data not found in HTML");
                    }
                    ScraperError::RateLimited(duration) => {
                        println!("⏰ Rate limited. Wait: {:?}", duration);
                    }
                    ScraperError::Generic(msg) => {
                        println!("❓ Generic error: {}", msg);
                    }
                    ScraperError::RegexError(err) => {
                        println!("🔧 Regex error: {}", err);
                    }
                    ScraperError::RequestError(err) => {
                        println!("🌐 Request error: {}", err);
                    }
                }
                panic!("Character detail scraping should succeed");
            }
        }
    }
}
