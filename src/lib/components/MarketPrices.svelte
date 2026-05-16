<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { Line } from 'svelte-chartjs';
  import {
    Chart as ChartJS,
    CategoryScale,
    LinearScale,
    PointElement,
    LineElement,
    Title,
    Tooltip,
    Legend,
    Filler
  } from 'chart.js';

  ChartJS.register(
    CategoryScale,
    LinearScale,
    PointElement,
    LineElement,
    Title,
    Tooltip,
    Legend,
    Filler
  );

  type MarketCategory = 'engraving' | 'honing' | 'additional_honing' | 'gems';

  interface MarketItem {
    item_slug: string;
    item_name: string;
    category: MarketCategory;
    price: number;
    fetched_at: number;
    is_manual_override: boolean;
    favorite: boolean;
  }

  interface RefreshResult {
    engravings_updated: number;
    honing_updated: number;
    additional_honing_updated: number;
    timestamp: number;
  }

  interface HistoricalPriceEntry {
    day: string;
    min_price: number;
    max_price: number;
    avg_price: number;
  }

  let activeMarketCategory: MarketCategory = 'engraving';
  let gemFilter: 'all' | 't3-damage' | 't3-cooldown' | 't4-damage' | 't4-cooldown' = 'all';
  let honingFilter: 'all' | 't3' | 't4' | 't4.5' = 'all';
  let showFavoritesOnly = false;
  let marketItems: MarketItem[] = [];
  let loading = false;

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
    'tailoring-decay-16-19',
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
    'destiny-shard-pouch-l',
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
    'destiny-shard-pouch-l',
  ]);

  function doesHoningTierMatchFilter(slug: string, filter: 'all' | 't3' | 't4' | 't4.5') {
    if (filter === 'all') return true;
    if (filter === 't3') return HONING_T3_SLUGS.has(slug);
    if (filter === 't4') return HONING_T4_SLUGS.has(slug);
    if (filter === 't4.5') return HONING_T4_5_SLUGS.has(slug);
    return true;
  }

  let refreshing = false;
  let lastRefreshed: string = 'Never';
  let searchQuery = '';
  let sortKey: 'name' | 'price' = 'name';
  let sortAsc = true;
  let needsRefresh = true;
  let editingSlug: string | null = null;
  let editPrice: string = '';

  // Price history modal state
  let showHistoryModal = false;
  let historyItemName = '';
  let historyItemSlug = '';
  let historyDays: 7 | 14 | 30 = 7;
  let historyLoading = false;
  let historyData: HistoricalPriceEntry[] = [];
  let chartData: any = { labels: [], datasets: [] };
  let chartOptions: any = {};

  $: filteredItems = marketItems
    .filter(item => item.category === activeMarketCategory)
    .filter(item => {
      if (activeMarketCategory === 'gems') {
        if (gemFilter === 'all') return true;
        return item.item_slug.startsWith(`gem-${gemFilter}`);
      }
      if (activeMarketCategory === 'honing' || activeMarketCategory === 'additional_honing') {
        return doesHoningTierMatchFilter(item.item_slug, honingFilter);
      }
      return true;
    })
    .filter(item => !showFavoritesOnly || item.favorite)
    .filter(item => !searchQuery || item.item_name.toLowerCase().includes(searchQuery.toLowerCase()))
    .sort((a, b) => {
      const mul = sortAsc ? 1 : -1;
      if (sortKey === 'name') return mul * a.item_name.localeCompare(b.item_name);
      return mul * (a.price - b.price);
    });

  onMount(async () => {
    await checkRefreshStatus();
    await loadPrices();
    if (needsRefresh) {
      await refreshPrices();
    }
  });

  async function checkRefreshStatus() {
    try {
      needsRefresh = await invoke<boolean>('market_needs_refresh');
    } catch (e) {
      console.error('Failed to check refresh status:', e);
    }
  }

  async function loadPrices() {
    loading = true;
    try {
      const [priceItems, gemItems] = await Promise.all([
        invoke<MarketItem[]>('get_all_market_prices'),
        invoke<MarketItem[]>('get_gem_prices')
      ]);
      marketItems = [...priceItems, ...gemItems];
      updateLastRefreshed();
    } catch (error) {
      console.error('Failed to load market prices:', error);
    } finally {
      loading = false;
    }
  }

  function updateLastRefreshed() {
    if (marketItems.length === 0) {
      lastRefreshed = 'Never';
      return;
    }
    const maxTs = Math.max(...marketItems.map(i => i.fetched_at));
    if (maxTs === 0) {
      lastRefreshed = 'Never';
      return;
    }
    const date = new Date(maxTs * 1000);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    if (diffMins < 1) lastRefreshed = 'Just now';
    else if (diffMins < 60) lastRefreshed = `${diffMins}m ago`;
    else if (diffMins < 1440) lastRefreshed = `${Math.floor(diffMins / 60)}h ago`;
    else lastRefreshed = date.toLocaleDateString();
  }

  async function refreshPrices() {
    refreshing = true;
    try {
      const result = await invoke<RefreshResult>('refresh_market_prices');
      console.log('Refresh result:', result);
      await loadPrices();
    } catch (error) {
      console.error('Failed to refresh market prices:', error);
    } finally {
      refreshing = false;
    }
  }

  function toggleSort(key: 'name' | 'price') {
    if (sortKey === key) {
      sortAsc = !sortAsc;
    } else {
      sortKey = key;
      sortAsc = true;
    }
  }

  async function toggleFavorite(item: MarketItem) {
    try {
      await invoke('set_market_favorite', {
        input: {
          item_slug: item.item_slug,
          favorite: !item.favorite
        }
      });
      item.favorite = !item.favorite;
      marketItems = [...marketItems];
    } catch (error) {
      console.error('Failed to toggle favorite:', error);
    }
  }

  function startEdit(item: MarketItem) {
    editingSlug = item.item_slug;
    editPrice = item.price.toString();
  }

  async function saveManualPrice(item: MarketItem) {
    if (editPrice === '') return;
    const price = parseFloat(editPrice);
    if (isNaN(price) || price < 0) return;

    try {
      await invoke('set_manual_price', { itemSlug: item.item_slug, price });
      await loadPrices();
    } catch (error) {
      console.error('Failed to set manual price:', error);
    } finally {
      editingSlug = null;
      editPrice = '';
    }
  }

  function handleEditKeydown(event: KeyboardEvent, item: MarketItem) {
    if (event.key === 'Enter') {
      saveManualPrice(item);
    } else if (event.key === 'Escape') {
      editingSlug = null;
      editPrice = '';
    }
  }

  async function removeOverride(itemSlug: string) {
    try {
      await invoke('remove_manual_price', { itemSlug });
      await loadPrices();
    } catch (error) {
      console.error('Failed to remove manual price:', error);
    }
  }

  const PNG_ONLY_MARKET_ICON_BASES = new Set<string>([
    'destiny_crystallized_guardian_stone',
    'destiny_crystallized_destruction_stone',
    'great_destiny_leapstone',
    'superior_abidos_fusion_material',
    'artisans_metallurgy_level_3',
    'artisans_metallurgy_level_4',
    'artisans_tailoring_level_3',
    'artisans_tailoring_level_4',
  ]);

  const MARKET_ICON_FALLBACK = '/images/market_icons/fallback_gem.png';

  function handleMarketIconError(event: Event) {
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
        const pngSrc = currentSrc.replace(/\.webp$/, '.png');
        img.dataset.triedAlternate = 'true';
        img.src = pngSrc;
        return;
      }

      if (currentSrc.endsWith('.png') && !currentSrc.includes('fallback_gem')) {
        const webpSrc = currentSrc.replace(/\.png$/, '.webp');
        img.dataset.triedAlternate = 'true';
        img.src = webpSrc;
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

  function buildMarketIconUrl(itemSlug: string, category: MarketCategory): string {
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

  function formatGold(amount: number): string {
    if (amount >= 1000000) {
      return `${(amount / 1000000).toFixed(1)}M`;
    } else if (amount >= 1000) {
      return `${(amount / 1000).toFixed(1)}K`;
    }
    return amount.toString();
  }

  async function openPriceHistory(item: MarketItem) {
    if (item.category === 'gems') return;
    historyItemName = item.item_name;
    historyItemSlug = item.item_slug;
    historyDays = 7;
    showHistoryModal = true;
    await fetchHistory();
  }

  async function fetchHistory() {
    historyLoading = true;
    try {
      const data = await invoke<HistoricalPriceEntry[]>('get_price_history', {
        itemSlug: historyItemSlug,
        days: historyDays
      });
      historyData = data;
      updateChartData();
    } catch (error) {
      console.error('Failed to fetch price history:', error);
      historyData = [];
    } finally {
      historyLoading = false;
    }
  }

  function updateChartData() {
    const labels = historyData.map(entry => entry.day);
    const minPrices = historyData.map(entry => entry.min_price);
    const maxPrices = historyData.map(entry => entry.max_price);
    const avgPrices = historyData.map(entry => entry.avg_price);

    chartData = {
      labels,
      datasets: [
        {
          label: 'Min Price',
          data: minPrices,
          borderColor: '#10b981',
          backgroundColor: 'rgba(16, 185, 129, 0.1)',
          fill: true,
          tension: 0.4
        },
        {
          label: 'Avg Price',
          data: avgPrices,
          borderColor: '#f59e0b',
          backgroundColor: 'rgba(245, 158, 11, 0.1)',
          fill: true,
          tension: 0.4
        },
        {
          label: 'Max Price',
          data: maxPrices,
          borderColor: '#ef4444',
          backgroundColor: 'rgba(239, 68, 68, 0.1)',
          fill: true,
          tension: 0.4
        }
      ]
    };

    chartOptions = {
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

  function closeHistoryModal() {
    showHistoryModal = false;
    historyData = [];
  }

  function changeHistoryDays(days: 7 | 14 | 30) {
    historyDays = days;
    fetchHistory();
  }
</script>

<div class="market-prices-container">
  <div class="planner-header">
    <h2 class="planner-title">Market Prices</h2>
    <p class="planner-subtitle">Market prices from Lost Ark (Europe Central)</p>
  </div>

  <div class="market-section">
    <div class="market-toolbar">
      <div class="toolbar-row-main">
        <div class="market-sub-tabs">
          <button
            class="sub-tab-btn"
            class:active={activeMarketCategory === 'engraving'}
            on:click={() => activeMarketCategory = 'engraving'}
          >
            Engravings
          </button>
          <button
            class="sub-tab-btn"
            class:active={activeMarketCategory === 'honing'}
            on:click={() => activeMarketCategory = 'honing'}
          >
            Honing Materials
          </button>
          <button
            class="sub-tab-btn"
            class:active={activeMarketCategory === 'additional_honing'}
            on:click={() => activeMarketCategory = 'additional_honing'}
          >
            Additional Honing
          </button>
          <button
            class="sub-tab-btn"
            class:active={activeMarketCategory === 'gems'}
            on:click={() => activeMarketCategory = 'gems'}
          >
            Gems
          </button>
        </div>

        <div class="filter-spacer"></div>

        {#if activeMarketCategory === 'gems'}
          <div class="gem-filter-tabs">
            <button class="gem-filter-btn" class:active={gemFilter === 'all'} on:click={() => gemFilter = 'all'}>All</button>
            <button class="gem-filter-btn" class:active={gemFilter === 't3-damage'} on:click={() => gemFilter = 't3-damage'}>T3 Damage</button>
            <button class="gem-filter-btn" class:active={gemFilter === 't3-cooldown'} on:click={() => gemFilter = 't3-cooldown'}>T3 Cooldown</button>
            <button class="gem-filter-btn" class:active={gemFilter === 't4-damage'} on:click={() => gemFilter = 't4-damage'}>T4 Damage</button>
            <button class="gem-filter-btn" class:active={gemFilter === 't4-cooldown'} on:click={() => gemFilter = 't4-cooldown'}>T4 Cooldown</button>
          </div>
        {/if}

        {#if activeMarketCategory === 'honing' || activeMarketCategory === 'additional_honing'}
          <div class="honing-filter-tabs">
            <button class="gem-filter-btn" class:active={honingFilter === 'all'} on:click={() => honingFilter = 'all'}>All</button>
            <button class="gem-filter-btn" class:active={honingFilter === 't3'} on:click={() => honingFilter = 't3'}>T3</button>
            <button class="gem-filter-btn" class:active={honingFilter === 't4'} on:click={() => honingFilter = 't4'}>T4</button>
            <button class="gem-filter-btn" class:active={honingFilter === 't4.5'} on:click={() => honingFilter = 't4.5'}>T4.5</button>
          </div>
        {/if}
      </div>

      <div class="toolbar-right">
        <button
          class="fav-filter-btn"
          class:active={showFavoritesOnly}
          on:click={() => showFavoritesOnly = !showFavoritesOnly}
        >
          {showFavoritesOnly ? '★ Favorites' : '☆ Favorites'}
        </button>
        <div class="search-box">
          <span class="search-icon">&#128269;</span>
          <input
            type="text"
            placeholder="Search items..."
            bind:value={searchQuery}
          />
        </div>
        <div class="refresh-info">
          <span class="refresh-time">Updated: {lastRefreshed}</span>
          <button
            class="refresh-btn"
            on:click={refreshPrices}
            disabled={refreshing}
          >
            {#if refreshing}
              <span class="spinner"></span>
            {:else}
              &#8635;
            {/if}
            Refresh
          </button>
        </div>
      </div>
    </div>

    {#if loading && marketItems.length === 0}
      <div class="loading-state">
        <span class="spinner large"></span>
        <p>Loading market data...</p>
      </div>
    {:else if filteredItems.length === 0}
      <div class="empty-state">
        <p>No items found{searchQuery ? ` matching "${searchQuery}"` : ''}.</p>
        {#if marketItems.length === 0}
          <button class="refresh-btn primary" on:click={refreshPrices} disabled={refreshing}>
            Fetch Market Data
          </button>
        {/if}
      </div>
    {:else}
      <div class="market-table-wrapper">
        <table class="market-table">
          <thead>
            <tr>
              <th class="sortable" on:click={() => toggleSort('name')}>
                Item Name
                {#if sortKey === 'name'}
                  <span class="sort-arrow">{@html sortAsc ? '&#9650;' : '&#9660;'}</span>
                {/if}
              </th>
              <th class="sortable price-col" on:click={() => toggleSort('price')}>
                Price
                {#if sortKey === 'price'}
                  <span class="sort-arrow">{@html sortAsc ? '&#9650;' : '&#9660;'}</span>
                {/if}
              </th>
              <th class="actions-col">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each filteredItems as item (item.item_slug)}
              <tr class:manual-override={item.is_manual_override}>
                <td class="item-name" class:clickable={item.category !== 'gems'} on:click={() => openPriceHistory(item)}>
                  <div class="item-name-row">
                    <img
                      src={buildMarketIconUrl(item.item_slug, item.category)}
                      alt=""
                      class="item-icon"
                      on:error={handleMarketIconError}
                    />
                    <div class="item-name-text">
                      <span>{item.item_name}</span>
                    </div>
                    <button
                      type="button"
                      class="favorite-star"
                      class:active={item.favorite}
                      on:click|stopPropagation={() => toggleFavorite(item)}
                      title={item.favorite ? 'Unfavorite' : 'Favorite'}
                    >
                      {item.favorite ? '★' : '☆'}
                    </button>
                    {#if item.category !== 'gems'}
                      <span class="chart-icon" title="View price history">📊</span>
                    {/if}
                  </div>
                </td>
                <td class="item-price">
                  {#if editingSlug === item.item_slug}
                    <input
                      class="edit-price-input"
                      type="number"
                      min="0"
                      bind:value={editPrice}
                      on:keydown={(e) => handleEditKeydown(e, item)}
                      on:blur={() => saveManualPrice(item)}
                    />
                  {:else}
                    <span class="gold-value">{formatGold(item.price)}</span>
                    <img src="/images/gold.png" alt="" class="gold-coin-icon" />
                    {#if item.is_manual_override && activeMarketCategory !== 'gems'}
                      <span class="override-badge" title="Manual override">M</span>
                    {/if}
                  {/if}
                </td>
                <td class="item-actions">
                  {#if editingSlug === item.item_slug}
                    <button class="action-btn save" on:click={() => saveManualPrice(item)} title="Save">
                      &#10003;
                    </button>
                    <button class="action-btn cancel" on:click={() => editingSlug = null} title="Cancel">
                      &#10005;
                    </button>
                  {:else}
                    <button class="action-btn edit" on:click={() => startEdit(item)} title="Set manual price">
                      &#9998;
                    </button>
                    {#if item.is_manual_override && activeMarketCategory !== 'gems'}
                      <button class="action-btn remove" on:click={() => removeOverride(item.item_slug)} title="Remove override">
                        &#8634;
                      </button>
                    {/if}
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <div class="table-footer">
        <span class="item-count">{filteredItems.length} items</span>
      </div>
    {/if}
  </div>
</div>

{#if showHistoryModal}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-overlay" on:click={closeHistoryModal}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="modal-content" on:click|stopPropagation>
      <div class="modal-header">
        <h3>{historyItemName} Price Trend</h3>
        <div class="modal-controls">
          <div class="history-range-tabs">
            <button class="range-btn" class:active={historyDays === 7} on:click={() => changeHistoryDays(7)}>7d</button>
            <button class="range-btn" class:active={historyDays === 14} on:click={() => changeHistoryDays(14)}>14d</button>
            <button class="range-btn" class:active={historyDays === 30} on:click={() => changeHistoryDays(30)}>30d</button>
          </div>
          <button class="modal-close" on:click={closeHistoryModal}>&#10005;</button>
        </div>
      </div>
      <div class="chart-container">
        {#if historyLoading}
          <div class="chart-loading">
            <span class="spinner"></span>
            <p>Loading price history...</p>
          </div>
        {:else if historyData.length === 0}
          <div class="chart-empty">
            <p>No historical data available for this item.</p>
          </div>
        {:else}
          <Line data={chartData} options={chartOptions} />
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .market-prices-container {
    padding: 1.5rem;
    max-width: 1000px;
    margin: 0 auto;
  }

  .planner-header {
    margin-bottom: 1.5rem;
  }

  .planner-title {
    font-size: 1.5rem;
    font-weight: 600;
    margin: 0 0 0.5rem 0;
    color: var(--md-sys-color-on-surface);
  }

  .planner-subtitle {
    font-size: 0.875rem;
    color: var(--md-sys-color-on-surface-variant);
    margin: 0;
  }

  .market-section {
    background: var(--md-sys-color-surface);
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 12px;
    overflow: hidden;
  }

  .market-toolbar {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1rem;
    border-bottom: 1px solid var(--md-sys-color-outline-variant);
  }

  .toolbar-row-main {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .market-sub-tabs {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .filter-spacer {
    flex: 1;
    min-width: 10px;
  }

  .sub-tab-btn {
    padding: 0.3rem 0.75rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    background: transparent;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.75rem;
    font-weight: 500;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .sub-tab-btn:hover {
    background: var(--md-sys-color-surface);
  }

  .sub-tab-btn.active {
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
    border-color: var(--md-sys-color-secondary-container);
  }

  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .fav-filter-btn {
    padding: 0.45rem 0.9rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.75rem;
    font-weight: 500;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .fav-filter-btn:hover {
    background: var(--md-sys-color-surface-variant);
  }

  .fav-filter-btn.active {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border-color: var(--md-sys-color-primary);
  }

  .search-box {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--md-sys-color-surface-variant);
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 6px;
    padding: 0.4rem 0.75rem;
    flex: 1;
    max-width: 300px;
  }

  .search-icon {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.875rem;
  }

  .search-box input {
    background: transparent;
    border: none;
    color: var(--md-sys-color-on-surface);
    font-size: 0.875rem;
    width: 100%;
    outline: none;
  }

  .search-box input::placeholder {
    color: var(--md-sys-color-on-surface-variant);
  }

  .refresh-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .refresh-time {
    font-size: 0.75rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .refresh-btn {
    padding: 0.45rem 0.9rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    font-size: 0.75rem;
    font-weight: 500;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .refresh-btn:hover:not(:disabled) {
    background: var(--md-sys-color-surface-variant);
  }

  .refresh-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .refresh-btn.primary {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border-color: var(--md-sys-color-primary);
  }

  .gem-filter-tabs,
  .honing-filter-tabs {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .gem-filter-btn {
    padding: 0.3rem 0.75rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    background: transparent;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.75rem;
    font-weight: 500;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .gem-filter-btn:hover {
    background: var(--md-sys-color-surface);
  }

  .gem-filter-btn.active {
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
    border-color: var(--md-sys-color-secondary-container);
  }

  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1rem;
    gap: 0.75rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--md-sys-color-outline-variant);
    border-top-color: var(--md-sys-color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .spinner.large {
    width: 24px;
    height: 24px;
    border-width: 3px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .market-table-wrapper {
    overflow-x: auto;
  }

  .market-table {
    width: 100%;
    border-collapse: collapse;
  }

  .market-table th {
    padding: 0.75rem 1rem;
    text-align: left;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface-variant);
    border-bottom: 1px solid var(--md-sys-color-outline-variant);
    background: var(--md-sys-color-surface-variant);
  }

  .market-table th.sortable {
    cursor: pointer;
    user-select: none;
  }

  .market-table th.sortable:hover {
    background: var(--md-sys-color-surface-container);
  }

  .sort-arrow {
    margin-left: 0.4rem;
    font-size: 0.7rem;
  }

  .market-table td {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--md-sys-color-outline-variant);
    background: var(--md-sys-color-surface);
  }

  .market-table tbody tr:hover td {
    background: var(--md-sys-color-surface-variant);
  }

  .market-table tr:last-child td {
    border-bottom: none;
  }

  .market-table tr.manual-override {
    background: rgba(255, 191, 0, 0.05);
  }

  .item-name {
    position: relative;
  }

  .item-name.clickable {
    cursor: pointer;
  }

  .item-name.clickable:hover {
    background: var(--md-sys-color-surface-variant);
  }

  .item-name-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
  }

  .item-icon {
    width: 32px;
    height: 32px;
    object-fit: contain;
    border-radius: 4px;
    flex-shrink: 0;
  }

  .item-name-text {
    flex: 1;
    min-width: 0;
  }

  .item-name-text span {
    font-size: 0.875rem;
    color: var(--md-sys-color-on-surface);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
  }

  .favorite-star {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1rem;
    color: var(--md-sys-color-on-surface-variant);
    padding: 0.2rem 0.4rem;
    transition: color 0.15s ease;
    flex-shrink: 0;
  }

  .favorite-star.active {
    color: #fbbf24;
  }

  .favorite-star:hover {
    color: #fbbf24;
  }

  .chart-icon {
    font-size: 0.875rem;
    color: var(--md-sys-color-primary);
    flex-shrink: 0;
  }

  .item-price {
    font-weight: 600;
  }

  .gold-value {
    font-size: 0.875rem;
    color: var(--md-sys-color-on-surface);
  }

  .gold-coin-icon {
    width: 16px;
    height: 16px;
    margin-left: 0.4rem;
    vertical-align: middle;
  }

  .override-badge {
    display: inline-block;
    padding: 0.15rem 0.4rem;
    background: rgba(255, 191, 0, 0.2);
    color: #fbbf24;
    font-size: 0.65rem;
    font-weight: 600;
    border-radius: 4px;
    margin-left: 0.5rem;
  }

  .item-actions {
    display: flex;
    gap: 0.4rem;
  }

  .action-btn {
    padding: 0.35rem 0.6rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    font-size: 0.875rem;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover {
    background: var(--md-sys-color-surface-variant);
  }

  .action-btn.save {
    border-color: #10b981;
    color: #10b981;
  }

  .action-btn.save:hover {
    background: rgba(16, 185, 129, 0.1);
  }

  .action-btn.cancel {
    border-color: #ef4444;
    color: #ef4444;
  }

  .action-btn.cancel:hover {
    background: rgba(239, 68, 68, 0.1);
  }

  .action-btn.remove {
    border-color: #f59e0b;
    color: #f59e0b;
  }

  .action-btn.remove:hover {
    background: rgba(245, 158, 11, 0.1);
  }

  .edit-price-input {
    width: 80px;
    padding: 0.35rem 0.5rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    font-size: 0.875rem;
    border-radius: 4px;
  }

  .edit-price-input:focus {
    outline: none;
    border-color: var(--md-sys-color-primary);
  }

  .table-footer {
    padding: 0.75rem 1rem;
    font-size: 0.75rem;
    color: var(--md-sys-color-on-surface-variant);
    background: var(--md-sys-color-surface-variant);
  }

  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(4px);
  }

  .modal-content {
    background: var(--md-sys-color-surface);
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 12px;
    max-width: 800px;
    width: 90%;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--md-sys-color-outline-variant);
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
  }

  .modal-controls {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .history-range-tabs {
    display: flex;
    gap: 0.4rem;
  }

  .range-btn {
    padding: 0.4rem 0.8rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    background: transparent;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.75rem;
    font-weight: 500;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .range-btn:hover {
    background: var(--md-sys-color-surface);
  }

  .range-btn.active {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border-color: var(--md-sys-color-primary);
  }

  .modal-close {
    background: none;
    border: none;
    font-size: 1.5rem;
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
    padding: 0;
    line-height: 1;
    transition: color 0.15s ease;
  }

  .modal-close:hover {
    color: var(--md-sys-color-on-surface);
  }

  .chart-container {
    padding: 1.5rem;
    min-height: 300px;
  }

  .chart-loading,
  .chart-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 300px;
    gap: 0.75rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  @media (max-width: 768px) {
    .market-toolbar {
      flex-direction: column;
      gap: 0.75rem;
    }

    .toolbar-right {
      flex-direction: column;
      align-items: stretch;
    }

    .search-box {
      max-width: 100%;
    }

    .refresh-info {
      flex-direction: column;
      align-items: stretch;
      gap: 0.5rem;
    }

    .market-sub-tabs {
      flex-wrap: wrap;
    }
  }
</style>
