<script lang="ts">
  import MarketPrices from './MarketPrices.svelte';
  import Planner from './Planner.svelte';
  import { createEventDispatcher } from 'svelte';

  // Props from parent
  export let activeProgressionTab: string = 'market_prices';

  // Event dispatcher for parent communication
  const dispatch = createEventDispatcher();

  // Local state for tab content
  let activeTab = activeProgressionTab;
  
  // Update local state when parent changes
  $: if (activeProgressionTab !== activeTab) {
    activeTab = activeProgressionTab;
  }

  function switchProgressionTab(tab: string) {
    activeTab = tab;
    // Inform parent about the change
    dispatch('tabChange', tab);
  }
</script>

<div class="progression-planner-container">
  <!-- Tab Content (no sub-tabs needed anymore - they're in the header) -->
  <div class="tab-content-area">
    {#if activeTab === 'market_prices'}
      <MarketPrices />
    {/if}

    {#if activeTab === 'planner'}
      <Planner />
    {/if}
  </div>
</div>

<style>
  .progression-planner-container {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .tab-content-area {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: var(--md-sys-color-surface);
    padding: 1rem;
  }

  @media (max-width: 768px) {
    .tab-content-area {
      padding: 0.75rem;
    }
  }
</style>
