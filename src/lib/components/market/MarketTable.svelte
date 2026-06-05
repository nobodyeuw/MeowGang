<script lang="ts">
  import { iconAsset } from '$lib/assets';
  import {
    buildMarketIconUrl,
    formatGold,
    getAccessoryMarketDisplay,
    getMarketDisplayName,
    handleMarketIconError
  } from '$lib/components/market/helpers';
  import type { MarketItem, MarketSortKey } from './types';

  export let loading = false;
  export let refreshing = false;
  export let marketItemsCount = 0;
  export let filteredItems: MarketItem[] = [];
  export let searchQuery = '';
  export let sortKey: MarketSortKey = 'name';
  export let sortAsc = true;
  export let editingSlug: string | null = null;
  export let editPrice = '';
  export let onRefresh: () => void;
  export let onToggleSort: (key: MarketSortKey) => void;
  export let onOpenPriceHistory: (item: MarketItem) => void;
  export let onToggleFavorite: (item: MarketItem) => void;
  export let onStartEdit: (item: MarketItem) => void;
  export let onSaveManualPrice: (item: MarketItem) => void;
  export let onHandleEditKeydown: (event: KeyboardEvent, item: MarketItem) => void;
  export let onResetEstimatedPrice: (itemSlug: string) => void;

  const goldIcon = iconAsset('gold.png');
  export let onRemoveOverride: (itemSlug: string) => void;
</script>

{#if loading && marketItemsCount === 0}
  <div class="ui-loading-state market-state">
    <span class="spinner large"></span>
    <p>Loading market data...</p>
  </div>
{:else if filteredItems.length === 0}
  <div class="ui-empty-state market-state">
    <p>No items found{searchQuery ? ` matching "${searchQuery}"` : ''}.</p>
    {#if marketItemsCount === 0}
      <button class="refresh-btn primary" on:click={onRefresh} disabled={refreshing}>
        Fetch Market Data
      </button>
    {/if}
  </div>
{:else}
  <div class="market-table-wrapper">
    <table class="market-table">
      <thead>
        <tr>
          <th class="sortable" on:click={() => onToggleSort('name')}>
            Item Name
            {#if sortKey === 'name'}
              <span class="sort-arrow">{@html sortAsc ? '&#9650;' : '&#9660;'}</span>
            {/if}
          </th>
          <th class="sortable price-col" on:click={() => onToggleSort('price')}>
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
          {@const accessoryDisplay = getAccessoryMarketDisplay(item)}
          <tr class:manual-override={item.is_manual_override}>
            <td class="item-name" class:clickable={!item.is_manual_only} on:click={() => !item.is_manual_only && onOpenPriceHistory(item)}>
              <div class="item-name-row">
                <img
                  src={buildMarketIconUrl(item.item_slug, item.category)}
                  alt=""
                  class="item-icon"
                  on:error={handleMarketIconError}
                />
                <div class="item-name-text">
                  {#if accessoryDisplay}
                    <div class="accessory-stat-row" title={item.item_name}>
                      <img
                        src={accessoryDisplay.roleIcon}
                        alt={accessoryDisplay.roleLabel}
                        class="accessory-role-icon"
                        title={accessoryDisplay.roleLabel}
                      />
                      {#each accessoryDisplay.stats as stat, statIndex}
                        <span class={`accessory-stat grade-${stat.color}`}>{stat.text}</span>
                        {#if statIndex < accessoryDisplay.stats.length - 1}
                          <span class="accessory-stat-separator">|</span>
                        {/if}
                      {/each}
                    </div>
                    <small>{accessoryDisplay.summaryText}</small>
                  {:else}
                    <span>{getMarketDisplayName(item)}</span>
                  {/if}
                </div>
                <button
                  type="button"
                  class="favorite-star"
                  class:active={item.favorite}
                  on:click|stopPropagation={() => onToggleFavorite(item)}
                  title={item.favorite ? 'Unfavorite' : 'Favorite'}
                >
                  {#if item.favorite}&#9733;{:else}&#9734;{/if}
                </button>
                {#if !item.is_manual_only}
                  <span class="chart-icon" title="View price history">&#128202;</span>
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
                  on:keydown={(event) => onHandleEditKeydown(event, item)}
                  on:blur={() => onSaveManualPrice(item)}
                />
              {:else}
                <span class="gold-value">{formatGold(item.price)}</span>
                <img src={goldIcon} alt="" class="gold-coin-icon" />
                {#if item.is_manual_override && !item.is_manual_only}
                  <span class="override-badge" title="Manual override">M</span>
                {/if}
                {#if item.is_manual_only && item.estimated_price}
                  <span class="override-badge estimate" title={`Estimated default: ${formatGold(item.estimated_price)} gold`}>E</span>
                {/if}
              {/if}
            </td>
            <td class="item-actions">
              {#if editingSlug === item.item_slug}
                <button class="ui-button icon success" on:click={() => onSaveManualPrice(item)} title="Save">
                  &#10003;
                </button>
                <button class="ui-button icon danger" on:click={() => editingSlug = null} title="Cancel">
                  &#10005;
                </button>
              {:else}
                <button class="ui-button icon" on:click={() => onStartEdit(item)} title="Set manual price">
                  &#9998;
                </button>
                {#if item.is_manual_only && item.estimated_price}
                  <button class="ui-button icon warning" on:click={() => onResetEstimatedPrice(item.item_slug)} title="Reset to estimated price">
                    &#8635;
                  </button>
                {/if}
                {#if item.is_manual_override && !item.is_manual_only}
                  <button class="ui-button icon warning" on:click={() => onRemoveOverride(item.item_slug)} title="Remove override">
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

<style>
  .market-state {
    flex-direction: column;
    padding: 3rem 1rem;
    gap: 0.75rem;
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
    background: color-mix(in srgb, var(--md-sys-color-warning) 5%, transparent);
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

  .item-name-text small {
    display: block;
    margin-top: 0.15rem;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.7rem;
    line-height: 1.2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .accessory-stat-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    column-gap: 0.35rem;
    row-gap: 0.12rem;
    min-width: 0;
  }

  .accessory-role-icon {
    width: 18px;
    height: 18px;
    object-fit: contain;
    flex-shrink: 0;
  }

  .accessory-stat {
    font-size: 0.72rem;
    font-weight: 600;
    line-height: 1.2;
    white-space: nowrap;
  }

  .accessory-stat-separator {
    color: var(--md-sys-color-outline-variant);
    font-size: 0.72rem;
    line-height: 1.2;
  }

  .accessory-stat.grade-blue {
    color: #60a5fa;
  }

  .accessory-stat.grade-purple {
    color: #c084fc;
  }

  .accessory-stat.grade-gold {
    color: #fbbf24;
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

  .favorite-star.active,
  .favorite-star:hover {
    color: var(--md-sys-color-warning);
  }

  .chart-icon {
    font-size: 0.875rem;
    color: var(--app-market-accent);
    flex-shrink: 0;
  }

  .item-price {
    font-weight: 600;
  }

  .gold-value {
    font-size: 0.875rem;
    color: var(--app-market-value-accent);
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
    background: color-mix(in srgb, var(--md-sys-color-warning) 20%, transparent);
    color: var(--md-sys-color-warning);
    font-size: 0.65rem;
    font-weight: 600;
    border-radius: 4px;
    margin-left: 0.5rem;
  }

  .override-badge.estimate {
    background: color-mix(in srgb, var(--md-sys-color-primary) 18%, transparent);
    color: var(--md-sys-color-primary);
  }

  .item-actions {
    display: flex;
    gap: 0.4rem;
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
</style>
