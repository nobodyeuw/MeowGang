<script lang="ts">
  import type { Character } from '$lib/store';
  import { RAIDS } from '$lib/data/raids';
  import {
    clearDashboardCalendarAssignment,
    dismissCalendarReminderToday,
    getAssignmentForEvent,
    getTodayCalendarEvents,
    saveDashboardCalendarAssignment,
    wasCalendarReminderDismissedToday,
    type DashboardCalendarAssignment,
    type DashboardCalendarEvent,
    type DashboardRaidReservation
  } from '$lib/services/dashboard-calendar';

  export let events: DashboardCalendarEvent[] = [];
  export let assignments: DashboardCalendarAssignment[] = [];
  export let reservations: DashboardRaidReservation[] = [];
  export let characters: Character[] = [];
  export let loading = false;

  let open = false;
  let reminderDismissed = false;
  let visibleWeekStart = getWednesdayOfWeek(new Date());
  let selectedDayKey: string | null = null;

  $: todayEvents = getTodayCalendarEvents(events);
  $: showTodayReminder = todayEvents.length > 0 && !reminderDismissed;
  $: calendarDays = buildCalendarDays(visibleWeekStart, events, assignments, reservations);
  $: totalCalendarItems = events.length + reservations.filter((reservation) => reservation.scheduledAt).length;
  $: weekLabel = formatWeekLabel(visibleWeekStart);
  $: filteredEvents = selectedDayKey
    ? events.filter((event) => dayKey(event.startsAt) === selectedDayKey)
    : events;
  $: filteredReservations = selectedDayKey
    ? reservations.filter((reservation) => reservation.scheduledAt && dayKey(reservation.scheduledAt) === selectedDayKey)
    : reservations;

  function assignedCharacterName(event: DashboardCalendarEvent): string {
    return assignments.find((assignment) => assignment.eventKey === event.id)?.charName || '';
  }

  async function assignCharacter(event: DashboardCalendarEvent, charIdValue: string) {
    const charId = Number(charIdValue);
    const character = characters.find((entry) => entry.char_id === charId);
    if (!character) {
      clearDashboardCalendarAssignment(event.id);
      return;
    }
    await saveDashboardCalendarAssignment(event, character);
  }

  function dismissReminder() {
    dismissCalendarReminderToday();
    reminderDismissed = true;
  }

  function getRaidName(contentId: string): string {
    return RAIDS.find((raid) => raid.id === contentId)?.name || contentId.replace(/_/g, ' ');
  }

  function getWednesdayOfWeek(date: Date): Date {
    const d = new Date(date);
    const day = d.getDay(); // 0 = Sunday, 1 = Monday, ..., 6 = Saturday
    // Calculate days to add to get to Wednesday
    // Wednesday is day 3, so: if day < 3, add (3 - day), if day > 3, add (10 - day)
    const daysToAdd = day <= 3 ? 3 - day : 10 - day;
    d.setDate(d.getDate() + daysToAdd);
    d.setHours(0, 0, 0, 0);
    return d;
  }

  function formatWeekLabel(weekStart: Date): string {
    const weekEnd = new Date(weekStart);
    weekEnd.setDate(weekEnd.getDate() + 6);
    const startStr = new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' }).format(weekStart);
    const endStr = new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric', year: 'numeric' }).format(weekEnd);
    return `${startStr} - ${endStr}`;
  }

  function startOfMonth(date: Date): Date {
    return new Date(date.getFullYear(), date.getMonth(), 1);
  }

  function dayKey(timestamp: number): string {
    const date = new Date(timestamp);
    return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
  }

  function formatTime(timestamp: number): string {
    return new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' }).format(new Date(timestamp));
  }

  function signupCalendarLabel(event: DashboardCalendarEvent, allAssignments: DashboardCalendarAssignment[]): string {
    const assignment = allAssignments.find((entry) => entry.eventKey === event.id);
    return assignment?.charName || event.title;
  }

  function buildCalendarDays(
    weekStart: Date,
    signupEvents: DashboardCalendarEvent[],
    allAssignments: DashboardCalendarAssignment[],
    allReservations: DashboardRaidReservation[]
  ) {
    const days = Array.from({ length: 7 }, (_, index) => {
      const date = new Date(weekStart);
      date.setDate(weekStart.getDate() + index);
      return {
        key: dayKey(date.getTime()),
        date,
        inMonth: true,
        isToday: dayKey(date.getTime()) === dayKey(Date.now()),
        items: [] as Array<{ id: string; time: string; label: string; kind: 'signup' | 'reservation' }>
      };
    });
    const dayMap = new Map(days.map((day) => [day.key, day]));

    for (const event of signupEvents) {
      const day = dayMap.get(dayKey(event.startsAt));
      if (!day) continue;
      day.items.push({
        id: `signup-${event.id}`,
        time: formatTime(event.startsAt),
        label: signupCalendarLabel(event, allAssignments),
        kind: 'signup'
      });
    }

    for (const reservation of allReservations) {
      if (!reservation.scheduledAt) continue;
      const day = dayMap.get(dayKey(reservation.scheduledAt));
      if (!day) continue;
      day.items.push({
        id: `reservation-${reservation.id}`,
        time: formatTime(reservation.scheduledAt),
        label: `${reservation.label || getRaidName(reservation.contentId)} - ${getRaidName(reservation.contentId)}`,
        kind: 'reservation'
      });
    }

    for (const day of days) {
      day.items.sort((a, b) => a.time.localeCompare(b.time) || a.label.localeCompare(b.label));
    }

    return days;
  }

  function moveWeek(delta: number) {
    visibleWeekStart = new Date(visibleWeekStart.getTime() + (delta * 7 * 24 * 60 * 60 * 1000));
  }

  $: if (typeof window !== 'undefined') {
    reminderDismissed = wasCalendarReminderDismissedToday();
  }
