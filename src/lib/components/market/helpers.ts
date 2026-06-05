import { marketAsset, assetUrl } from '$lib/assets';
import { getAccessoryCombinationSummary } from '$lib/data/accessories';
import { getTradeSkillCategoryForSlug } from '$lib/data/stronghold-crafting';
import type { AccessoryRoleFilter, HistoricalPriceEntry, HoningFilter, MarketCategory, MarketItem, MarketSortKey, TradeFilter } from './types';
import type { AccessoryRollColor, AccessoryRollGrade } from '$lib/data/accessories';

const HONING_T3_SLUGS = new Set<string>([
  'guardian-stone-fragment',
  'destruction-stone-fragment',
  'destruction-stone',
  'guardian-stone',
  'crystallized-guardian-stone',
  'crystallized-destruction-stone',
  'protection-stone',
  'obliteration-stone',
  'refined-protection-stone',
  'refined-obliteration-stone',
  'harmony-shard-pouch-s',
  'harmony-shard-pouch-m',
  'harmony-shard-pouch-l',
  'honor-shard-pouch-s',
  'honor-shard-pouch-m',
  'honor-shard-pouch-l',
  'harmony-leapstone',
  'life-leapstone',
  'honor-leapstone',
  'great-honor-leapstone',
  'marvelous-honor-leapstone',
  'radiant-honor-leapstone',
  'oreha-fusion-material',
  'superior-oreha-fusion-material',
  'prime-oreha-fusion-material',
  'solar-grace',
  'solar-blessing',
  'solar-protection',
  'metallurgy-decay-16-19',
  'tailoring-decay-16-19'
]);

const HONING_T4_SLUGS = new Set<string>([
  'destiny-leapstone',
  'abidos-fusion-material',
  'destiny-guardian-stone',
  'destiny-destruction-stone',
  'artisans-metallurgy-level-1',
  'artisans-tailoring-level-1',
  'artisans-metallurgy-level-2',
  'artisans-tailoring-level-2',
  'artisans-metallurgy-level-3',
  'artisans-tailoring-level-3',
  'artisans-metallurgy-level-4',
  'artisans-tailoring-level-4',
  'metallurgy-hellfire-11-14',
  'metallurgy-hellfire-15-18',
  'metallurgy-hellfire-19-20',
  'tailoring-hellfire-11-14',
  'tailoring-hellfire-15-18',
  'tailoring-hellfire-19-20',
  'glaciers-breath',
  'lavas-breath',
  'destiny-shard-pouch-s',
  'destiny-shard-pouch-m',
  'destiny-shard-pouch-l'
]);

const HONING_T4_5_SLUGS = new Set<string>([
  'destiny-crystallized-guardian-stone',
  'destiny-crystallized-destruction-stone',
  'great-destiny-leapstone',
  'superior-abidos-fusion-material',
  'glaciers-breath',
  'lavas-breath',
  'destiny-shard-pouch-s',
  'destiny-shard-pouch-m',
  'destiny-shard-pouch-l'
]);

const PNG_ONLY_MARKET_ICON_BASES = new Set<string>([
  'destiny_crystallized_guardian_stone',
  'destiny_crystallized_destruction_stone',
  'great_destiny_leapstone',
  'superior_abidos_fusion_material',
  'artisans_metallurgy_level_3',
  'artisans_metallurgy_level_4',
  'artisans_tailoring_level_3',
  'artisans_tailoring_level_4'
]);

const MARKET_ICON_FALLBACK = marketAsset('gem/fallback_gem.png');

export function doesHoningTierMatchFilter(slug: string, filter: HoningFilter) {
  if (filter === 'all') return true;
  if (filter === 't3') return HONING_T3_SLUGS.has(slug);
  if (filter === 't4') return HONING_T4_SLUGS.has(slug);
  if (filter === 't4.5') return HONING_T4_5_SLUGS.has(slug);
  return true;
}

export function doesTradeSkillCategoryMatchFilter(slug: string, filter: TradeFilter) {
  if (filter === 'all') return true;
  return getTradeSkillCategoryForSlug(slug) === filter;
}

export function doesAccessoryRoleMatchFilter(slug: string, filter: AccessoryRoleFilter) {
  if (filter === 'all') return true;
  const summary = getAccessoryCombinationSummary(slug);
  return summary?.role === filter;
}

export function getLastRefreshedLabel(items: MarketItem[]): string {
  if (items.length === 0) return 'Never';
  const maxTs = Math.max(...items.map((item) => item.fetched_at));
  if (maxTs === 0) return 'Never';

  const date = new Date(maxTs * 1000);
  const diffMins = Math.floor((Date.now() - date.getTime()) / 60000);
  if (diffMins < 1) return 'Just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffMins < 1440) return `${Math.floor(diffMins / 60)}h ago`;
  return date.toLocaleDateString();
}

