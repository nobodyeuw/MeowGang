<script lang="ts">
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
  import type { HistoricalPriceEntry, HistoryDays } from './types';

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

  export let historyItemName = '';
  export let historyDays: HistoryDays = 7;
  export let historyLoading = false;
  export let historyData: HistoricalPriceEntry[] = [];
  export let chartData: any = { labels: [], datasets: [] };
  export let chartOptions: any = {};
  export let onClose: () => void;
  export let onChangeDays: (days: HistoryDays) => void;
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="ui-modal-overlay market-history-overlay" on:click={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="ui-modal-card modal-content" on:click|stopPropagation>
    <div class="ui-modal-header modal-header">
      <h3 class="ui-modal-title">{historyItemName} Price Trend</h3>
      <div class="modal-controls">
        <div class="history-range-tabs">
          <button class="range-btn" class:active={historyDays === 7} on:click={() => onChangeDays(7)}>7d</button>
          <button class="range-btn" class:active={historyDays === 14} on:click={() => onChangeDays(14)}>14d</button>
          <button class="range-btn" class:active={historyDays === 30} on:click={() => onChangeDays(30)}>30d</button>
        </div>
        <button class="ui-icon-button modal-close" on:click={onClose} aria-label="Close price history">&#10005;</button>
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

<style>
  .market-history-overlay {
    z-index: 1000;
  }

  .modal-content {
    max-width: 800px;
    width: 90%;
    max-height: 80vh;
  }

  .modal-header {
    padding: 1.25rem 1.5rem;
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
    font-size: 1.5rem;
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
</style>
