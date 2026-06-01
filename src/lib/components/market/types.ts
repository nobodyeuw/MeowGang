export type MarketCategory = 'engraving' | 'honing' | 'additional_honing' | 'gems';
export type GemFilter = 'all' | 't3-damage' | 't3-cooldown' | 't4-damage' | 't4-cooldown';
export type HoningFilter = 'all' | 't3' | 't4' | 't4.5';
export type MarketSortKey = 'name' | 'price';
export type HistoryDays = 7 | 14 | 30;

export interface MarketItem {
  item_slug: string;
  item_name: string;
  category: MarketCategory;
  price: number;
  fetched_at: number;
  is_manual_override: boolean;
  favorite: boolean;
}

export interface RefreshResult {
  engravings_updated: number;
  honing_updated: number;
  additional_honing_updated: number;
  timestamp: number;
}

export interface HistoricalPriceEntry {
  day: string;
  min_price: number;
  max_price: number;
  avg_price: number;
}