export function getNextSortState(currentKey: MarketSortKey, currentAscending: boolean, nextKey: MarketSortKey) {
  if (currentKey === nextKey) {
    return { sortKey: currentKey, sortAsc: !currentAscending };
  }
  return { sortKey: nextKey, sortAsc: true };
}

function getGemSortParts(item: MarketItem): { tier: number; kind: number; level: number } | undefined {
  if (item.category !== 'gems') return undefined;
  const match = item.item_slug.match(/^gem-t([34])-(damage|cooldown)-lv(\d+)$/);
  if (!match) return undefined;

  return {
    tier: Number(match[1]),
    kind: match[2] === 'damage' ? 0 : 1,
    level: Number(match[3])
  };
}

const ACCESSORY_ROLE_SORT_ORDER = { dps: 0, support: 1 } as const;
const ACCESSORY_SLOT_SORT_ORDER = { necklace: 0, earring: 1, ring: 2 } as const;
const ACCESSORY_GRADE_SORT_ORDER = { low: 0, mid: 1, high: 2 } as const;

function getAccessorySortParts(item: MarketItem): { role: number; slot: number; comboRank: number; primary: number; secondary: number } | undefined {
  if (item.category !== 'accessories') return undefined;

  const summary = getAccessoryCombinationSummary(item.item_slug);
  if (!summary) return undefined;

  const primaryGrade = summary.stats[0]?.grade as AccessoryRollGrade | undefined;
  const secondaryGrade = summary.stats[1]?.grade as AccessoryRollGrade | undefined;
  const primary = primaryGrade ? ACCESSORY_GRADE_SORT_ORDER[primaryGrade] : 99;
  const secondary = secondaryGrade ? ACCESSORY_GRADE_SORT_ORDER[secondaryGrade] : 99;
  const isSingleStat = summary.stats.length < 2;
  const normalizedLow = Math.min(primary, secondary);
  const normalizedHigh = Math.max(primary, secondary);

  // Two-stat accessories come first in value order:
  // low+low, low+mid, low+high, mid+mid, mid+high, high+high.
  // Single-stat budget references are grouped after the two-stat rows.
  return {
    role: ACCESSORY_ROLE_SORT_ORDER[summary.role],
    slot: ACCESSORY_SLOT_SORT_ORDER[summary.slot],
    comboRank: isSingleStat ? 100 + Math.min(primary, secondary) : normalizedLow * 3 + normalizedHigh,
    primary,
    secondary
  };
}

export function compareMarketItems(a: MarketItem, b: MarketItem, sortKey: MarketSortKey, ascending: boolean): number {
  if (a.favorite !== b.favorite) return a.favorite ? -1 : 1;

  const direction = ascending ? 1 : -1;
  if (sortKey === 'price') {
    return direction * (a.price - b.price);
  }

  const aGem = getGemSortParts(a);
  const bGem = getGemSortParts(b);
  if (aGem && bGem) {
    const gemCompare =
      aGem.tier - bGem.tier ||
      aGem.kind - bGem.kind ||
      aGem.level - bGem.level;
    if (gemCompare !== 0) return direction * gemCompare;
  }

  const aAccessory = getAccessorySortParts(a);
  const bAccessory = getAccessorySortParts(b);
  if (aAccessory && bAccessory) {
    const accessoryCompare =
      aAccessory.role - bAccessory.role ||
      aAccessory.slot - bAccessory.slot ||
      aAccessory.comboRank - bAccessory.comboRank ||
      aAccessory.primary - bAccessory.primary ||
      aAccessory.secondary - bAccessory.secondary;
    if (accessoryCompare !== 0) return direction * accessoryCompare;
  }

  return direction * a.item_name.localeCompare(b.item_name, undefined, { numeric: true });
}

export function handleMarketIconError(event: Event) {
  const img = event.target as HTMLImageElement;
  const currentSrc = img.src;

  if (currentSrc === MARKET_ICON_FALLBACK) {
    img.style.display = 'none';
    return;
  }

  const triedAlternate = img.dataset.triedAlternate === 'true';
  const triedFallback = img.dataset.triedFallback === 'true';

  if (!triedAlternate) {
    if (currentSrc.endsWith('.webp')) {
      img.dataset.triedAlternate = 'true';
      img.src = currentSrc.replace(/\.webp$/, '.png');
      return;
    }

    if (currentSrc.endsWith('.png') && !currentSrc.includes('fallback_gem')) {
      img.dataset.triedAlternate = 'true';
      img.src = currentSrc.replace(/\.png$/, '.webp');
      return;
    }
  }

  if (!triedFallback) {
    img.dataset.triedFallback = 'true';
    img.src = MARKET_ICON_FALLBACK;
    return;
  }

  img.style.display = 'none';
}

