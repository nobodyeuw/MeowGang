<script lang="ts">
  import type { GemFilter, HoningFilter, MarketCategory } from './types';

  export let activeMarketCategory: MarketCategory = 'engraving';
  export let gemFilter: GemFilter = 'all';
  export let honingFilter: HoningFilter = 'all';
  export let showFavoritesOnly = false;
  export let searchQuery = '';
  export let lastRefreshed = 'Never';
  export let refreshing = false;
  export let onRefresh: () => void;
</script>

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
      {#if showFavoritesOnly}&#9733; Favorites{:else}&#9734; Favorites{/if}
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
        on:click={onRefresh}
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

<style>
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

  .sub-tab-btn,
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

  .sub-tab-btn:hover,
  .gem-filter-btn:hover {
    background: var(--md-sys-color-surface);
  }

  .sub-tab-btn.active,
  .gem-filter-btn.active {
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

  .gem-filter-tabs,
  .honing-filter-tabs {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--md-sys-color-outline-variant);
    border-top-color: var(--md-sys-color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
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
