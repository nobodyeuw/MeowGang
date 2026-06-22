<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onDestroy, onMount } from 'svelte';
  import { iconAsset } from '$lib/assets';
  import {
    getCurrentCalendarEventIcons,
    getCurrentCalendarEventLabel,
    getOpenCount,
    getOpenStatusKind
  } from '$lib/components/dashboard/helpers';
  import { getCurrentAvailabilityStatus, getSoonCalendarEventIds, getTimeUntilAvailable } from '$lib/utils/availability';
  import { activeFilterCharId, activeRosterId } from '$lib/store';
  import { updateTodoRosterEventStatus } from '$lib/services/todo';
  import type {
    DashboardDailyDetail,
    DashboardFocusEntry,
    DashboardRaidDetail,
    DashboardRosterFocusEntry,
    DashboardRosterEventDetail,
    DashboardWeeklyTaskDetail
  } from '$lib/components/dashboard/types';
  import type { Character } from '$lib/store';
  import type { DashboardCharacterData } from '$lib/components/dashboard/types';
  import type { DashboardCalendarEvent, DashboardCalendarAssignment, DashboardRaidReservation } from '$lib/services/dashboard-calendar';
  import DashboardCalendarWidget from '$lib/components/dashboard/DashboardCalendarWidget.svelte';

  type PopoverKind = 'raids' | 'dailies' | 'weeklies' | 'calendar' | 'gold-earners';

  export let totalRaidsCompleted = 0;
  export let totalRaidsPossible = 0;
  export let totalAdditionalRaidsCompleted = 0;
  export let totalAdditionalRaidsPossible = 0;
  export let totalDailiesCompleted = 0;
  export let totalDailiesPossible = 0;
  export let totalDailiesTracked = 0;
  export let totalWeekliesCompleted = 0;
  export let totalWeekliesPossible = 0;
  export let totalCalendarEventsCompleted = 0;
  export let totalCalendarEventsPossible = 0;
  export let goldEarnerCount = 0;
  export let visibleCharacterCount = 0;
  export let raidDetails: DashboardRaidDetail[] = [];
  export let additionalRaidDetails: DashboardRaidDetail[] = [];
  export let dailyDetails: DashboardDailyDetail[] = [];
  export let weeklyTaskDetails: DashboardWeeklyTaskDetail[] = [];
  export let calendarEventDetails: DashboardRosterEventDetail[] = [];
  export let calendarEvents: DashboardCalendarEvent[] = [];
  export let calendarAssignments: DashboardCalendarAssignment[] = [];
  export let raidReservations: DashboardRaidReservation[] = [];
  export let calendarCharacters: Character[] = [];
  export let calendarLoading = false;
  export let calendarCharacterDataMap: Record<string, DashboardCharacterData> = {};

  let activePopover: PopoverKind | null = null;
  let popoverTop = 0;
  let goldEarnerArmed = false;
  const statIcons = {
    raid: iconAsset('kazeros-raid.webp'),
    daily: iconAsset('icons8-last-24-hours-80.png'),
    weekly: iconAsset('calendar_7743808.png'),
    gold: iconAsset('gold.png'),
    gate: iconAsset('chaos_gate.png'),
    boss: iconAsset('boss.png')
  };

  $: calendarAvailability = getCurrentAvailabilityStatus();
  $: soonCalendarEventIds = !calendarAvailability.gate && !calendarAvailability.boss
    ? getSoonCalendarEventIds()
    : [];
  $: soonCalendarEventNames = soonCalendarEventIds
    .map((taskId) => taskId === 'gate' ? 'Chaos Gate' : 'Field Boss')
    .join(' | ');
  $: currentCalendarEventIcons = calendarAvailability.gate || calendarAvailability.boss
    ? getCurrentCalendarEventIcons()
    : soonCalendarEventIds.length > 0
      ? soonCalendarEventIds.map((taskId) => taskId === 'gate' ? statIcons.gate : statIcons.boss)
      : [statIcons.gate, statIcons.boss];
  $: currentCalendarEventLabel = calendarAvailability.gate || calendarAvailability.boss
    ? getCurrentCalendarEventLabel()
    : soonCalendarEventNames || 'Chaos Gate | Field Boss';

  $: showStats =
    totalRaidsPossible > 0 ||
    totalAdditionalRaidsPossible > 0 ||
    totalDailiesTracked > 0 ||
    totalWeekliesPossible > 0 ||
    goldEarnerCount > 0 ||
    visibleCharacterCount > 0;

  onMount(() => {
    document.addEventListener('click', handleOutsideClick);
  });

  onDestroy(() => {
    document.removeEventListener('click', handleOutsideClick);
  });

  function handleOutsideClick(event: MouseEvent) {
    const target = event.target as HTMLElement | null;
    if (target?.closest('.stat-card')) return;
    activePopover = null;
    goldEarnerArmed = false;
  }

  function togglePopover(kind: PopoverKind, event: MouseEvent | KeyboardEvent) {
    event.stopPropagation();
    const target = event.currentTarget as HTMLElement | null;
    if (target) {
      const rect = target.getBoundingClientRect();
      popoverTop = rect.bottom + 6;
    }

    if (kind === 'gold-earners' && activePopover === 'gold-earners' && goldEarnerArmed) {
      hideToTray();
      activePopover = null;
      goldEarnerArmed = false;
      return;
    }

    activePopover = activePopover === kind ? null : kind;
    goldEarnerArmed = kind === 'gold-earners' && activePopover === 'gold-earners';
  }

  function handleCardKeydown(kind: PopoverKind, event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      togglePopover(kind, event);
    }
  }

  async function hideToTray() {
    try {
      await getCurrentWindow().hide();
    } catch (error) {
      console.warn('Failed to hide LOA Tracker to tray:', error);
    }
  }

  function focusCharacter(entry: DashboardFocusEntry, event?: MouseEvent) {
    event?.stopPropagation();
    activeRosterId.set(entry.rosterId);
    activeFilterCharId.set(entry.charId);
    activePopover = null;
    const target = document.querySelector(`[data-dashboard-character-id="${entry.charId}"]`) as HTMLElement | null;
    const rosterTarget = document.querySelector(`[data-dashboard-roster-id="${entry.rosterId}"]`) as HTMLElement | null;
    const highlightTarget = target || rosterTarget;
    highlightTarget?.scrollIntoView({ block: 'center', inline: 'center', behavior: 'smooth' });
    if (target) {
      target.classList.add('dashboard-focus-highlight');
      window.setTimeout(() => target.classList.remove('dashboard-focus-highlight'), 2600);
    }
  }

  function focusRoster(entry: DashboardRosterFocusEntry, event?: MouseEvent) {
    event?.stopPropagation();
    activeRosterId.set(entry.rosterId);
    activePopover = null;
    const rosterTarget = document.querySelector(`[data-dashboard-roster-id="${entry.rosterId}"]`) as HTMLElement | null;
    rosterTarget?.scrollIntoView({ block: 'center', inline: 'center', behavior: 'smooth' });
    if (rosterTarget) {
      rosterTarget.classList.add('dashboard-focus-highlight');
      window.setTimeout(() => rosterTarget.classList.remove('dashboard-focus-highlight'), 2600);
    }
  }

  async function markRosterEvent(detail: DashboardRosterEventDetail, event: MouseEvent) {
    event.stopPropagation();
    await updateTodoRosterEventStatus(detail.rosterId, detail.taskId, true);
    window.dispatchEvent(new CustomEvent('roster-event-progress-updated'));
    window.dispatchEvent(new CustomEvent('todo-task-status-changed', {
      detail: {
        taskId: detail.taskId,
        rosterId: detail.rosterId,
        source: 'dashboard-label'
      }
    }));
    activePopover = null;
  }

  function openRaidEntries() {
    return [...raidDetails, ...additionalRaidDetails].filter((detail) => !detail.completed);
  }

  function uniqueFocusEntries(entries: DashboardFocusEntry[]) {
    const seen = new Set<string>();
    return entries.filter((entry) => {
      const key = `${entry.rosterId}:${entry.charId}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  }

  function openRaidCharacters() {
    const grouped = new Map<string, DashboardFocusEntry & { openCount: number }>();

    for (const entry of openRaidEntries()) {
      const key = `${entry.rosterId}:${entry.charId}`;
      const existing = grouped.get(key);
      if (existing) {
        existing.openCount += 1;
      } else {
        grouped.set(key, { ...entry, openCount: 1 });
      }
    }

    return Array.from(grouped.values());
  }

  function openWeeklyCharacters() {
    return uniqueFocusEntries(
      weeklyTaskDetails.flatMap((task) => task.openCharacters)
    );
  }

  function openWeeklyRosters() {
    const seen = new Set<string>();
    return weeklyTaskDetails
      .filter((task) => task != null)
      .flatMap((task) => (task.openRosters || []).map((roster) => ({ ...roster, taskName: task.name })))
      .filter((entry) => {
        const key = `${entry.rosterId}:${entry.taskName}`;
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });
  }

  function formatOpenDailyTasks(tasks: string[]) {
    return tasks.map((task) => task === 'chaos' ? 'Chaos' : task === 'guardian' ? 'Guardian' : task).join(' + ');
  }
</script>

{#if showStats}
  <div class="header-stats">
    {#if totalRaidsPossible > 0 || totalAdditionalRaidsPossible > 0}
      {@const displayedRaidCompleted = totalRaidsPossible > 0 ? totalRaidsCompleted : totalAdditionalRaidsCompleted}
      {@const displayedRaidPossible = totalRaidsPossible > 0 ? totalRaidsPossible : totalAdditionalRaidsPossible}
      <div class="stat-card" role="button" tabindex="0" on:click={(event) => togglePopover('raids', event)} on:keydown={(event) => handleCardKeydown('raids', event)}>
        <div class="stat-card-main">
          <div class="stat-icon"><img src={statIcons.raid} alt="Raids" /></div>
          <div class="stat-content">
            <div class="stat-status" class:done={getOpenStatusKind(displayedRaidCompleted, displayedRaidPossible) === 'done'}>
              {#if getOpenStatusKind(displayedRaidCompleted, displayedRaidPossible) === 'done'}
                <span class="stat-status-text">All done</span>
              {:else}
                <span class="stat-open-count">{getOpenCount(displayedRaidCompleted, displayedRaidPossible)}</span>
                <span class="stat-open-label">open</span>
              {/if}
            </div>
          </div>
        </div>
        <div class="stat-label">Raids</div>
        {#if activePopover === 'raids'}
          <div class="stat-popover" style={`--popover-top: ${popoverTop}px`}>
            <strong>You cleared {totalRaidsCompleted} out of {totalRaidsPossible} gold raids.</strong>
            {#if totalAdditionalRaidsPossible > 0}
              <p>+ an additional {totalAdditionalRaidsCompleted} out of {totalAdditionalRaidsPossible} raids.</p>
            {/if}
            {#if openRaidCharacters().length > 0}
              <div class="popover-list">
                {#each openRaidCharacters() as entry}
                  <button type="button" class="popover-row" on:click={(event) => focusCharacter(entry, event)}>
                    <span>{entry.charName}</span>
                    <small>{entry.openCount} open | {entry.rosterName}</small>
                  </button>
                {/each}
              </div>
            {:else}
              <p>No open raid focus needed.</p>
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    {#if totalDailiesTracked > 0}
      <div class="stat-card" role="button" tabindex="0" on:click={(event) => togglePopover('dailies', event)} on:keydown={(event) => handleCardKeydown('dailies', event)}>
        <div class="stat-card-main">
          <div class="stat-icon"><img src={statIcons.daily} alt="Dailies" /></div>
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
        {#if activePopover === 'dailies'}
          <div class="stat-popover" style={`--popover-top: ${popoverTop}px`}>
            <strong>You cleared {totalDailiesCompleted} out of {totalDailiesPossible} available dailies.</strong>
            <p>Resting lazy dailies are treated as not available today.</p>
            {#if dailyDetails.length > 0}
              <div class="popover-list">
                {#each dailyDetails as entry}
                  <button type="button" class="popover-row" on:click={(event) => focusCharacter(entry, event)}>
                    <span>{entry.charName}</span>
                    <small>{formatOpenDailyTasks(entry.openTasks)}</small>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    {#if totalWeekliesPossible > 0}
      <div class="stat-card" role="button" tabindex="0" on:click={(event) => togglePopover('weeklies', event)} on:keydown={(event) => handleCardKeydown('weeklies', event)}>
        <div class="stat-card-main">
          <div class="stat-icon"><img src={statIcons.weekly} alt="Weeklies" /></div>
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
        {#if activePopover === 'weeklies'}
          <div class="stat-popover" style={`--popover-top: ${popoverTop}px`}>
            <strong>You cleared {totalWeekliesCompleted} out of {totalWeekliesPossible} tracked weeklies.</strong>
            <div class="task-summary">
              {#each weeklyTaskDetails as task}
                {#if task}
                  <div class="task-summary-row">
                    <img src={task.icon} alt={task.name} />
                    <span>{task.completed}/{task.total}</span>
                    <small>{task.name}</small>
                  </div>
                {/if}
              {/each}
            </div>
            {#if openWeeklyCharacters().length > 0}
              <div class="popover-list">
                {#each openWeeklyCharacters() as entry}
                  <button type="button" class="popover-row" on:click={(event) => focusCharacter(entry, event)}>
                    <span>{entry.charName}</span>
                    <small>{entry.rosterName}</small>
                  </button>
                {/each}
              </div>
            {/if}
            {#if openWeeklyRosters().length > 0}
              <div class="popover-list">
                {#each openWeeklyRosters() as entry}
                  <button type="button" class="popover-row" on:click={(event) => focusRoster(entry, event)}>
                    <span>{entry.rosterName}</span>
                    <small>{entry.taskName}</small>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    <div class="stat-card calendar-event-card" role="button" tabindex="0" on:click={(event) => togglePopover('calendar', event)} on:keydown={(event) => handleCardKeydown('calendar', event)}>
      <div class="stat-card-main">
        <div class="stat-icon event-icon-stack">
          {#each currentCalendarEventIcons as icon, iconIndex}
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
              <span class="stat-status-text">{soonCalendarEventIds.length > 0 ? 'Soon' : 'Not today'}</span>
            {:else if getOpenStatusKind(totalCalendarEventsCompleted, totalCalendarEventsPossible) === 'done'}
              <span class="stat-status-text">All done</span>
            {:else}
              <span class="stat-open-count">{getOpenCount(totalCalendarEventsCompleted, totalCalendarEventsPossible)}</span>
              <span class="stat-open-label">open</span>
            {/if}
          </div>
        </div>
      </div>
      <div class="stat-label event-name">{currentCalendarEventLabel}</div>
      {#if activePopover === 'calendar'}
        <div class="stat-popover" style={`--popover-top: ${popoverTop}px`}>
          {#if totalCalendarEventsPossible <= 0}
            <strong>{soonCalendarEventIds.length > 0 ? 'Soon.' : 'Not today.'}</strong>
            <div class="popover-list">
              <div class="popover-row static-row calendar-event-row">
                <img src={statIcons.gate} alt="Chaos Gate" class="popover-task-icon" />
                <span>Chaos Gate</span>
                <small>{getTimeUntilAvailable('gate') || 'available now'}</small>
              </div>
              <div class="popover-row static-row calendar-event-row">
                <img src={statIcons.boss} alt="Field Boss" class="popover-task-icon" />
                <span>Field Boss</span>
                <small>{getTimeUntilAvailable('boss') || 'available now'}</small>
              </div>
            </div>
          {:else}
            <strong>You cleared {totalCalendarEventsCompleted} out of {totalCalendarEventsPossible} calendar events.</strong>
            <div class="popover-list">
              {#each calendarEventDetails as eventDetail}
                <div class="popover-row static-row calendar-event-row">
                  <img src={eventDetail.icon} alt={eventDetail.name} class="popover-task-icon" />
                  <span>{eventDetail.rosterName}</span>
                  <button
                    type="button"
                    class="mini-action"
                    disabled={eventDetail.completedToday || !eventDetail.available}
                    on:click={(event) => markRosterEvent(eventDetail, event)}
                  >
                    {eventDetail.completedToday ? 'Completed' : eventDetail.available ? 'Available' : 'Not today'}
                  </button>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <DashboardCalendarWidget
      events={calendarEvents}
      assignments={calendarAssignments}
      reservations={raidReservations}
      characters={calendarCharacters}
      loading={calendarLoading}
      characterDataMap={calendarCharacterDataMap}
      inline={true}
    />
  </div>
{/if}

  {#if goldEarnerCount > 0}
    <div class="stat-card" role="button" tabindex="0" on:click={(event) => togglePopover('gold-earners', event)} on:keydown={(event) => handleCardKeydown('gold-earners', event)}>
      <div class="stat-card-main">
        <div class="stat-icon"><img src={statIcons.gold} alt="Gold Earners" /></div>
        <div class="stat-content"><div class="stat-value">{goldEarnerCount}</div></div>
      </div>
      <div class="stat-label">Gold Earners</div>
      {#if activePopover === 'gold-earners'}
        <div class="stat-popover" style={`--popover-top: ${popoverTop}px`}>
          <strong>if you click me again LOA Tracker will uninstall itself</strong>
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
    position: relative;
    cursor: pointer;
  }

  .stat-card:hover,
  .stat-card:focus-visible {
    border-color: var(--app-dashboard-accent-border);
    box-shadow: var(--app-shadow-sm);
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

  .stat-popover {
    position: fixed;
    z-index: 20;
    top: var(--popover-top, 9rem);
    left: 50vw;
    transform: translateX(-50%);
    width: min(19rem, calc(100vw - 1rem));
    max-height: 22rem;
    overflow-x: hidden;
    overflow-y: auto;
    cursor: default;
    text-align: left;
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 8px;
    padding: 0.7rem;
    box-shadow: var(--app-shadow-md);
  }

  .stat-popover strong {
    display: block;
    font-size: 0.78rem;
    margin-bottom: 0.45rem;
  }

  .stat-popover p {
    margin: 0.25rem 0;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
  }

  .popover-list {
    display: grid;
    gap: 0.3rem;
    margin-top: 0.45rem;
  }

  .popover-row {
    box-sizing: border-box;
    width: 100%;
    min-width: 0;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 6px;
    background: var(--md-sys-color-surface-container);
    color: var(--md-sys-color-on-surface);
    padding: 0.36rem 0.4rem;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.28rem;
    text-align: left;
    font: inherit;
    cursor: pointer;
  }

  .popover-row:hover {
    border-color: var(--app-dashboard-accent-border);
    background: var(--app-dashboard-accent-soft);
  }

  .popover-row span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.76rem;
  }

  .popover-row small {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.66rem;
  }

  .static-row {
    cursor: default;
  }

  .calendar-event-row {
    grid-template-columns: 16px minmax(0, 1fr) auto;
  }

  .roster-event-row {
    grid-template-columns: 16px minmax(0, 1fr) auto auto;
  }

  .popover-task-icon {
    width: 15px;
    height: 15px;
    object-fit: contain;
  }

  .task-summary {
    display: grid;
    gap: 0.25rem;
  }

  .task-summary-row {
    display: grid;
    grid-template-columns: 18px auto minmax(0, 1fr);
    gap: 0.4rem;
    align-items: center;
    font-size: 0.72rem;
  }

  .task-summary-row img {
    width: 16px;
    height: 16px;
    object-fit: contain;
  }

  .task-summary-row small {
    color: var(--md-sys-color-on-surface-variant);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mini-action {
    box-sizing: border-box;
    border: 1px solid var(--app-dashboard-accent-border);
    border-radius: 5px;
    background: var(--app-dashboard-accent-soft);
    color: var(--md-sys-color-on-surface);
    font-size: 0.65rem;
    padding: 0.2rem 0.34rem;
    white-space: nowrap;
    cursor: pointer;
  }

  .mini-action:disabled {
    cursor: default;
    opacity: 0.72;
    color: var(--md-sys-color-on-surface-variant);
  }
</style>
