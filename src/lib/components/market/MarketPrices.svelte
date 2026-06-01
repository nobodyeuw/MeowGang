<script lang="ts">
  import { onMount } from 'svelte';
  import MarketPriceHistoryModal from '$lib/components/market/MarketPriceHistoryModal.svelte';
  import MarketTable from '$lib/components/market/MarketTable.svelte';
  import MarketToolbar from '$lib/components/market/MarketToolbar.svelte';
  import {
    buildHistoryChartData,
    buildHistoryChartOptions,
    doesHoningTierMatchFilter,
    getLastRefreshedLabel,
    getNextSortState
  } from '$lib/components/market/helpers';
  import type {
    GemFilter,
    HistoricalPriceEntry,
    HistoryDays,
    HoningFilter,
    MarketCategory,
    MarketItem,
    MarketSortKey
  } from '$lib/components/market/types';
  import {
    loadMarketPrices,
    loadPriceHistory,
    marketNeedsRefresh,
    refreshMarketPrices,
    removeManualMarketPrice,
    setManualMarketPrice,
    setMarketFavorite
  } from '$lib/services/market-prices';

  let activeMarketCategory: MarketCategory = 'engraving';
  let gemFilter: GemFilter = 'all';
  let honingFilter: HoningFilter = 'all';
  let showFavoritesOnly = false;
  let marketItems: MarketItem[] = [];
  let loading = false;

  let refreshing = false;
  let lastRefreshed: string = 'Never';
  let searchQuery = '';
  let sortKey: MarketSortKey = 'name';
  let sortAsc = true;
  let needsRefresh = true;
  let editingSlug: string | null = null;
  let editPrice: string = '';

  // Price history modal state
  let showHistoryModal = false;
  let historyItemName = '';
  let historyItemSlug = '';
  let historyDays: HistoryDays = 7;
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
      if (a.favorite !== b.favorite) return a.favorite ? -1 : 1;
      const mul = sortAsc ? 1 : -1;
      if (sortKey === 'name') return mul * a.item_name.localeCompare(b.item_name);
      return mul * (a.price - b.price);
    });

  $: if (activeMarketCategory === 'gems') {
    activeMarketCategory = 'engraving';
  }

  onMount(async () => {
    await checkRefreshStatus();
    await loadPrices();
    if (needsRefresh) {
      await refreshPrices();
    }
  });

  async function checkRefreshStatus() {
    try {
      needsRefresh = await marketNeedsRefresh();
    } catch (e) {
      console.error('Failed to check refresh status:', e);
    }
  }

  async function loadPrices() {
    loading = true;
    try {
      marketItems = await loadMarketPrices();
      updateLastRefreshed();
    } catch (error) {
      console.error('Failed to load market prices:', error);
    } finally {
      loading = false;
    }
  }

  function updateLastRefreshed() {
    lastRefreshed = getLastRefreshedLabel(marketItems);
  }

  async function refreshPrices() {
    refreshing = true;
    try {
      await refreshMarketPrices();
      await loadPrices();
    } catch (error) {
      console.error('Failed to refresh market prices:', error);
    } finally {
      refreshing = false;
    }
  }

  function toggleSort(key: MarketSortKey) {
    const nextSort = getNextSortState(sortKey, sortAsc, key);
    sortKey = nextSort.sortKey;
    sortAsc = nextSort.sortAsc;
  }

  async function toggleFavorite(item: MarketItem) {
    try {
      await setMarketFavorite(item.item_slug, !item.favorite);
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
      await setManualMarketPrice(item, price);
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
      await removeManualMarketPrice(itemSlug);
      await loadPrices();
    } catch (error) {
      console.error('Failed to remove manual price:', error);
    }
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
      historyData = await loadPriceHistory(historyItemSlug, historyDays);
      updateChartData();
    } catch (error) {
      console.error('Failed to fetch price history:', error);
      historyData = [];
    } finally {
      historyLoading = false;
    }
  }

  function updateChartData() {
    chartData = buildHistoryChartData(historyData);
    chartOptions = buildHistoryChartOptions();
  }

  function closeHistoryModal() {
    showHistoryModal = false;
    historyData = [];
  }

  function changeHistoryDays(days: HistoryDays) {
    historyDays = days;
    fetchHistory();
  }
</script>

<div class="market-prices-container" data-guide="marketplace">
  <div class="planner-header">
    <h2 class="planner-title">Market Prices</h2>
    <p class="planner-subtitle">Market prices from Lost Ark (Europe Central)</p>
  </div>

  <div class="market-section">
    <MarketToolbar
      bind:activeMarketCategory
      bind:gemFilter
      bind:honingFilter
      bind:showFavoritesOnly
      bind:searchQuery
      {lastRefreshed}
      {refreshing}
      onRefresh={refreshPrices}
    />

    <MarketTable
      {loading}
      {refreshing}
      marketItemsCount={marketItems.length}
      {filteredItems}
      {searchQuery}
      {sortKey}
      {sortAsc}
      bind:editingSlug
      bind:editPrice
      {activeMarketCategory}
      onRefresh={refreshPrices}
      onToggleSort={toggleSort}
      onOpenPriceHistory={openPriceHistory}
      onToggleFavorite={toggleFavorite}
      onStartEdit={startEdit}
      onSaveManualPrice={saveManualPrice}
      onHandleEditKeydown={handleEditKeydown}
      onRemoveOverride={removeOverride}
    />
  </div>
</div>
{#if showHistoryModal}
  <MarketPriceHistoryModal
    {historyItemName}
    {historyDays}
    {historyLoading}
    {historyData}
    {chartData}
    {chartOptions}
    onClose={closeHistoryModal}
    onChangeDays={changeHistoryDays}
  />
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

</style>
