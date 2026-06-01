use super::item_mapping::{get_engraving_name, get_gem_name};
use super::skill_mapping::get_gem_skill_label;
use super::ScraperError;
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

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
    pub effects_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterGem {
    pub slot_index: i64,
    pub gem_name: String,
    pub skill_name: String,
    pub gem_type: String,
    pub gem_level: i64,
    pub is_bound: bool,
    pub gem_item_id: Option<i64>,
    pub skill_id: Option<i64>,
    pub skill_icon: Option<String>,
    pub effect_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterDetailData {
    pub character_name: String,
    pub engravings: Vec<CharacterEngraving>,
    pub equipment: Vec<CharacterEquipment>,
    pub gems: Vec<CharacterGem>,
}

/// Experimental character-detail scraper used only by the hidden progression planner.
///
/// This intentionally does not extend the working roster scraper type. Keeping
/// detail scraping isolated lets us tune loadout/equipment/gem parsing without
/// risking roster scraping, which is the stable production path.
#[derive(Debug, Clone)]
pub struct DetailedCharacterScraper {
    roster_name: String,
    client: Client,
    user_agents: Vec<String>,
    referers: Vec<String>,
}

impl DetailedCharacterScraper {
    pub fn new(roster_name: String) -> Self {
        crate::log_info!("Starting character-detail scraper session for roster '{}'", roster_name);

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
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

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

        Self {
            roster_name,
            client,
            user_agents,
            referers,
        }
    }

    async fn humanized_delay(&self, min_seconds: f64, max_seconds: f64) {
        let delay = {
            let mut rng = rand::thread_rng();
            rng.gen_range(min_seconds..=max_seconds)
        };
        crate::log_debug!("Applying character-detail scraper delay: {:.2} seconds", delay);
        sleep(Duration::from_secs_f64(delay)).await;
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

    async fn setup_character_session(
        &self,
        character_name: &str,
    ) -> Result<reqwest::RequestBuilder, Box<dyn std::error::Error + Send + Sync>> {
        crate::log_debug!(
            "Setting up HTTP session for character detail scraping in roster '{}'",
            self.roster_name
        );

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

        crate::log_debug!("Character detail HTTP request configured with headers");
        Ok(request)
    }

    fn parse_character_loadouts(&self, html: &str) -> Result<Vec<serde_json::Value>, ScraperError> {
        crate::log_debug!("Parsing character loadouts from HTML, length: {}", html.len());

        let loadouts_start = html
            .find("loadouts:[")
            .ok_or_else(|| ScraperError::RosterDataNotFound)?;
        let html_slice = &html[loadouts_start..];
        let bracket_pos = html_slice.find('[').ok_or_else(|| ScraperError::RosterDataNotFound)?;
        let array_start = loadouts_start + bracket_pos;

        let loadouts_js = self
            .extract_array_by_bracket_counting_from_position(html, array_start)
            .ok_or_else(|| ScraperError::RosterDataNotFound)?;

        crate::log_info!("Found loadouts data using bracket counting");
        self.parse_loadouts_json(&loadouts_js)
    }

    fn extract_array_by_bracket_counting_from_position(&self, html: &str, start_pos: usize) -> Option<String> {
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
                        let array_end = start_pos + i + 1;
                        return Some(html[start_pos..array_end].to_string());
                    }
                }
                _ => {}
            }
        }

        None
    }

    fn parse_loadouts_json(&self, loadouts_js: &str) -> Result<Vec<serde_json::Value>, ScraperError> {
        let sanitized = loadouts_js.replace("void 0", "null").replace("void(0)", "null");
        json5::from_str(&sanitized).map_err(|e| {
            crate::log_error!("Character loadout JSON5 parsing error: {}", e);
            ScraperError::Generic(format!("JSON5 parsing error: {}", e))
        })
    }

    fn extract_engravings_from_loadout(&self, loadout: &serde_json::Value) -> Vec<CharacterEngraving> {
        let mut engravings = Vec::new();
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
                                stone_bonus_map.insert(Self::normalize_engraving_id(id), nodes);
                            }
                        }
                    }
                    break;
                }
            }
        }

        if let Some(engraving_data) = loadout.get("engravings").and_then(|e| e.as_array()) {
            for engraving in engraving_data {
                let id = match engraving.get("id").and_then(|i| i.as_i64()) {
                    Some(v) => v,
                    None => continue,
                };
                if (id >= 800 && id <= 803) || (id >= 1800 && id <= 1803) {
                    continue;
                }

                let books_read = Self::extract_numeric_i64(
                    engraving,
                    &["progress", "booksRead", "readCount", "bookCount", "points"],
                )
                .unwrap_or_else(|| {
                    if engraving.get("max").and_then(|m| m.as_bool()) == Some(true)
                        || engraving.get("complete").and_then(|m| m.as_bool()) == Some(true)
                        || engraving.get("completed").and_then(|m| m.as_bool()) == Some(true)
                    {
                        20
                    } else {
                        0
                    }
                });
                let normalized_id = Self::normalize_engraving_id(id);
                let stone_bonus = *stone_bonus_map.get(&normalized_id).unwrap_or(&0);
                let engraving_name = get_engraving_name(id)
                    .unwrap_or_else(|| {
                        crate::log_debug!("Unknown engraving id: {}", id);
                        "Unknown Engraving"
                    })
                    .to_string();

                engravings.push(CharacterEngraving {
                    engraving_name,
                    books_read: books_read as f64,
                    max_books: 20.0,
                    stone_bonus: stone_bonus as f64,
                });
            }
        }

        engravings
    }

    fn normalize_engraving_id(id: i64) -> i64 {
        if id >= 1000 {
            id - 1000
        } else {
            id
        }
    }

    fn extract_numeric_i64(value: &serde_json::Value, keys: &[&str]) -> Option<i64> {
        for key in keys {
            if let Some(raw) = value.get(key) {
                if let Some(v) = raw.as_i64() {
                    return Some(v);
                }
                if let Some(v) = raw.as_f64() {
                    return Some(v as i64);
                }
                if let Some(v) = raw.as_str().and_then(|s| s.parse::<i64>().ok()) {
                    return Some(v);
                }
            }
        }
        None
    }

    fn item_effect_label(stat: &serde_json::Value, is_armor: bool) -> Option<String> {
        for key in ["label", "name", "displayName", "text", "tooltip", "description"] {
            if let Some(value) = stat.get(key).and_then(|v| v.as_str()) {
                if !value.trim().is_empty() {
                    return Some(value.trim().to_string());
                }
            }
        }

        let stat_type = stat.get("type").and_then(|v| v.as_i64());
        let index = stat.get("index").and_then(|v| v.as_i64());
        match (stat_type, index) {
            (Some(1), _) => Some("Crit".to_string()),
            (Some(2), _) => Some("Specialization".to_string()),
            (Some(3), _) => Some("Domination".to_string()),
            (Some(4), _) => Some("Swiftness".to_string()),
            (Some(5), _) => Some("Endurance".to_string()),
            (Some(6), _) => Some("Expertise".to_string()),
            (Some(45), _) => Some("Max HP".to_string()),
            (Some(46), _) => Some("Attack Power".to_string()),
            (Some(53), _) => Some("Weapon Power".to_string()),
            (Some(57), Some(1)) if is_armor => Some("Quality".to_string()),
            (Some(57), _) => Some("Evolution Points".to_string()),
            _ => None,
        }
    }

    fn item_effect_grade(stat: &serde_json::Value) -> Option<String> {
        for key in ["grade", "rank", "quality", "tier"] {
            if let Some(value) = stat.get(key).and_then(|v| v.as_str()) {
                if !value.trim().is_empty() {
                    return Some(value.trim().to_lowercase());
                }
            }
        }
        None
    }

    fn extract_item_effects_json(data: Option<&serde_json::Value>, is_armor: bool) -> Option<String> {
        let stats = data.and_then(|d| d.get("stats")).and_then(|s| s.as_array())?;

        let mut effects = Vec::new();
        for stat in stats {
            let label = match Self::item_effect_label(stat, is_armor) {
                Some(label) => label,
                None => continue,
            };
            if is_armor && label == "Quality" {
                continue;
            }

            let value = stat
                .get("value")
                .or_else(|| stat.get("amount"))
                .or_else(|| stat.get("v"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            if value.is_null() {
                continue;
            }

            let mut effect = serde_json::Map::new();
            effect.insert("label".to_string(), serde_json::Value::String(label));
            effect.insert("value".to_string(), value);
            if let Some(grade) = Self::item_effect_grade(stat) {
                effect.insert("grade".to_string(), serde_json::Value::String(grade));
            }
            effects.push(serde_json::Value::Object(effect));
        }

        if effects.is_empty() {
            None
        } else {
            serde_json::to_string(&effects).ok()
        }
    }

    fn extract_equipment_from_loadout(&self, loadout: &serde_json::Value) -> Vec<CharacterEquipment> {
        let mut equipment = Vec::new();
        let slot_map: std::collections::HashMap<&str, &str> = [
            ("weapon", "weapon"),
            ("head", "head"),
            ("upper_body", "chest"),
            ("lower_body", "pants"),
            ("hand", "gloves"),
            ("shoulder", "shoulder"),
            ("neck", "neck"),
            ("ear1", "earring1"),
            ("ear2", "earring2"),
            ("finger1", "ring1"),
            ("finger2", "ring2"),
            ("bracelet", "bracelet"),
            ("ability_stone", "ability_stone"),
        ]
        .iter()
        .cloned()
        .collect();

        let armor_slots = ["weapon", "head", "upper_body", "lower_body", "hand", "shoulder"];
        let accessory_slots = [
            "neck",
            "ear1",
            "ear2",
            "finger1",
            "finger2",
            "bracelet",
            "ability_stone",
        ];

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
                let is_armor = armor_slots.contains(&raw_slot);
                let quality = if is_armor {
                    data.and_then(|d| d.get("stats"))
                        .and_then(|s| s.as_array())
                        .and_then(|stats| {
                            stats.iter().find(|s| {
                                s.get("type").and_then(|t| t.as_i64()) == Some(57)
                                    && s.get("index").and_then(|i| i.as_i64()) == Some(1)
                            })
                        })
                        .and_then(|s| s.get("value").and_then(|v| v.as_i64()))
                } else {
                    None
                };
                let item_level = data
                    .and_then(|d| d.get("itemLevel"))
                    .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)))
                    .or_else(|| {
                        if armor_slots.contains(&raw_slot) {
                            loadout.get("itemLevel").and_then(|v| v.as_f64())
                        } else {
                            None
                        }
                    });
                let tier = item_level
                    .map(|ilvl| {
                        if ilvl >= 1680.0 {
                            "T4.5".to_string()
                        } else if ilvl >= 1600.0 {
                            "T4".to_string()
                        } else {
                            "T3".to_string()
                        }
                    })
                    .or_else(|| {
                        data.and_then(|d| d.get("type")).and_then(|t| t.as_str()).map(|t| {
                            if t.contains("tier4") {
                                "T4".to_string()
                            } else {
                                "T3".to_string()
                            }
                        })
                    })
                    .or_else(|| {
                        if accessory_slots.contains(&raw_slot)
                            && loadout
                                .get("itemLevel")
                                .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)))
                                .map(|ilvl| ilvl >= 1600.0)
                                .unwrap_or(false)
                        {
                            Some("T4".to_string())
                        } else {
                            None
                        }
                    });
                let effects_json = Self::extract_item_effects_json(data, is_armor);

                equipment.push(CharacterEquipment {
                    slot,
                    enhancement_level: enhancement_level.map(|v| v as f64),
                    tier,
                    quality: quality.map(|v| v as f64),
                    item_level,
                    effects_json,
                });
            }
        }

        equipment
    }

    fn extract_gems_from_loadout(&self, loadout: &serde_json::Value) -> Vec<CharacterGem> {
        let mut gems = Vec::new();

        if let Some(gems_data) = loadout.get("gems").and_then(|g| g.as_array()) {
            for gem in gems_data {
                let gem_item_id = match gem.get("id").and_then(|i| i.as_i64()) {
                    Some(v) => v,
                    None => continue,
                };
                let slot_index = gem.get("slot").and_then(|s| s.as_i64()).unwrap_or(0);
                let last3 = gem_item_id % 1000;
                let gem_level = {
                    let lv = last3 / 10;
                    if lv == 0 {
                        10
                    } else {
                        lv
                    }
                };
                let gem_name = get_gem_name(gem_item_id)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("Gem (id {})", gem_item_id));
                let is_bound = gem.get("bound").and_then(|b| b.as_bool()).unwrap_or(false);
                let effects = gem.get("effects").and_then(|e| e.as_array());
                let primary = effects.and_then(|effs| {
                    effs.iter()
                        .find(|e| e.get("type").and_then(|t| t.as_i64()).map(|t| t != 2).unwrap_or(false))
                });
                let gem_type = primary
                    .and_then(|e| e.get("type").and_then(|t| t.as_i64()))
                    .map(|t| match t {
                        5 | 34 => "attack".to_string(),
                        27 => "cooldown".to_string(),
                        o => format!("type_{}", o),
                    })
                    .unwrap_or_else(|| "unknown".to_string());
                let skill_id = primary.and_then(|e| e.get("id").and_then(|i| i.as_i64()));
                let skill_label = skill_id.and_then(get_gem_skill_label);
                let skill_name = skill_label
                    .map(|label| label.name.to_string())
                    .unwrap_or_else(|| gem_name.clone());
                let skill_icon = skill_label.map(|label| label.icon.to_string());
                let effect_value = primary.and_then(|e| e.get("value")).and_then(|v| {
                    v.as_f64()
                        .or_else(|| v.as_i64().map(|n| n as f64))
                        .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                });

                gems.push(CharacterGem {
                    slot_index,
                    gem_name,
                    skill_name,
                    gem_type,
                    gem_level,
                    is_bound,
                    gem_item_id: Some(gem_item_id),
                    skill_id,
                    skill_icon,
                    effect_value,
                });
            }
        }

        gems
    }

    pub async fn scrape_character_details(
        &mut self,
        character_name: String,
    ) -> Result<CharacterDetailData, ScraperError> {
        crate::log_info!("Starting character detail scrape for '{}'", character_name);

        self.humanized_delay(1.0, 3.0).await;

        let request = self
            .setup_character_session(&character_name)
            .await
            .map_err(|e| ScraperError::Generic(e.to_string()))?;

        let response = request
            .send()
            .await
            .map_err(|e| ScraperError::Generic(format!("Request error: {}", e)))?;
        let status = response.status().as_u16();

        if status == 404 {
            return Err(ScraperError::PrivateProfile);
        }

        let content = response
            .text()
            .await
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

        let loadouts = self.parse_character_loadouts(&content)?;
        if loadouts.is_empty() {
            crate::log_warn!("No loadouts found for character '{}'", character_name);
            return Err(ScraperError::NoCharactersFound);
        }

        let preferred_loadout = crate::roster::pick_preferred_raid_loadout(&loadouts)
            .ok_or_else(|| ScraperError::Generic("No suitable raid loadout found".to_string()))?;

        crate::log_info!(
            "Selected loadout with classification: {:?}",
            preferred_loadout.get("classification").and_then(|c| c.as_str())
        );

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
