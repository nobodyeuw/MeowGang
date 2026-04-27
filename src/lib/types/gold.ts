export interface GoldLogEntry {
  timestamp: number;
  charId: number;
  source: 'raid' | 'auction' | 'other';
  goldValueTotal: number;
  goldBound: number;
  goldTradable: number;
  notes: string; // Contains details like "Aegir Normal Gate 1"
}

export interface WeeklyGoldSummary {
  tradableGold: number;
  boundGold: number;
  totalGold: number;
  totalEntries: number;
  extraIncomeGold: number;
  boxPurchaseCost: number;
}

export interface GoldStatsResponse {
  weekly: WeeklyGoldSummary;
  recentEntries: GoldLogEntry[];
}

export type GoldSource = 'raid' | 'auction' | 'other';
