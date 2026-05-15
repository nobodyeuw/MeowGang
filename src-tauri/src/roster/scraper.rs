use rand::Rng;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tokio::time::sleep;
use urlencoding;

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
}
