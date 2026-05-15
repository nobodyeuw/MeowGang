use serde::{Deserialize, Serialize};
use tauri::State;

use crate::market::market_database::MarketItem;
use crate::market::market_scraper::{HistoricalPriceEntry, RefreshResult};
use crate::market::{MarketDatabase, MarketScraper};

#[derive(Debug, Serialize, Deserialize)]
pub struct ManualPriceInput {
    pub item_slug: String,
    pub item_name: String,
    pub category: String,
    pub price: i64,
}

/// Fetch all market prices (engravings + honing) from the API and store them.
#[tauri::command]
pub async fn refresh_market_prices(market_db: State<'_, MarketDatabase>) -> Result<RefreshResult, String> {
    crate::log_info!("Handler: refresh_market_prices called");

    let scraper = MarketScraper::new();
    scraper.refresh_all(&market_db).await.map_err(|e| {
        crate::log_error!("Failed to refresh market prices: {}", e);
        format!("Failed to refresh market prices: {}", e)
    })
}

/// Get all market prices, with manual overrides applied.
#[tauri::command]
pub fn get_all_market_prices(market_db: State<'_, MarketDatabase>) -> Result<Vec<MarketItem>, String> {
    market_db
        .get_all_prices()
        .map_err(|e| format!("Failed to get market prices: {}", e))
}

/// Get market prices for a specific category.
#[tauri::command]
pub fn get_market_prices_by_category(
    category: String,
    market_db: State<'_, MarketDatabase>,
) -> Result<Vec<MarketItem>, String> {
    let valid_categories = ["engraving", "honing", "additional_honing", "gems"];
    if !valid_categories.contains(&category.as_str()) {
        return Err(format!(
            "Invalid category: {}. Must be one of: {:?}",
            category, valid_categories
        ));
    }

    market_db
        .get_prices_by_category(&category)
        .map_err(|e| format!("Failed to get prices for category {}: {}", category, e))
}

/// Get the effective price for a single item (manual override or API price).
#[tauri::command]
pub fn get_market_price(item_slug: String, market_db: State<'_, MarketDatabase>) -> Result<Option<MarketItem>, String> {
    if item_slug.is_empty() {
        return Err("Item slug cannot be empty".to_string());
    }

    market_db
        .get_effective_price(&item_slug)
        .map_err(|e| format!("Failed to get price for {}: {}", item_slug, e))
}

/// Set a manual price override for an item.
#[tauri::command]
pub fn set_manual_market_price(input: ManualPriceInput, market_db: State<'_, MarketDatabase>) -> Result<(), String> {
    if input.item_slug.is_empty() {
        return Err("Item slug cannot be empty".to_string());
    }
    if input.price < 0 {
        return Err("Price cannot be negative".to_string());
    }

    market_db
        .set_manual_price(&input.item_slug, &input.item_name, &input.category, input.price)
        .map_err(|e| format!("Failed to set manual price: {}", e))
}

/// Remove a manual price override, reverting to the API price.
#[tauri::command]
pub fn remove_manual_market_price(item_slug: String, market_db: State<'_, MarketDatabase>) -> Result<bool, String> {
    if item_slug.is_empty() {
        return Err("Item slug cannot be empty".to_string());
    }

    market_db
        .remove_manual_price(&item_slug)
        .map_err(|e| format!("Failed to remove manual price: {}", e))
}

/// Check if market data needs a refresh (older than 1 hour).
#[tauri::command]
pub fn market_needs_refresh(market_db: State<'_, MarketDatabase>) -> Result<bool, String> {
    market_db
        .needs_refresh()
        .map_err(|e| format!("Failed to check refresh status: {}", e))
}

/// Get all gem prices (manual-only entries).
#[tauri::command]
pub fn get_gem_prices(market_db: State<'_, MarketDatabase>) -> Result<Vec<MarketItem>, String> {
    market_db
        .get_gem_prices()
        .map_err(|e| format!("Failed to get gem prices: {}", e))
}

/// Get historical price data for an item (fetched live from API).
#[tauri::command]
pub async fn get_price_history(item_slug: String, days: u32) -> Result<Vec<HistoricalPriceEntry>, String> {
    if item_slug.is_empty() {
        return Err("Item slug cannot be empty".to_string());
    }
    if days == 0 || days > 30 {
        return Err("Days must be between 1 and 30".to_string());
    }

    let scraper = MarketScraper::new();
    scraper
        .fetch_price_history(&item_slug, days)
        .await
        .map_err(|e| format!("Failed to get price history for {}: {}", item_slug, e))
}
