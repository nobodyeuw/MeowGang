<script lang="ts">
  import MarketPrices from './MarketPrices.svelte';
  import Planner from './Planner.svelte';
  import { createEventDispatcher } from 'svelte';

  // Props from parent
  export let activeProgressionTab: string = 'market_prices';

  const dispatch = createEventDispatcher();
  let activeTab = activeProgressionTab;

  $: if (activeProgressionTab !== activeTab) {
    activeTab = activeProgressionTab;
  }

  function switchProgressionTab(tab: string) {
    activeTab = tab;
    dispatch('tabChange', tab);
  }

  // Toggle between development mode and live mode by changing this constant.
  // Set to true to show the under-development placeholder.
  // Set to false to show the real progression planner content.
  const DEVELOPMENT_MODE = true;
</script>

{#if DEVELOPMENT_MODE && activeTab === 'planner'}
  <div class="under-development-shell">
    <div class="under-development-card">
      <div class="card-header">
        <div class="status-pill">Under Development</div>
      </div>

      <h2>Progression Planner</h2>
      <p class="subtitle">
        This section is being engineered for an overhauled, hyper-focused tracking experience.
        Here is what's coming to your MeowGang workspace soon:
      </p>

      <div class="feature-roadmap-grid">
        <div class="feature-card">
          <div class="feature-icon">💎</div>
          <div class="feature-info">
            <h3>Automated Character Details</h3>
            <p>Seamless synchronization for your Gems, Ark Grid setups, current Honing-levels and more.</p>
          </div>
        </div>

        <div class="feature-card">
          <div class="feature-icon">📈</div>
          <div class="feature-info">
            <h3>Real-Time Market Data</h3>
            <p>Live economic price updates to accurately evaluate material costs, crafting margins, and roster progression efficiency.</p>
          </div>
        </div>

        <div class="feature-card">
          <div class="feature-icon">🪙</div>
          <div class="feature-info">
            <h3>Goal Planning & Estimates</h3>
            <p>Define custom item level and gear goals for your roster and instantly receive a calculated estimation of required gold costs & recommendations for the most efficient progression path to increase a single characters combat power OR overall roster improvement.</p>
          </div>
        </div>
      </div>
    </div>
  </div>
{:else}
  <div class="progression-planner-container">
    <div class="tab-content-area">
      {#if activeTab === 'market_prices'}
        <MarketPrices />
      {/if}

      {#if activeTab === 'planner'}
        <Planner />
      {/if}
    </div>
  </div>
{/if}

<style>
  /* --- DARK ENVIRONMENT WITH ORANGE ACCENTS --- */

  .under-development-shell {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem 1.5rem;
    /* Tiefdunkler, leicht bläulich-grauer Roster-Hintergrund */
    background: linear-gradient(180deg, rgba(16, 18, 27, 0.98), rgba(10, 11, 18, 0.99));
  }

  .under-development-card {
    width: min(840px, 100%);
    max-width: 840px;
    padding: 2.5rem;
    border-radius: 20px;
    /* Semi-transparenter "Glass"-Look */
    background: rgba(22, 25, 39, 0.75);
    border: 1px solid rgba(249, 115, 22, 0.12); /* Subtiler oranger Rand */
    box-shadow: 0 40px 90px rgba(4, 5, 10, 0.6);
    backdrop-filter: blur(20px);
    color: #f8fafc;
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 1rem;
    margin-bottom: 2rem;
  }

  .under-development-card h2 {
    margin: 0 0 0.75rem;
    font-size: clamp(1.8rem, 2.5vw, 2.3rem);
    font-weight: 800;
    letter-spacing: -0.03em;
    background: linear-gradient(135deg, #ffffff 60%, #ffedd5 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
  }

  .subtitle {
    margin: 0 0 2.5rem;
    line-height: 1.6;
    color: #94a3b8;
    font-size: 1.05rem;
    max-width: 90%;
  }

  /* --- GLOSSY SHINY STATUS PILL --- */
  .status-pill {
    font-size: 0.75rem;
    font-weight: 700;
    padding: 0.4rem 0.8rem;
    border-radius: 999px;
    letter-spacing: 0.06em;
    position: relative;
    overflow: hidden;
    color: #fff;
    text-transform: uppercase;
    background: linear-gradient(135deg, #f97316 0%, #c2410c 100%); /* Orange base */
    border: 1px solid rgba(255, 255, 255, 0.25);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.35),
      0 4px 12px rgba(249, 115, 22, 0.25);
  }

  .status-pill::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: linear-gradient(135deg, rgba(255,255,255,0.3) 0%, rgba(255,255,255,0) 50%);
  }

  /* --- SUB-TABS (Orange Accent) --- */
  .progression-sub-tabs {
    display: flex;
    gap: 0.5rem;
    background: rgba(10, 11, 18, 0.5);
    padding: 0.3rem;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .progression-tab-button {
    appearance: none;
    border: none;
    background: transparent;
    color: #94a3b8;
    padding: 0.5rem 1.2rem;
    border-radius: 999px;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .progression-tab-button:hover {
    color: #f8fafc;
  }

  .progression-tab-button.active {
    background: #f97316; /* Reines Orange für den aktiven Tab */
    color: #ffffff;
    box-shadow: 0 2px 8px rgba(249, 115, 22, 0.3);
  }

  /* --- FEATURE ROADMAP GRID --- */
  .feature-roadmap-grid {
    display: grid;
    gap: 1rem;
  }

  .feature-card {
    display: flex;
    gap: 1.25rem;
    padding: 1.25rem;
    border-radius: 14px;
    background: rgba(13, 15, 24, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.03);
    transition: transform 0.2s ease, border-color 0.2s ease;
  }

  .feature-card:hover {
    transform: translateX(4px);
    border-color: rgba(249, 115, 22, 0.2); /* Sanftes Aufglühen bei Hover */
  }

  .feature-icon {
    font-size: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    border-radius: 10px;
    background: rgba(249, 115, 22, 0.1); /* Subtiler oranger Kreis im Hintergrund */
    border: 1px solid rgba(249, 115, 22, 0.15);
    flex-shrink: 0;
  }

  .feature-info h3 {
    margin: 0 0 0.35rem;
    font-size: 1.05rem;
    font-weight: 600;
    color: #f1f5f9;
  }

  .feature-info p {
    margin: 0;
    font-size: 0.92rem;
    line-height: 1.5;
    color: #64748b;
  }

  /* --- LIVE CONTENT LAYOUT --- */
  .progression-planner-container {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .tab-content-area {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: rgba(16, 18, 27, 1);
    padding: 1rem;
  }

  /* --- RESPONSIVENESS --- */
  @media (max-width: 768px) {
    .under-development-card {
      padding: 1.75rem;
    }

    .card-header {
      flex-direction: column-reverse;
      align-items: flex-start;
    }

    .status-pill {
      align-self: flex-start;
    }

    .feature-card {
      gap: 1rem;
      padding: 1rem;
    }
  }
</style>
