<script lang="ts">
  import MarketPrices from '$lib/components/market/MarketPrices.svelte';
</script>

<div class="marketplace-container">
  <MarketPrices />
</div>

<style>
  .marketplace-container {
    --app-control-accent: var(--app-market-accent);
    --app-control-on-accent: var(--md-sys-color-on-primary);
    --app-control-accent-container: var(--app-market-accent-soft);
    --app-control-hover-border: var(--app-market-accent);
    width: 100%;
    min-height: 100%;
  }
</style>
