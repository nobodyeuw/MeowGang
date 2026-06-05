export type MarketCategory = 'engraving' | 'honing' | 'additional_honing' | 'trade' | 'gems' | 'accessories';
export type GemFilter = 'all' | 't3-damage' | 't3-cooldown' | 't4-damage' | 't4-cooldown';
export type AccessoryFilter = 'all' | 'necklace' | 'earring' | 'ring';
export type AccessoryRoleFilter = 'all' | 'dps' | 'support';
export type HoningFilter = 'all' | 't3' | 't4' | 't4.5';
export type TradeFilter = 'all' | 'foraging' | 'logging' | 'mining' | 'hunting' | 'fishing' | 'excavation';
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
  gem_tier?: string | null;
  gem_kind?: string | null;
  gem_level?: number | null;
  is_manual_only?: boolean;
  estimated_price?: number | null;
}

export interface RefreshResult {
  engravings_updated: number;
  honing_updated: number;
  additional_honing_updated: number;
  trade_updated: number;
  timestamp: number;
}

export interface HistoricalPriceEntry {
  day: string;
  min_price: number;
  max_price: number;
  avg_price: number;
}
