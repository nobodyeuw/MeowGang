import { invoke } from '@tauri-apps/api/core';
import type { HistoricalPriceEntry, MarketItem, RefreshResult } from '$lib/components/market/types';

export function marketNeedsRefresh(): Promise<boolean> {
  return invoke<boolean>('market_needs_refresh');
}

export async function loadMarketPrices(): Promise<MarketItem[]> {
  const [priceItems, gemItems, accessoryItems] = await Promise.all([
    invoke<MarketItem[]>('get_all_market_prices'),
    invoke<MarketItem[]>('get_gem_prices'),
    invoke<MarketItem[]>('get_accessory_prices')
  ]);
  return [...priceItems, ...gemItems, ...accessoryItems];
}

export function refreshMarketPrices(): Promise<RefreshResult> {
  return invoke<RefreshResult>('refresh_market_prices');
}

export function setMarketFavorite(itemSlug: string, favorite: boolean) {
  return invoke('set_market_favorite', {
    input: {
      item_slug: itemSlug,
      favorite
    }
  });
}

export function setManualMarketPrice(item: MarketItem, price: number) {
  return invoke('set_manual_market_price', {
    input: {
      item_slug: item.item_slug,
      item_name: item.item_name,
      category: item.category,
      price: Math.round(price)
    }
  });
}

export function removeManualMarketPrice(itemSlug: string) {
  return invoke('remove_manual_market_price', { itemSlug });
}

export function resetManualMarketPriceToEstimate(itemSlug: string) {
  return invoke('reset_manual_market_price_to_estimate', { itemSlug });
}

export function loadPriceHistory(itemSlug: string, days: number): Promise<HistoricalPriceEntry[]> {
  return invoke<HistoricalPriceEntry[]>('get_price_history', { itemSlug, days });
}
