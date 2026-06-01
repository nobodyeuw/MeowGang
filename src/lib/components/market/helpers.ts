import type { HistoricalPriceEntry, HoningFilter, MarketCategory, MarketItem, MarketSortKey } from './types';

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

const MARKET_ICON_FALLBACK = '/images/market_icons/fallback_gem.png';

export function doesHoningTierMatchFilter(slug: string, filter: HoningFilter) {
  if (filter === 'all') return true;
  if (filter === 't3') return HONING_T3_SLUGS.has(slug);
  if (filter === 't4') return HONING_T4_SLUGS.has(slug);
  if (filter === 't4.5') return HONING_T4_5_SLUGS.has(slug);
  return true;
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
      return `/images/market_icons/${tier}_${type}_gem_${level}.png`;
    }
  }

  if (category === 'engraving') {
    return '/images/market_icons/relic_book.webp';
  }

  if (category === 'honing' || category === 'additional_honing') {
    const base = slugToMarketIconBase(itemSlug);
    const extension = PNG_ONLY_MARKET_ICON_BASES.has(base) ? 'png' : 'webp';
    return `/images/market_icons/${base}.${extension}`;
  }

  return MARKET_ICON_FALLBACK;
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
