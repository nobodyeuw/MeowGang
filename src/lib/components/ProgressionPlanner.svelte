<script lang="ts">
  import MarketPrices from './MarketPrices.svelte';
  import { createEventDispatcher } from 'svelte';

  export let activeProgressionTab: string = 'market_prices';

  const dispatch = createEventDispatcher();

  $: if (activeProgressionTab !== 'market_prices') {
    dispatch('tabChange', 'market_prices');
  }
</script>

<div class="marketplace-container">
  <MarketPrices />
</div>

<style>
  .marketplace-container {
    width: 100%;
    min-height: 100%;
  }
</style>
