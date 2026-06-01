<script lang="ts">
  import {
    getCurrentCalendarEventIcons,
    getCurrentCalendarEventLabel,
    getOpenCount,
    getOpenStatusKind
  } from '$lib/components/dashboard/helpers';
  import type { ArgeosStatusKind } from '$lib/components/dashboard/types';

  export let totalRaidsCompleted = 0;
  export let totalRaidsPossible = 0;
  export let totalDailiesCompleted = 0;
  export let totalDailiesPossible = 0;
  export let totalDailiesTracked = 0;
  export let totalWeekliesCompleted = 0;
  export let totalWeekliesPossible = 0;
  export let totalCalendarEventsCompleted = 0;
  export let totalCalendarEventsPossible = 0;
  export let totalArgeosTracked = 0;
  export let totalArgeosAvailableToday = 0;
  export let goldEarnerCount = 0;
  export let visibleCharacterCount = 0;
  export let argeosStatusKind: ArgeosStatusKind = 'empty';

  $: showStats =
    totalRaidsPossible > 0 ||
    totalDailiesTracked > 0 ||
    totalWeekliesPossible > 0 ||
    totalArgeosTracked > 0 ||
    goldEarnerCount > 0 ||
    visibleCharacterCount > 0;
</script>

{#if showStats}
  <div class="header-stats">
    {#if totalRaidsPossible > 0}
      <div class="stat-card">
        <div class="stat-card-main">
          <div class="stat-icon">
            <img src="/images/kazeros-raid.webp" alt="Raids" />
          </div>
          <div class="stat-content">
            <div class="stat-status" class:done={getOpenStatusKind(totalRaidsCompleted, totalRaidsPossible) === 'done'}>
              {#if getOpenStatusKind(totalRaidsCompleted, totalRaidsPossible) === 'done'}
                <span class="stat-status-text">All done</span>
              {:else}
                <span class="stat-open-count">{getOpenCount(totalRaidsCompleted, totalRaidsPossible)}</span>
                <span class="stat-open-label">open</span>
              {/if}
            </div>
          </div>
        </div>
        <div class="stat-label">Raids</div>
      </div>
    {/if}

    {#if totalDailiesTracked > 0}
      <div class="stat-card">
        <div class="stat-card-main">
          <div class="stat-icon">
            <img src="/images/icons8-last-24-hours-80.png" alt="Dailies" />
          </div>
          <div class="stat-content">
            <div
              class="stat-status"
              class:done={getOpenStatusKind(totalDailiesCompleted, totalDailiesPossible, totalDailiesTracked) === 'done'}
              class:idle={getOpenStatusKind(totalDailiesCompleted, totalDailiesPossible, totalDailiesTracked) === 'idle'}
            >
              {#if getOpenStatusKind(totalDailiesCompleted, totalDailiesPossible, totalDailiesTracked) === 'idle'}
                <span class="stat-status-text">Resting</span>
              {:else if getOpenStatusKind(totalDailiesCompleted, totalDailiesPossible, totalDailiesTracked) === 'done'}
                <span class="stat-status-text">All done</span>
              {:else}
                <span class="stat-open-count">{getOpenCount(totalDailiesCompleted, totalDailiesPossible)}</span>
                <span class="stat-open-label">open</span>
              {/if}
            </div>
          </div>
        </div>
        <div class="stat-label">Dailies</div>
      </div>
    {/if}

    {#if totalWeekliesPossible > 0}
      <div class="stat-card">
        <div class="stat-card-main">
          <div class="stat-icon">
            <img src="images/calendar_7743808.png" alt="Weeklies" />
          </div>
          <div class="stat-content">
            <div class="stat-status" class:done={getOpenStatusKind(totalWeekliesCompleted, totalWeekliesPossible) === 'done'}>
              {#if getOpenStatusKind(totalWeekliesCompleted, totalWeekliesPossible) === 'done'}
                <span class="stat-status-text">All done</span>
              {:else}
                <span class="stat-open-count">{getOpenCount(totalWeekliesCompleted, totalWeekliesPossible)}</span>
                <span class="stat-open-label">open</span>
              {/if}
            </div>
          </div>
        </div>
        <div class="stat-label">Weeklies</div>
      </div>
    {/if}

    <div class="stat-card calendar-event-card">
      <div class="stat-card-main">
        <div class="stat-icon event-icon-stack">
          {#each getCurrentCalendarEventIcons() as icon, iconIndex}
            <img src={icon} alt="Calendar Event" style={`--event-icon-index: ${iconIndex}`} />
          {/each}
        </div>
        <div class="stat-content">
          <div
            class="stat-status"
            class:done={getOpenStatusKind(totalCalendarEventsCompleted, totalCalendarEventsPossible) === 'done'}
            class:empty={getOpenStatusKind(totalCalendarEventsCompleted, totalCalendarEventsPossible) === 'empty'}
          >
            {#if getOpenStatusKind(totalCalendarEventsCompleted, totalCalendarEventsPossible) === 'empty'}
              <span class="stat-status-text">No event</span>
            {:else if getOpenStatusKind(totalCalendarEventsCompleted, totalCalendarEventsPossible) === 'done'}
              <span class="stat-status-text">All done</span>
            {:else}
              <span class="stat-open-count">{getOpenCount(totalCalendarEventsCompleted, totalCalendarEventsPossible)}</span>
              <span class="stat-open-label">open</span>
            {/if}
          </div>
        </div>
      </div>
      <div class="stat-label event-name">{getCurrentCalendarEventLabel()}</div>
    </div>

    {#if totalArgeosTracked > 0}
      <div class="stat-card">
        <div class="stat-card-main">
          <div class="stat-icon">
            <img src="/images/event_quest.webp" alt="Stoopid Argeos" />
          </div>
          <div class="stat-content">
            <div
              class="stat-status"
              class:done={argeosStatusKind === 'done'}
              class:idle={argeosStatusKind === 'today'}
            >
              {#if argeosStatusKind === 'done'}
                <span class="stat-status-text">Fully done</span>
              {:else if argeosStatusKind === 'today'}
                <span class="stat-status-text">Done today</span>
              {:else if argeosStatusKind === 'open'}
                <span class="stat-open-count">{totalArgeosAvailableToday}</span>
                <span class="stat-open-label">open</span>
              {:else}
                <span class="stat-status-text">Not tracked</span>
              {/if}
            </div>
          </div>
        </div>
        <div class="stat-label event-name">Stoopid Argeos</div>
      </div>
    {/if}

    {#if goldEarnerCount > 0}
      <div class="stat-card">
        <div class="stat-card-main">
          <div class="stat-icon">
            <img src="/images/gold.png" alt="Gold Earners" />
          </div>
          <div class="stat-content">
            <div class="stat-value">{goldEarnerCount}</div>
          </div>
        </div>
        <div class="stat-label">Gold Earners</div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .header-stats {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.38rem;
    width: var(--dashboard-frame-width);
    box-sizing: border-box;
    margin-bottom: 0.5rem;
  }

  .stat-card {
    flex: 0 1 138px;
    min-width: 118px;
    max-width: 156px;
    box-sizing: border-box;
    background: var(--surface-variant);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 25%, transparent);
    border-radius: 8px;
    padding: 0.38rem 0.48rem 0.34rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.14rem;
  }

  .stat-card-main {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr) 22px;
    align-items: center;
    column-gap: 0.12rem;
    width: 100%;
    min-width: 0;
  }

  .stat-card-main::after {
    content: "";
    width: 22px;
    height: 1px;
  }

  .stat-icon {
    width: 22px;
    height: 22px;
    flex: 0 0 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--primary);
    border-radius: 8px;
  }

  .stat-icon img {
    width: 15px;
    height: 15px;
    object-fit: contain;
  }

  .event-icon-stack {
    position: relative;
  }

  .event-icon-stack img {
    position: absolute;
    left: calc(var(--event-icon-index, 0) * 7px);
    width: 16px;
    height: 16px;
    filter: drop-shadow(0 1px 2px color-mix(in srgb, black 28%, transparent));
  }

  .stat-content {
    flex: 1;
    min-width: 0;
    text-align: center;
    display: flex;
    justify-content: center;
  }

  .stat-value {
    font-size: clamp(1rem, 1.4vw, 1.25rem);
    font-weight: 700;
    color: var(--on-surface);
    line-height: 1;
    white-space: nowrap;
  }

  .stat-status {
    min-height: 1.12rem;
    display: inline-flex;
    max-width: 100%;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
    line-height: 1;
    color: var(--on-surface);
    white-space: nowrap;
  }

  .stat-open-count {
    font-size: 0.88rem;
    font-weight: 800;
    color: var(--on-surface);
  }

  .stat-open-label {
    color: var(--on-surface-variant);
    font-size: 0.58rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0;
  }

  .stat-status.done,
  .stat-status.idle,
  .stat-status.empty {
    min-width: 0;
    max-width: 100%;
  }

  .stat-status.done .stat-status-text {
    color: color-mix(in srgb, var(--md-sys-color-success) 72%, var(--on-surface));
  }

  .stat-status.idle .stat-status-text,
  .stat-status.empty .stat-status-text {
    color: var(--on-surface-variant);
  }

  .stat-status-text {
    color: var(--on-surface);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 0.61rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0;
  }

  .stat-label {
    align-self: stretch;
    font-size: 0.54rem;
    color: var(--on-surface-variant);
    margin-top: 0;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1;
  }

  .stat-label.event-name {
    font-size: 0.54rem;
  }
</style>
