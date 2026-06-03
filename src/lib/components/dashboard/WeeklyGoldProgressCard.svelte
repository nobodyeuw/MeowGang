<script lang="ts">
  import { iconAsset } from '$lib/assets';

  export let progressPercentage = 0;
  export let earnedGoldPercentage = 0;
  export let actualGoldDisplay = 0;
  export let estimatedGoldDisplay = 0;
  export let remainingGoldDisplay = 0;
  export let actualBoundGoldDisplay = 0;
  export let actualTradableGoldDisplay = 0;
  export let mismatchGoldLost = 0;
  export let mismatchGoldBonus = 0;

  const goldIcon = iconAsset('gold.png');

  $: cappedEarnedPercentage = Math.min(earnedGoldPercentage, 100);
  $: lostPercentage = estimatedGoldDisplay > 0
    ? Math.min((mismatchGoldLost / estimatedGoldDisplay) * 100, cappedEarnedPercentage)
    : 0;
  $: bonusPercentage = estimatedGoldDisplay > 0
    ? Math.min((mismatchGoldBonus / estimatedGoldDisplay) * 100, cappedEarnedPercentage)
    : 0;
</script>

<div class="gold-card-modern">
  <div class="card-glass-overlay"></div>

  <div class="card-content">
    <div class="gold-info-main">
      <div class="title-group">
        <img src={goldIcon} alt="Gold" class="gold-icon-large" />
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
        <div class="progress-fill-glow" style="width: {cappedEarnedPercentage}%"></div>
        {#if mismatchGoldLost > 0 && estimatedGoldDisplay > 0}
          <div
            class="progress-fill-lost"
            style="left: {Math.max(cappedEarnedPercentage - lostPercentage, 0)}%; width: {lostPercentage}%"
          ></div>
        {/if}
        {#if mismatchGoldBonus > 0 && estimatedGoldDisplay > 0}
          <div
            class="progress-fill-bonus"
            style="left: {Math.max(cappedEarnedPercentage - bonusPercentage, 0)}%; width: {bonusPercentage}%"
          ></div>
        {/if}
      </div>
      <div class="progress-labels">
        <span class="pct-text">{Math.round(progressPercentage)}% complete</span>
        <span class="remaining-stack">
          {#if mismatchGoldLost > 0}
            <span class="remaining-text mismatch-loss">{mismatchGoldLost.toLocaleString()} lost to difficulty mismatch</span>
          {/if}
          {#if mismatchGoldBonus > 0}
            <span class="remaining-text mismatch-bonus">+{mismatchGoldBonus.toLocaleString()} bonus from difficulty mismatch</span>
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
    border: 1px solid var(--app-dashboard-gold-panel-border);
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
    background: var(--app-dashboard-gold-panel-overlay);
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
    filter: var(--app-dashboard-gold-icon-glow);
  }

  .gold-values {
    font-size: 1.55rem;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
  }

  .gold-values .current {
    color: var(--app-dashboard-gold-number-color);
    text-shadow: var(--app-dashboard-gold-number-shadow);
  }

  .gold-values .divider { color: var(--md-sys-color-outline); margin: 0 0.25rem; }
  .gold-values .target { color: var(--md-sys-color-on-surface-variant); }
  .gold-values .unit { font-size: 0.875rem; color: var(--md-sys-color-on-surface-variant); margin-left: 0.5rem; text-transform: uppercase; }

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
    background: var(--app-dashboard-gold-progress-gradient);
    border-radius: 5px;
    position: relative;
    box-shadow: var(--app-dashboard-gold-progress-shadow);
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

  .pct-text { color: var(--app-dashboard-gold-percent-color); }
  .remaining-text { color: var(--md-sys-color-on-surface-variant); }
  .remaining-text.mismatch-loss { color: var(--md-sys-color-error); }
  .remaining-text.mismatch-bonus { color: var(--md-sys-color-success); }

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

  .progress-fill-bonus {
    position: absolute;
    top: 0;
    height: 100%;
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--md-sys-color-success) 18%, transparent),
      color-mix(in srgb, var(--md-sys-color-success) 70%, transparent)
    );
    border-radius: 3px;
    box-shadow: 0 0 10px color-mix(in srgb, var(--md-sys-color-success) 35%, transparent);
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
  .dot.tradable { background: var(--app-dashboard-gold-tradable-dot); box-shadow: var(--app-dashboard-gold-tradable-dot-shadow); }
  .detail-item .label { color: var(--md-sys-color-on-surface-variant); }
  .detail-item .val { color: var(--md-sys-color-on-surface); font-weight: 600; }
</style>