</script>

<div class="dashboard-calendar">
  <button
    type="button"
    class="calendar-toggle"
    class:has-events={events.length > 0}
    on:click={() => open = !open}
    title="Open planned MeowGang raid calendar"
  >
    <span aria-hidden="true">Cal</span>
    {#if totalCalendarItems > 0}
      <strong>{totalCalendarItems}</strong>
    {/if}
  </button>

  {#if showTodayReminder}
    <div class="calendar-reminder">
      <div>
        <strong>Planned raids today</strong>
        <span>{todayEvents.map((event) => event.sectionLabel || event.raidName).join(' | ')}</span>
      </div>
      <button type="button" on:click={dismissReminder}>Dismiss</button>
    </div>
  {/if}

  {#if open}
    <div class="calendar-popover">
      <header>
        <div>
          <strong>MeowGang Calendar</strong>
          <span>
            {loading
              ? 'Loading signups...'
              : `${filteredEvents.length} signup${filteredEvents.length === 1 ? '' : 's'} | ${filteredReservations.filter((reservation) => reservation.scheduledAt).length} reservation${filteredReservations.filter((reservation) => reservation.scheduledAt).length === 1 ? '' : 's'}`}
          </span>
        </div>
        <button type="button" on:click={() => open = false}>Close</button>
      </header>

      <div class="month-header">
        <button type="button" on:click={() => moveWeek(-1)}>Prev</button>
        <strong>{weekLabel}</strong>
        <button type="button" on:click={() => moveWeek(1)}>Next</button>
      </div>

      <div class="month-grid">
        {#each ['Wed', 'Thu', 'Fri', 'Sat', 'Sun', 'Mon', 'Tue'] as weekday}
          <span class="weekday">{weekday}</span>
        {/each}
        {#each calendarDays as day}
          <button
            type="button"
            class="calendar-day"
            class:today={day.isToday}
            class:selected={selectedDayKey === day.key}
            aria-pressed={selectedDayKey === day.key}
            on:click={() => selectedDayKey = selectedDayKey === day.key ? null : day.key}
          >
            <strong>{day.date.getDate()}</strong>
            <div class="day-items">
              {#each day.items.slice(0, 3) as item}
                <span class:item-signup={item.kind === 'signup'} class:item-reservation={item.kind === 'reservation'}>
                  <small>{item.time}</small>
                  {item.label}
                </span>
              {/each}
              {#if day.items.length > 3}
                <em>+{day.items.length - 3} more</em>
              {/if}
            </div>
          </button>
        {/each}
      </div>

      {#if filteredEvents.length > 0}
        <h4>Character Assignments</h4>
        {#if selectedDayKey}
          <div class="filter-info">
            <span>Filtered by {selectedDayKey}</span>
            <button type="button" on:click={() => selectedDayKey = null}>Clear filter</button>
          </div>
        {/if}
        <div class="event-list">
          {#each filteredEvents as event}
            <article class="calendar-event">
              <div class="event-main">
                <strong>{event.title}</strong>
                <span>{event.startsAtLabel} | {event.sectionLabel || event.raidName}</span>
                <small>{event.runType.replace('-', ' ')} | {event.role.toUpperCase()} | {event.status.replace('_', ' ')}</small>
              </div>
              <label>
                <span>Character</span>
                <select
                  value={getAssignmentForEvent(event.id)?.charId || ''}
                  on:change={(changeEvent) => assignCharacter(event, (changeEvent.currentTarget as HTMLSelectElement).value)}
                >
                  <option value="">Not assigned</option>
                  {#each characters as character}
                    <option value={character.char_id}>{character.char_name}</option>
                  {/each}
                </select>
              </label>
              {#if assignedCharacterName(event)}
                <small class="assigned">Assigned: {assignedCharacterName(event)}</small>
              {/if}
            </article>
          {/each}
        </div>
      {:else if totalCalendarItems === 0}
        <p class="empty">No active signup sheets or dated reservations found.</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .dashboard-calendar {
    position: relative;
    width: var(--dashboard-frame-width);
    margin: 0 auto 0.55rem;
    display: flex;
    justify-content: flex-end;
    z-index: 12;
  }

  .calendar-toggle {
    appearance: none;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 34%, transparent);
    background: color-mix(in srgb, var(--md-sys-color-surface-container) 86%, transparent);
    color: var(--md-sys-color-on-surface);
    border-radius: 6px;
    height: 30px;
    min-width: 46px;
    padding: 0 0.55rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.35rem;
    cursor: pointer;
    font: inherit;
    font-size: 0.7rem;
    font-weight: 600;
  }

  .calendar-toggle.has-events strong {
    min-width: 1rem;
    height: 1rem;
    border-radius: 999px;
    display: grid;
    place-items: center;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    font-size: 0.62rem;
  }

  .calendar-reminder {
    position: absolute;
    top: 34px;
    right: 0;
    width: min(28rem, calc(100vw - 2rem));
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 38%, transparent);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
    border-radius: 8px;
    box-shadow: var(--app-shadow-md);
    padding: 0.65rem;
    display: flex;
    gap: 0.65rem;
    justify-content: space-between;
    align-items: center;
  }

  .calendar-reminder div,
  .calendar-popover header div,
  .event-main {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .calendar-reminder span,
  .calendar-popover header span,
  .calendar-event small,
  .assigned {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.68rem;
  }

  .calendar-reminder button,
  .calendar-popover button {
    border: 1px solid color-mix(in srgb, var(--md-sys-color-outline) 70%, transparent);
    background: color-mix(in srgb, var(--md-sys-color-surface) 78%, transparent);
    color: inherit;
    border-radius: 5px;
    padding: 0.32rem 0.55rem;
    cursor: pointer;
    font: inherit;
    font-size: 0.68rem;
  }

  .calendar-popover {
    position: absolute;
    top: 34px;
    right: 0;
    width: min(46rem, calc(100vw - 2rem));
    max-height: min(42rem, calc(100vh - 8rem));
    overflow: auto;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 34%, transparent);
    background: var(--md-sys-color-surface-container);
    color: var(--md-sys-color-on-surface);
    border-radius: 8px;
    box-shadow: var(--app-shadow-md);
    padding: 0.75rem;
  }

  .calendar-popover header {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: center;
    margin-bottom: 0.65rem;
  }

  .empty {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.75rem;
  }

  .month-header {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.55rem;
  }

  .month-header strong {
    text-align: center;
    font-size: 0.82rem;
  }

  .month-grid {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 0.32rem;
    margin-bottom: 0.8rem;
  }

  .weekday {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.62rem;
    text-align: center;
    text-transform: uppercase;
    font-weight: 700;
  }

  .calendar-day {
    appearance: none;
    min-height: 5rem;
    min-width: 0;
    border-radius: 6px;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-outline) 42%, transparent);
    background: color-mix(in srgb, var(--md-sys-color-surface-container-high) 62%, transparent);
    padding: 0.35rem;
    display: flex;
    flex-direction: column;
    gap: 0.28rem;
    cursor: pointer;
    transition: all 0.15s ease;
    font: inherit;
    text-align: left;
  }

  .calendar-day.today {
    border-color: color-mix(in srgb, var(--md-sys-color-primary) 68%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--md-sys-color-primary) 18%, transparent);
  }

  .calendar-day.selected {
    border-color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 12%, transparent);
    box-shadow: inset 0 0 0 2px var(--md-sys-color-primary);
  }

  .calendar-day > strong {
    font-size: 0.68rem;
    line-height: 1;
  }

  .day-items {
    display: grid;
    gap: 0.18rem;
    min-width: 0;
  }

  .day-items span {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    border-radius: 4px;
    padding: 0.12rem 0.22rem;
    font-size: 0.61rem;
    line-height: 1.15;
    border: 1px solid transparent;
  }

  .day-items small {
    margin-right: 0.18rem;
    color: inherit;
    opacity: 0.72;
    font-size: 0.56rem;
  }

  .day-items em {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.58rem;
    font-style: normal;
  }

  .item-signup {
    background: color-mix(in srgb, var(--md-sys-color-primary) 14%, transparent);
    border-color: color-mix(in srgb, var(--md-sys-color-primary) 28%, transparent);
    color: var(--md-sys-color-primary);
  }

  .item-reservation {
    background: color-mix(in srgb, var(--app-color-static) 14%, transparent);
    border-color: color-mix(in srgb, var(--app-color-static) 28%, transparent);
    color: var(--app-color-on-static);
  }

  .calendar-popover h4 {
    margin: 0 0 0.45rem;
    font-size: 0.72rem;
    text-transform: uppercase;
    color: var(--md-sys-color-on-surface-variant);
  }

  .event-list {
    display: grid;
    gap: 0.5rem;
  }

  .filter-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.55rem;
    border-radius: 6px;
    background: color-mix(in srgb, var(--md-sys-color-primary) 8%, transparent);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 24%, transparent);
    font-size: 0.66rem;
    color: var(--md-sys-color-on-surface);
  }

  .filter-info button {
    padding: 0.25rem 0.5rem;
    font-size: 0.6rem;
    border-radius: 4px;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border: none;
    cursor: pointer;
  }

  .calendar-event {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(9rem, 12rem);
    gap: 0.65rem;
    align-items: center;
    padding: 0.55rem;
    border-radius: 6px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-high) 72%, transparent);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-outline) 45%, transparent);
  }

  .calendar-event label {
    display: grid;
    gap: 0.25rem;
    font-size: 0.66rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .calendar-event select {
    min-width: 0;
    height: 1.8rem;
    border-radius: 5px;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-outline) 70%, transparent);
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    font: inherit;
    font-size: 0.72rem;
  }

  .assigned {
    grid-column: 1 / -1;
  }

  @media (max-width: 760px) {
    .calendar-event {
      grid-template-columns: 1fr;
    }

    .month-grid {
      gap: 0.2rem;
    }

    .calendar-day {
      min-height: 4.2rem;
      padding: 0.24rem;
    }
  }
</style>
