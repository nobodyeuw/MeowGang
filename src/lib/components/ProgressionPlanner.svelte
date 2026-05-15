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

  interface MarketItem {
    item_slug: string;
    item_name: string;
    category: string;
    price: number;
    fetched_at: number;
    is_manual_override: boolean;
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

  type Category = 'engraving' | 'honing' | 'additional_honing' | 'gems';
  let activeCategory: Category = 'engraving';
  let gemFilter: 'all' | 't3-damage' | 't3-cooldown' | 't4-damage' | 't4-cooldown' = 'all';
  let marketItems: MarketItem[] = [];
  let loading = false;
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
    .filter(item => item.category === activeCategory)
    .filter(item => {
      if (activeCategory !== 'gems' || gemFilter === 'all') return true;
      return item.item_slug.startsWith(`gem-${gemFilter}`);
    })
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
      const apiItems = await invoke<MarketItem[]>('get_all_market_prices');
      const gemItems = await invoke<MarketItem[]>('get_gem_prices');
      marketItems = [...apiItems, ...gemItems];
      updateLastRefreshed();
    } catch (e) {
      console.error('Failed to load market prices:', e);
    } finally {
      loading = false;
    }
  }

  async function refreshPrices() {
    refreshing = true;
    try {
      const result = await invoke<RefreshResult>('refresh_market_prices');
      await loadPrices();
      needsRefresh = false;
    } catch (e) {
      console.error('Failed to refresh market prices:', e);
    } finally {
      refreshing = false;
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

  function formatGold(price: number): string {
    if (price >= 1000) {
      return price.toLocaleString();
    }
    return price.toString();
  }

  function toggleSort(key: 'name' | 'price') {
    if (sortKey === key) {
      sortAsc = !sortAsc;
    } else {
      sortKey = key;
      sortAsc = key === 'name';
    }
  }

  function startEdit(item: MarketItem) {
    editingSlug = item.item_slug;
    editPrice = item.price.toString();
  }

  async function saveManualPrice(item: MarketItem) {
    const price = parseInt(editPrice, 10);
    if (isNaN(price) || price < 0) {
      editingSlug = null;
      return;
    }

    try {
      await invoke('set_manual_market_price', {
        input: {
          item_slug: item.item_slug,
          item_name: item.item_name,
          category: item.category,
          price: price
        }
      });
      editingSlug = null;
      await loadPrices();
    } catch (e) {
      console.error('Failed to set manual price:', e);
    }
  }

  async function removeOverride(itemSlug: string) {
    try {
      await invoke('remove_manual_market_price', { itemSlug });
      await loadPrices();
    } catch (e) {
      console.error('Failed to remove override:', e);
    }
  }

  function handleEditKeydown(event: KeyboardEvent, item: MarketItem) {
    if (event.key === 'Enter') {
      saveManualPrice(item);
    } else if (event.key === 'Escape') {
      editingSlug = null;
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
      historyData = await invoke<HistoricalPriceEntry[]>('get_price_history', {
        itemSlug: historyItemSlug,
        days: historyDays
      });
      buildChartData();
    } catch (e) {
      console.error('Failed to fetch price history:', e);
      historyData = [];
      buildChartData();
    } finally {
      historyLoading = false;
    }
  }

  async function changeHistoryDays(days: 7 | 14 | 30) {
    historyDays = days;
    await fetchHistory();
  }

  function buildChartData() {
    const labels = historyData.map(d => d.day);
    chartData = {
      labels,
      datasets: [
        {
          label: 'Min Price',
          data: historyData.map(d => d.min_price),
          borderColor: '#ef4444',
          backgroundColor: 'rgba(239, 68, 68, 0.1)',
          borderWidth: 2,
          pointRadius: 3,
          tension: 0.3
        },
        {
          label: 'Max Price',
          data: historyData.map(d => d.max_price),
          borderColor: '#22c55e',
          backgroundColor: 'rgba(34, 197, 94, 0.1)',
          fill: '-1',
          borderWidth: 2,
          pointRadius: 3,
          tension: 0.3
        },
        {
          label: 'Avg Price',
          data: historyData.map(d => d.avg_price),
          borderColor: '#3b82f6',
          backgroundColor: 'rgba(59, 130, 246, 0.15)',
          borderWidth: 2.5,
          pointRadius: 3,
          tension: 0.3
        }
      ]
    };
    chartOptions = {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { mode: 'index' as const, intersect: false },
      plugins: {
        legend: {
          labels: { color: '#a0a0a0', font: { size: 12 } }
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
</script>

<div class="progression-planner">
  <div class="planner-header">
    <h2 class="planner-title">Progression Planner</h2>
    <p class="planner-subtitle">Market prices from Lost Ark (Europe Central)</p>
  </div>

  <div class="market-section">
    <div class="market-toolbar">
      <div class="category-tabs">
        <button
          class="tab-btn"
          class:active={activeCategory === 'engraving'}
          on:click={() => activeCategory = 'engraving'}
        >
          Engravings
        </button>
        <button
          class="tab-btn"
          class:active={activeCategory === 'honing'}
          on:click={() => activeCategory = 'honing'}
        >
          Honing Materials
        </button>
        <button
          class="tab-btn"
          class:active={activeCategory === 'additional_honing'}
          on:click={() => activeCategory = 'additional_honing'}
        >
          Additional Honing
        </button>
        <button
          class="tab-btn"
          class:active={activeCategory === 'gems'}
          on:click={() => activeCategory = 'gems'}
        >
          Gems
        </button>
      </div>

      {#if activeCategory === 'gems'}
        <div class="gem-filter-tabs">
          <button class="gem-filter-btn" class:active={gemFilter === 'all'} on:click={() => gemFilter = 'all'}>All</button>
          <button class="gem-filter-btn" class:active={gemFilter === 't3-damage'} on:click={() => gemFilter = 't3-damage'}>T3 Damage</button>
          <button class="gem-filter-btn" class:active={gemFilter === 't3-cooldown'} on:click={() => gemFilter = 't3-cooldown'}>T3 Cooldown</button>
          <button class="gem-filter-btn" class:active={gemFilter === 't4-damage'} on:click={() => gemFilter = 't4-damage'}>T4 Damage</button>
          <button class="gem-filter-btn" class:active={gemFilter === 't4-cooldown'} on:click={() => gemFilter = 't4-cooldown'}>T4 Cooldown</button>
        </div>
      {/if}

      <div class="toolbar-right">
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
                  <span class="sort-arrow">{sortAsc ? '&#9650;' : '&#9660;'}</span>
                {/if}
              </th>
              <th class="sortable price-col" on:click={() => toggleSort('price')}>
                Price
                {#if sortKey === 'price'}
                  <span class="sort-arrow">{sortAsc ? '&#9650;' : '&#9660;'}</span>
                {/if}
              </th>
              <th class="actions-col">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each filteredItems as item (item.item_slug)}
              <tr class:manual-override={item.is_manual_override}>
                <td class="item-name" class:clickable={item.category !== 'gems'} on:click={() => openPriceHistory(item)}>
                  {item.item_name}
                  {#if item.category !== 'gems'}
                    <span class="chart-icon" title="View price history">&#128200;</span>
                  {/if}
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
                    <span class="gold-icon">G</span>
                    {#if item.is_manual_override && activeCategory !== 'gems'}
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
                    {#if item.is_manual_override && activeCategory !== 'gems'}
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

  <div class="coming-soon-section">
    <div class="coming-soon-card">
      <span class="coming-soon-icon">&#128736;</span>
      <div class="coming-soon-text">
        <strong>Character Details & Goal Calculator</strong>
        <p>Coming soon: Set progression goals, compare to current state, and calculate gold costs.</p>
      </div>
    </div>
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
  .progression-planner {
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
    color: var(--md-sys-color-on-surface);
    margin: 0 0 0.25rem;
  }

  .planner-subtitle {
    font-size: 0.85rem;
    color: var(--md-sys-color-on-surface-variant);
    margin: 0;
  }

  .market-section {
    background: var(--md-sys-color-surface-variant);
    border-radius: 12px;
    border: 1px solid var(--md-sys-color-outline-variant);
    overflow: hidden;
  }

  .market-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--md-sys-color-outline-variant);
    gap: 1rem;
    flex-wrap: wrap;
  }

  .category-tabs {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
  }

  .tab-btn {
    padding: 0.5rem 1rem;
    border: none;
    background: transparent;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.85rem;
    font-weight: 500;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tab-btn:hover {
    background: var(--md-sys-color-surface);
  }

  .tab-btn.active {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
  }

  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .gem-filter-tabs {
    display: flex;
    gap: 0.25rem;
    padding: 0 1rem 0.5rem;
    flex-wrap: wrap;
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
    background: var(--md-sys-color-tertiary-container);
    color: var(--md-sys-color-on-tertiary-container);
    border-color: var(--md-sys-color-tertiary-container);
  }

  .search-box {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.75rem;
    background: var(--md-sys-color-surface);
    border-radius: 8px;
    border: 1px solid var(--md-sys-color-outline-variant);
  }

  .search-icon {
    font-size: 0.8rem;
    opacity: 0.6;
  }

  .search-box input {
    border: none;
    background: none;
    outline: none;
    color: var(--md-sys-color-on-surface);
    font-size: 0.8rem;
    width: 140px;
  }

  .refresh-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .refresh-time {
    font-size: 0.75rem;
    color: var(--md-sys-color-on-surface-variant);
    white-space: nowrap;
  }

  .refresh-btn {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.4rem 0.75rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    font-size: 0.8rem;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .refresh-btn:hover:not(:disabled) {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border-color: var(--md-sys-color-primary);
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .refresh-btn.primary {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border-color: var(--md-sys-color-primary);
  }

  .spinner {
    display: inline-block;
    width: 12px;
    height: 12px;
    border: 2px solid transparent;
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  .spinner.large {
    width: 24px;
    height: 24px;
    border-width: 3px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .loading-state, .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1rem;
    gap: 0.75rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .market-table-wrapper {
    overflow-x: auto;
  }

  .market-table {
    width: 100%;
    border-collapse: collapse;
  }

  .market-table thead th {
    padding: 0.6rem 1rem;
    text-align: left;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--md-sys-color-on-surface-variant);
    border-bottom: 1px solid var(--md-sys-color-outline-variant);
    user-select: none;
  }

  .market-table thead th.sortable {
    cursor: pointer;
  }

  .market-table thead th.sortable:hover {
    color: var(--md-sys-color-primary);
  }

  .sort-arrow {
    font-size: 0.65rem;
    margin-left: 0.25rem;
  }

  .price-col {
    text-align: right !important;
    width: 140px;
  }

  .actions-col {
    width: 80px;
    text-align: center !important;
  }

  .market-table tbody tr {
    border-bottom: 1px solid var(--md-sys-color-outline-variant);
    transition: background 0.1s ease;
  }

  .market-table tbody tr:hover {
    background: var(--md-sys-color-surface);
  }

  .market-table tbody tr:last-child {
    border-bottom: none;
  }

  .market-table tbody td {
    padding: 0.55rem 1rem;
    font-size: 0.85rem;
    color: var(--md-sys-color-on-surface);
  }

  .item-name {
    font-weight: 500;
  }

  .item-price {
    text-align: right;
    white-space: nowrap;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.35rem;
  }

  .gold-value {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .gold-icon {
    font-size: 0.7rem;
    font-weight: 700;
    color: #d4a017;
    background: rgba(212, 160, 23, 0.15);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
  }

  .override-badge {
    font-size: 0.6rem;
    font-weight: 700;
    color: var(--md-sys-color-tertiary);
    background: var(--md-sys-color-tertiary-container);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
  }

  .manual-override {
    background: rgba(var(--md-sys-color-tertiary-rgb, 130, 100, 200), 0.05);
  }

  .edit-price-input {
    width: 90px;
    padding: 0.25rem 0.4rem;
    border: 1px solid var(--md-sys-color-primary);
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    border-radius: 4px;
    font-size: 0.85rem;
    text-align: right;
    outline: none;
  }

  .item-actions {
    text-align: center;
    white-space: nowrap;
  }

  .action-btn {
    border: none;
    background: none;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0.2rem 0.35rem;
    border-radius: 4px;
    transition: all 0.1s ease;
    color: var(--md-sys-color-on-surface-variant);
  }

  .action-btn:hover {
    background: var(--md-sys-color-surface);
  }

  .action-btn.save {
    color: #4caf50;
  }

  .action-btn.cancel {
    color: #f44336;
  }

  .action-btn.edit:hover {
    color: var(--md-sys-color-primary);
  }

  .action-btn.remove:hover {
    color: var(--md-sys-color-error);
  }

  .table-footer {
    padding: 0.5rem 1rem;
    border-top: 1px solid var(--md-sys-color-outline-variant);
    text-align: right;
  }

  .item-count {
    font-size: 0.75rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .coming-soon-section {
    margin-top: 1.5rem;
  }

  .coming-soon-card {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1rem 1.25rem;
    background: var(--md-sys-color-surface-variant);
    border-radius: 12px;
    border: 1px dashed var(--md-sys-color-outline-variant);
  }

  .coming-soon-icon {
    font-size: 1.5rem;
    flex-shrink: 0;
  }

  .coming-soon-text strong {
    font-size: 0.9rem;
    color: var(--md-sys-color-on-surface);
  }

  .coming-soon-text p {
    font-size: 0.8rem;
    color: var(--md-sys-color-on-surface-variant);
    margin: 0.25rem 0 0;
  }

  /* Clickable item name */
  .item-name.clickable {
    cursor: pointer;
  }

  .item-name.clickable:hover {
    color: var(--md-sys-color-primary);
  }

  .chart-icon {
    font-size: 0.7rem;
    opacity: 0;
    margin-left: 0.35rem;
    transition: opacity 0.15s ease;
  }

  .item-name.clickable:hover .chart-icon {
    opacity: 0.7;
  }

  /* Modal overlay */
  :global(.modal-overlay) {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(2px);
  }

  :global(.modal-content) {
    background: var(--md-sys-color-surface-container, #1e1e1e);
    border-radius: 16px;
    border: 1px solid var(--md-sys-color-outline-variant, #444);
    padding: 1.5rem;
    width: 90%;
    max-width: 700px;
    max-height: 80vh;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  :global(.modal-header) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1rem;
  }

  :global(.modal-header h3) {
    font-size: 1rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface, #e0e0e0);
    margin: 0;
  }

  :global(.modal-controls) {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  :global(.history-range-tabs) {
    display: flex;
    gap: 0.25rem;
    background: var(--md-sys-color-surface, #121212);
    border-radius: 8px;
    padding: 0.15rem;
  }

  :global(.range-btn) {
    padding: 0.3rem 0.75rem;
    border: none;
    background: transparent;
    color: var(--md-sys-color-on-surface-variant, #aaa);
    font-size: 0.8rem;
    font-weight: 600;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  :global(.range-btn:hover) {
    background: var(--md-sys-color-surface-variant, #333);
  }

  :global(.range-btn.active) {
    background: var(--md-sys-color-primary, #6750a4);
    color: var(--md-sys-color-on-primary, #fff);
  }

  :global(.modal-close) {
    padding: 0.25rem 0.5rem;
    border: none;
    background: transparent;
    color: var(--md-sys-color-on-surface-variant, #aaa);
    font-size: 1rem;
    cursor: pointer;
    border-radius: 6px;
    transition: background 0.15s ease;
  }

  :global(.modal-close:hover) {
    background: var(--md-sys-color-surface-variant, #333);
  }

  :global(.chart-container) {
    height: 350px;
    position: relative;
  }

  :global(.chart-loading),
  :global(.chart-empty) {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--md-sys-color-on-surface-variant, #aaa);
    gap: 0.5rem;
  }

  @media (max-width: 640px) {
    .market-toolbar {
      flex-direction: column;
      align-items: stretch;
    }

    .toolbar-right {
      flex-wrap: wrap;
    }

    .search-box input {
      width: 100px;
    }

    :global(.modal-content) {
      width: 95%;
      padding: 1rem;
    }

    :global(.chart-container) {
      height: 250px;
    }
  }
</style>
