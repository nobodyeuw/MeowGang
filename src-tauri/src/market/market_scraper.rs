use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::MarketDatabase;

const API_URL: &str = "https://marketdata-api.yrzhao1068589.workers.dev/v1/prices/latest";
const HISTORICAL_API_URL: &str = "https://marketdata-api.yrzhao1068589.workers.dev/v1/prices/historical";
const REGION_SLUG: &str = "euc";

/// Item slugs for engraving recipes.
const ENGRAVING_SLUGS: &[&str] = &[
    "adrenaline",
    "all-out-attack",
    "ambush-master",
    "awakening",
    "barricade",
    "broken-bone",
    "contender",
    "crisis-evasion",
    "crushing-fist",
    "cursed-doll",
    "disrespect",
    "divine-protection",
    "drops-of-ether",
    "emergency-rescue",
    "enhanced-shield",
    "ether-predator",
    "expert",
    "explosive-expert",
    "fortitude",
    "grudge",
    "heavy-armor",
    "hit-master",
    "keen-blunt-weapon",
    "lightning-fury",
    "magick-stream",
    "mass-increase",
    "master-brawler",
    "master-of-escape",
    "masters-tenacity",
    "max-mp-increase",
    "mp-efficiency-increase",
    "necromancy",
    "precise-dagger",
    "preemptive-strike",
    "propulsion",
    "raid-captain",
    "shield-piercing",
    "sight-focus",
    "spirit-absorption",
    "stabilized-status",
    "strong-will",
    "super-charge",
    "vital-point-hit",
];

/// Item slugs for honing materials.
const HONING_SLUGS: &[&str] = &[
    "guardian-stone-fragment",
    "destruction-stone-fragment",
    "destruction-stone",
    "guardian-stone",
    "crystallized-guardian-stone",
    "crystallized-destruction-stone",
    "protection-stone",
    "obliteration-stone",
    "refined-protection-stone",
    "refined-obliteration-stone",
    "destiny-guardian-stone",
    "destiny-destruction-stone",
    "destiny-crystallized-guardian-stone",
    "destiny-crystallized-destruction-stone",
    "harmony-shard-pouch-s",
    "harmony-shard-pouch-m",
    "harmony-shard-pouch-l",
    "honor-shard-pouch-s",
    "honor-shard-pouch-m",
    "honor-shard-pouch-l",
    "destiny-shard-pouch-s",
    "destiny-shard-pouch-m",
    "destiny-shard-pouch-l",
    "harmony-leapstone",
    "life-leapstone",
    "honor-leapstone",
    "great-honor-leapstone",
    "marvelous-honor-leapstone",
    "radiant-honor-leapstone",
    "destiny-leapstone",
    "great-destiny-leapstone",
    "oreha-fusion-material",
    "superior-oreha-fusion-material",
    "prime-oreha-fusion-material",
    "abidos-fusion-material",
    "superior-abidos-fusion-material",
];

/// Item slugs for additional honing materials.
const ADDITIONAL_HONING_SLUGS: &[&str] = &[
    "solar-grace",
    "solar-blessing",
    "solar-protection",
    "glaciers-breath",
    "lavas-breath",
    "artisans-metallurgy-level-1",
    "artisans-tailoring-level-1",
    "artisans-metallurgy-level-2",
    "artisans-tailoring-level-2",
    "artisans-metallurgy-level-3",
    "artisans-tailoring-level-3",
    "artisans-metallurgy-level-4",
    "artisans-tailoring-level-4",
    "metallurgy-decay-16-19",
    "tailoring-decay-16-19",
    "metallurgy-hellfire-11-14",
    "tailoring-hellfire-11-14",
    "metallurgy-hellfire-15-18",
    "tailoring-hellfire-15-18",
    "metallurgy-hellfire-19-20",
    "tailoring-hellfire-19-20",
];

/// API request body.
#[derive(Serialize)]
struct PriceRequest {
    region_slug: String,
    item_slugs: Vec<String>,
}

/// Single price entry from the API response.
#[derive(Debug, Deserialize)]
pub struct PriceEntry {
    pub item_slug: String,
    pub price: i64,
    pub timestamp: i64,
}

/// Single historical price entry from the API.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoricalPriceEntry {
    pub day: String,
    pub min_price: f64,
    pub max_price: f64,
    pub avg_price: f64,
}

/// Result of a market refresh operation.
#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshResult {
    pub engravings_updated: usize,
    pub honing_updated: usize,
    pub additional_honing_updated: usize,
    pub timestamp: i64,
}

/// Scrapes market data from the LOA Buddy API.
pub struct MarketScraper {
    client: Client,
}

impl MarketScraper {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("Failed to create HTTP client for market scraper");

