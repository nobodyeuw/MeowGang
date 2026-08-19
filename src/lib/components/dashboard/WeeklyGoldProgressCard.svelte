<script lang="ts">
  import { iconAsset } from '$lib/assets';

  export let progressPercentage = 0;
  export let earnedGoldPercentage = 0;
  export let actualGoldDisplay = 0;
  export let estimatedGoldDisplay = 0;
  export let remainingGoldDisplay = 0;
  export let actualBoundGoldDisplay = 0;
  export let actualTradableGoldDisplay = 0;
  export let mismatchGoldNet = 0;
  
  const goldIcon = iconAsset('gold.png');

  $: cappedEarnedPercentage = Math.min(earnedGoldPercentage, 100);
  $: tradableGoldPercentage = estimatedGoldDisplay > 0
    ? Math.min((actualTradableGoldDisplay / estimatedGoldDisplay) * 100, 100)
    : 0;
  $: boundGoldPercentage = estimatedGoldDisplay > 0
    ? Math.min((actualBoundGoldDisplay / estimatedGoldDisplay) * 100, Math.max(100 - tradableGoldPercentage, 0))
    : 0;
  $: mismatchMagnitudePercentage = estimatedGoldDisplay > 0
    ? Math.min((Math.abs(mismatchGoldNet) / estimatedGoldDisplay) * 100, 100)
    : 0;
  $: mismatchLeftPercentage = mismatchGoldNet < 0
    ? Math.max(Math.min(cappedEarnedPercentage, 100) - mismatchMagnitudePercentage, 0)
    : Math.min(tradableGoldPercentage + boundGoldPercentage, 100);
  $: mismatchWidthPercentage = mismatchGoldNet > 0
    ? Math.min(mismatchMagnitudePercentage, Math.max(100 - mismatchLeftPercentage, 0))
    : Math.min(mismatchMagnitudePercentage, Math.max(cappedEarnedPercentage, mismatchMagnitudePercentage));
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
        <div class="progress-fill-actual">
          <div
            class="progress-fill-segment tradable"
            style="width: {tradableGoldPercentage}%"
            title="Tradable gold: {actualTradableGoldDisplay.toLocaleString()}"
          ></div>
          <div
            class="progress-fill-segment bound"
            style="width: {boundGoldPercentage}%"
            title="Bound gold: {actualBoundGoldDisplay.toLocaleString()}"
          ></div>
        </div>
        {#if mismatchGoldNet !== 0 && estimatedGoldDisplay > 0}
          <div
            class:mismatch-positive={mismatchGoldNet > 0}
            class:mismatch-negative={mismatchGoldNet < 0}
            class="progress-fill-mismatch"
            style="left: {mismatchLeftPercentage}%; width: {mismatchWidthPercentage}%"
            title="{mismatchGoldNet > 0 ? `Earned ${Math.abs(mismatchGoldNet).toLocaleString()} gold extra` : `Lost ${Math.abs(mismatchGoldNet).toLocaleString()} gold`}"
          ></div>
        {/if}
      </div>
      <div class="progress-labels">
        <span class="pct-text">{Math.round(progressPercentage)}% complete</span>
        <span class="remaining-stack">
          {#if mismatchGoldNet !== 0}
            <span class="remaining-text" style="color: {mismatchGoldNet > 0 ? 'var(--md-sys-color-success)' : 'var(--md-sys-color-error)'}">
              {mismatchGoldNet > 0 ? '+' : ''}{mismatchGoldNet.toLocaleString()} from mismatch
            </span>
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
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--app-color-gold) 4%, transparent), transparent 40%),
      var(--md-sys-color-surface);
    border: 1px solid var(--app-dashboard-gold-panel-border);
    border-radius: 14px;
    padding: 0.85rem 1rem 0.9rem;
    margin-bottom: 0.6rem;
    overflow: hidden;
    box-shadow: var(--app-shadow-md), inset 0 1px 0 0 color-mix(in srgb, var(--app-color-gold) 12%, transparent);
  }

  .gold-card-modern::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg,
      transparent,
      color-mix(in srgb, var(--app-color-gold) 75%, transparent) 30%,
      color-mix(in srgb, var(--app-color-gold) 75%, transparent) 70%,
      transparent
    );
    pointer-events: none;
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
    margin-bottom: 0.55rem;
  }

  .title-group {
    display: flex;
    align-items: center;
    gap: 0.65rem;
  }

  .title-group h3 {
    margin: 0;
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--md-sys-color-on-surface-variant);
  }

  .gold-icon-large {
    width: 26px;
    height: 26px;
    padding: 4px;
    box-sizing: border-box;
    border-radius: 8px;
    background: color-mix(in srgb, var(--app-color-gold) 14%, var(--md-sys-color-surface-container));
    border: 1px solid color-mix(in srgb, var(--app-color-gold) 30%, transparent);
    filter: var(--app-dashboard-gold-icon-glow);
  }

  .gold-values {
    font-size: 1.6rem;
    font-weight: 800;
    letter-spacing: -0.01em;
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
    margin-bottom: 0.5rem;
  }

  .progress-track {
    height: 11px;
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 6%, transparent);
    border-radius: var(--app-radius-pill);
    overflow: hidden;
    position: relative;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-on-surface) 6%, transparent);
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.35);
  }

  .progress-fill-actual {
    position: absolute;
    inset: 0;
    display: flex;
    border-radius: inherit;
    overflow: hidden;
  }

  .progress-fill-segment {
    height: 100%;
    position: relative;
    transition: width 1s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .progress-fill-segment.tradable {
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--app-color-gold) 80%, #f7f2da 20%),
      color-mix(in srgb, var(--app-color-gold) 92%, #ffffff 8%)
    );
    box-shadow: var(--app-dashboard-gold-progress-shadow);
  }

  .progress-fill-segment.bound {
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--app-color-hidden) 70%, var(--app-color-gold) 30%),
      color-mix(in srgb, var(--app-color-hidden) 86%, white 14%)
    );
  }

  .progress-fill-segment::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.28), transparent 55%);
    pointer-events: none;
  }

  .progress-fill-mismatch {
    position: absolute;
    top: 0;
    height: 100%;
    min-width: 3px;
    border-left: 1px solid rgba(255, 255, 255, 0.42);
    border-right: 1px solid rgba(0, 0, 0, 0.24);
    z-index: 2;
    transition:
      left 1s cubic-bezier(0.4, 0, 0.2, 1),
      width 1s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .progress-fill-mismatch.mismatch-positive {
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--md-sys-color-success) 45%, transparent),
      color-mix(in srgb, var(--md-sys-color-success) 86%, white 14%)
    );
    box-shadow: 0 0 10px color-mix(in srgb, var(--md-sys-color-success) 42%, transparent);
  }

  .progress-fill-mismatch.mismatch-negative {
    background: repeating-linear-gradient(
      45deg,
      color-mix(in srgb, var(--md-sys-color-error) 84%, transparent),
      color-mix(in srgb, var(--md-sys-color-error) 84%, transparent) 4px,
      color-mix(in srgb, var(--md-sys-color-error) 38%, transparent) 4px,
      color-mix(in srgb, var(--md-sys-color-error) 38%, transparent) 8px
    );
    box-shadow: 0 0 10px color-mix(in srgb, var(--md-sys-color-error) 34%, transparent);
  }

  .progress-labels {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-top: 0.4rem;
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .pct-text { color: var(--app-dashboard-gold-percent-color); }
  .remaining-text { color: var(--md-sys-color-on-surface-variant); }

  .remaining-stack {
    display: inline-flex;
    flex-direction: row;
    align-items: flex-end;
    gap: 0.55rem;
    text-align: right;
  }

  .gold-details-minimal {
    display: flex;
    gap: 1.1rem;
    border-top: 1px solid color-mix(in srgb, var(--md-sys-color-on-surface) 6%, transparent);
    padding-top: 0.5rem;
  }

  .detail-item {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }

  .dot { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }
  .dot.bound { background: var(--app-color-hidden); box-shadow: 0 0 6px var(--app-color-hidden); }
  .dot.tradable { background: var(--app-dashboard-gold-tradable-dot); box-shadow: var(--app-dashboard-gold-tradable-dot-shadow); }
  .detail-item .label { color: var(--md-sys-color-on-surface-variant); font-weight: 600; }
  .detail-item .val { color: var(--md-sys-color-on-surface); font-weight: 700; }
</style>