function slugToMarketIconBase(slug: string): string {
  const range = slug.match(/^(.*)-(\d+)-(\d+)$/);
  if (range) {
    const prefix = range[1].replace(/-/g, '_');
    return `${prefix}_${range[2]}_${range[3]}`;
  }
  return slug.replace(/-/g, '_');
}

export function buildMarketIconUrl(itemSlug: string, category: MarketCategory): string {
  if (itemSlug.startsWith('gem-')) {
    const match = itemSlug.match(/^gem-(t[34])-(damage|cooldown)-lv(\d+)$/);
    if (match) {
      const [, tier, type, level] = match;
      return marketAsset(`gem/${tier}_${type}_gem_${level}.png`);
    }
  }

  if (category === 'accessories') {
    const summary = getAccessoryCombinationSummary(itemSlug);
    if (summary) {
      return marketAsset(`accessories/ancient_${summary.slot}.png`);
    }
  }

  if (category === 'engraving') {
    return marketAsset('engraving/relic_book.webp');
  }

  if (category === 'honing' || category === 'additional_honing') {
    const base = slugToMarketIconBase(itemSlug);
    const extension = PNG_ONLY_MARKET_ICON_BASES.has(base) ? 'png' : 'webp';
    return marketAsset(`honing/${base}.${extension}`);
  }

  if (category === 'trade') {
    return marketAsset(`trading/${slugToMarketIconBase(itemSlug)}.webp`);
  }

  return MARKET_ICON_FALLBACK;
}

export function getAccessoryMarketDisplay(item: MarketItem) {
  if (item.category !== 'accessories') return undefined;

  const summary = getAccessoryCombinationSummary(item.item_slug);
  if (!summary || summary.stats.length === 0) return undefined;

  return {
    role: summary.role,
    roleIcon: assetUrl(`equipment/${summary.role === 'support' ? 'supporter.webp' : 'combat.webp'}`),
    roleLabel: summary.role === 'support' ? 'Support' : 'DPS',
    slot: summary.slot,
    combinedDamagePercent: summary.combinedDamagePercent,
    summaryText: summary.stats.every((stat) => stat.damagePercent === 0)
      ? 'Base stat contribution'
      : `${summary.combinedDamagePercent.toFixed(2)}% damage`,
    stats: summary.stats.map((stat) => ({
      text: `${stat.label}: ${stat.value}${stat.statId === 'support-flat-weapon-power' ? '' : '%'}`,
      grade: stat.grade,
      color: stat.color as AccessoryRollColor
    }))
  };
}

export function getMarketDisplayName(item: MarketItem): string {
  if (item.category === 'engraving') {
    return item.item_name.replace(/\s+Engraving Recipe$/i, '');
  }
  return item.item_name;
}

export function formatGold(amount: number): string {
  if (amount >= 1000000) {
    return `${(amount / 1000000).toFixed(1)}M`;
  }
  if (amount >= 1000) {
    return `${(amount / 1000).toFixed(1)}K`;
  }
  return amount.toString();
}

export function buildHistoryChartData(historyData: HistoricalPriceEntry[]) {
  return {
    labels: historyData.map((entry) => entry.day),
    datasets: [
      {
        label: 'Min Price',
        data: historyData.map((entry) => entry.min_price),
        borderColor: '#10b981',
        backgroundColor: 'rgba(16, 185, 129, 0.1)',
        fill: true,
        tension: 0.4
      },
      {
        label: 'Avg Price',
        data: historyData.map((entry) => entry.avg_price),
        borderColor: '#f59e0b',
        backgroundColor: 'rgba(245, 158, 11, 0.1)',
        fill: true,
        tension: 0.4
      },
      {
        label: 'Max Price',
        data: historyData.map((entry) => entry.max_price),
        borderColor: '#ef4444',
        backgroundColor: 'rgba(239, 68, 68, 0.1)',
        fill: true,
        tension: 0.4
      }
    ]
  };
}

export function buildHistoryChartOptions() {
  return {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        display: true,
        position: 'top',
        labels: {
          color: '#fff',
          font: { size: 12 }
        }
      },
      tooltip: {
        backgroundColor: 'rgba(30, 30, 30, 0.95)',
        titleColor: '#fff',
        bodyColor: '#ccc',
        borderColor: '#555',
        borderWidth: 1
      }
    },
    scales: {
      x: {
        ticks: { color: '#888', font: { size: 11 } },
        grid: { color: 'rgba(255,255,255,0.06)' }
      },
      y: {
        ticks: { color: '#888', font: { size: 11 } },
        grid: { color: 'rgba(255,255,255,0.06)' }
      }
    }
  };
}