        Self { client }
    }

    /// Fetch prices for a list of item slugs from the API.
    async fn fetch_prices(&self, slugs: &[&str]) -> Result<Vec<PriceEntry>> {
        let request_body = PriceRequest {
            region_slug: REGION_SLUG.to_string(),
            item_slugs: slugs.iter().map(|s| s.to_string()).collect(),
        };

        crate::log_debug!("Fetching {} market prices from LOA Buddy API", slugs.len());

        let response = self.client.post(API_URL).json(&request_body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Market API returned {}: {}", status, body);
        }

        let entries: Vec<PriceEntry> = response.json().await?;
        crate::log_info!("Received {} price entries from API", entries.len());
        Ok(entries)
    }

    /// Convert item slug to display name.
    fn slug_to_display_name(slug: &str) -> String {
        slug.split('-')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        let upper = first.to_uppercase().to_string();
                        upper + &chars.collect::<String>()
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Fetch and store all market data (engravings + honing materials).
    pub async fn refresh_all(&self, db: &MarketDatabase) -> Result<RefreshResult> {
        crate::log_info!("Starting full market data refresh");

        let now = chrono::Utc::now().timestamp();

        // Fetch engravings
        let engraving_entries = self.fetch_prices(ENGRAVING_SLUGS).await?;
        let engraving_items: Vec<(String, String, String, i64)> = engraving_entries
            .iter()
            .map(|e| {
                let display_name = format!("{} Engraving Recipe", Self::slug_to_display_name(&e.item_slug));
                (e.item_slug.clone(), display_name, "engraving".to_string(), e.price)
            })
            .collect();
        let engravings_updated = db.upsert_prices(&engraving_items, now)?;

        // Fetch honing materials
        let honing_entries = self.fetch_prices(HONING_SLUGS).await?;
        let honing_items: Vec<(String, String, String, i64)> = honing_entries
            .iter()
            .map(|e| {
                let display_name = Self::slug_to_display_name(&e.item_slug);
                (e.item_slug.clone(), display_name, "honing".to_string(), e.price)
            })
            .collect();
        let honing_updated = db.upsert_prices(&honing_items, now)?;

        // Fetch additional honing materials
        let additional_entries = self.fetch_prices(ADDITIONAL_HONING_SLUGS).await?;
        let additional_items: Vec<(String, String, String, i64)> = additional_entries
            .iter()
            .map(|e| {
                let display_name = Self::slug_to_display_name(&e.item_slug);
                (
                    e.item_slug.clone(),
                    display_name,
                    "additional_honing".to_string(),
                    e.price,
                )
            })
            .collect();
        let additional_honing_updated = db.upsert_prices(&additional_items, now)?;

        // Update last refresh timestamp
        db.set_setting("last_full_refresh", &now.to_string())?;

        let result = RefreshResult {
            engravings_updated,
            honing_updated,
            additional_honing_updated,
            timestamp: now,
        };

        crate::log_info!(
            "Market refresh complete: {} engravings, {} honing, {} additional honing",
            engravings_updated,
            honing_updated,
            additional_honing_updated
        );

        Ok(result)
    }

    /// Fetch only engraving prices.
    pub async fn refresh_engravings(&self, db: &MarketDatabase) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();
        let entries = self.fetch_prices(ENGRAVING_SLUGS).await?;
        let items: Vec<(String, String, String, i64)> = entries
            .iter()
            .map(|e| {
                let display_name = format!("{} Engraving Recipe", Self::slug_to_display_name(&e.item_slug));
                (e.item_slug.clone(), display_name, "engraving".to_string(), e.price)
            })
            .collect();
        db.upsert_prices(&items, now)
    }

    /// Fetch historical price data for a single item.
    pub async fn fetch_price_history(&self, item_slug: &str, days: u32) -> Result<Vec<HistoricalPriceEntry>> {
        let end_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let start_date = (chrono::Utc::now() - chrono::Duration::days(days as i64))
            .format("%Y-%m-%d")
            .to_string();

        let url = format!(
            "{}/{}/{}?start_date={}&end_date={}",
            HISTORICAL_API_URL, REGION_SLUG, item_slug, start_date, end_date
        );

        crate::log_debug!(
            "Fetching {}-day price history for {} from LOA Buddy API",
            days,
            item_slug
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Historical API returned {}: {}", status, body);
        }

        let entries: Vec<HistoricalPriceEntry> = response.json().await?;
        crate::log_info!("Received {} historical entries for {}", entries.len(), item_slug);
        Ok(entries)
    }

    /// Fetch only honing material prices.
    pub async fn refresh_honing(&self, db: &MarketDatabase) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();
        let entries = self.fetch_prices(HONING_SLUGS).await?;
        let items: Vec<(String, String, String, i64)> = entries
            .iter()
            .map(|e| {
                let display_name = Self::slug_to_display_name(&e.item_slug);
                (e.item_slug.clone(), display_name, "honing".to_string(), e.price)
            })
            .collect();
        db.upsert_prices(&items, now)
    }
}
