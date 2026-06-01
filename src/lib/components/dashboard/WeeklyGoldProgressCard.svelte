<script lang="ts">
  export let progressPercentage = 0;
  export let earnedGoldPercentage = 0;
  export let actualGoldDisplay = 0;
  export let estimatedGoldDisplay = 0;
  export let remainingGoldDisplay = 0;
  export let actualBoundGoldDisplay = 0;
  export let actualTradableGoldDisplay = 0;
  export let mismatchGoldLost = 0;
</script>

<div class="gold-card-modern">
  <div class="card-glass-overlay"></div>

  <div class="card-content">
    <div class="gold-info-main">
      <div class="title-group">
        <img src="/images/gold.png" alt="Gold" class="gold-icon-large" />
        <h3>Weekly Gold Progress</h3>
      </div>

      <div class="gold-values">
        <span class="current" style={`--gold-progress: ${Math.min(progressPercentage, 100)}%`}>{actualGoldDisplay.toLocaleString()}</span>
        <span class="divider">/</span>
        <span class="target">{estimatedGoldDisplay.toLocaleString()}</span>
        <span class="unit">Gold</span>
      </div>
    </div>

    <div class="progress-container-modern">
      <div class="progress-track">
        <div class="progress-fill-glow" style="width: {Math.min(earnedGoldPercentage, 100)}%"></div>
        {#if mismatchGoldLost > 0 && estimatedGoldDisplay > 0}
          <div
            class="progress-fill-lost"
            style="left: {Math.min(earnedGoldPercentage, 100)}%; width: {Math.min(mismatchGoldLost / estimatedGoldDisplay * 100, 100 - Math.min(earnedGoldPercentage, 100))}%"
          ></div>
        {/if}
      </div>
      <div class="progress-labels">
        <span class="pct-text">{Math.round(progressPercentage)}% complete</span>
        <span class="remaining-stack">
          {#if mismatchGoldLost > 0}
            <span class="remaining-text mismatch-loss">{mismatchGoldLost.toLocaleString()} lost to difficulty mismatch</span>
          {/if}
          <span class="remaining-text">{remainingGoldDisplay.toLocaleString()} gold remaining</span>
        </span>
      </div>
    </div>

    <div class="gold-details-minimal">
      <div class="detail-item">
        <span class="dot bound"></span>
        <span class="label">Bound:</span>
        <span class="val">{actualBoundGoldDisplay.toLocaleString()}</span>
      </div>
      <div class="detail-item">
        <span class="dot tradable"></span>
        <span class="label">Tradable:</span>
        <span class="val">{actualTradableGoldDisplay.toLocaleString()}</span>
      </div>
    </div>
  </div>
</div>

<style>
  .gold-card-modern {
    position: relative;
    width: var(--dashboard-frame-width);
    box-sizing: border-box;
    background: var(--md-sys-color-surface);
    border: 1px solid color-mix(in srgb, var(--app-color-gold) 15%, transparent);
    border-radius: 14px;
    padding: 0.78rem 0.9rem;
    margin-bottom: 0.5rem;
    overflow: hidden;
    box-shadow: var(--app-shadow-md);
  }

  .card-glass-overlay {
    position: absolute;
    top: -50%;
    left: -20%;
    width: 140%;
    height: 200%;
    background: radial-gradient(circle at center, color-mix(in srgb, var(--app-color-gold) 5%, transparent) 0%, transparent 70%);
    pointer-events: none;
  }

  .card-content {
    position: relative;
    z-index: 2;
  }

  .gold-info-main {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.48rem;
  }

  .title-group {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .gold-icon-large {
    width: 28px;
    height: 28px;
    filter: drop-shadow(0 0 8px color-mix(in srgb, var(--app-color-gold) 40%, transparent));
  }

  .gold-values {
    font-size: 1.55rem;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
  }

  .gold-values .current {
    color: transparent;
    background: linear-gradient(90deg, color-mix(in srgb, var(--app-color-gold) 62%, black) 0%, var(--app-color-gold) var(--gold-progress), color-mix(in srgb, var(--app-color-gold) 64%, white) 100%);
    background-clip: text;
    -webkit-background-clip: text;
    text-shadow: 0 0 15px color-mix(in srgb, var(--app-color-gold) 25%, transparent);
  }

  .gold-values .divider { color: var(--md-sys-color-outline); margin: 0 0.25rem; }
  .gold-values .target { color: var(--md-sys-color-on-surface-variant); }
  .gold-values .unit { font-size: 0.875rem; color: var(--md-sys-color-outline-variant); margin-left: 0.5rem; text-transform: uppercase; }

  .progress-container-modern {
    margin-bottom: 0.42rem;
  }

  .progress-track {
    height: 8px;
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 5%, transparent);
    border-radius: 5px;
    overflow: hidden;
    position: relative;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-on-surface) 3%, transparent);
  }

  .progress-fill-glow {
    height: 100%;
    background: linear-gradient(90deg, color-mix(in srgb, var(--app-color-gold) 62%, black), var(--app-color-gold), color-mix(in srgb, var(--app-color-gold) 64%, white));
    border-radius: 5px;
    position: relative;
    box-shadow: 0 0 15px color-mix(in srgb, var(--app-color-gold) 40%, transparent);
    transition: width 1s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .progress-labels {
    display: flex;
    justify-content: space-between;
    margin-top: 0.34rem;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .pct-text { color: var(--app-color-gold); }
  .remaining-text { color: var(--md-sys-color-on-surface-variant); }
  .remaining-text.mismatch-loss { color: var(--md-sys-color-error); }

  .remaining-stack {
    display: inline-flex;
    flex-direction: row;
    align-items: flex-end;
    gap: 0.55rem;
    text-align: right;
  }

  .progress-fill-lost {
    position: absolute;
    top: 0;
    height: 100%;
    background: repeating-linear-gradient(
      45deg,
      color-mix(in srgb, var(--app-color-muted-state) 45%, transparent),
      color-mix(in srgb, var(--app-color-muted-state) 45%, transparent) 4px,
      color-mix(in srgb, var(--app-color-muted-state) 20%, transparent) 4px,
      color-mix(in srgb, var(--app-color-muted-state) 20%, transparent) 8px
    );
    border-radius: 0 3px 3px 0;
    transition: width 1s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .gold-details-minimal {
    display: flex;
    gap: 1rem;
    border-top: 1px solid color-mix(in srgb, var(--md-sys-color-on-surface) 5%, transparent);
    padding-top: 0.45rem;
  }

  .detail-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }

  .dot { width: 8px; height: 8px; border-radius: 50%; }
  .dot.bound { background: var(--app-color-hidden); box-shadow: 0 0 6px var(--app-color-hidden); }
  .dot.tradable { background: var(--app-color-gold); box-shadow: 0 0 6px var(--app-color-gold); }
  .detail-item .label { color: var(--md-sys-color-on-surface-variant); }
  .detail-item .val { color: var(--md-sys-color-on-surface); font-weight: 600; }
</style>
