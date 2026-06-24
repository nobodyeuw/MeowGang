<script lang="ts">
  import type { Character } from '$lib/store';
  import { RAIDS } from '$lib/data/raids';
  import { appAsset } from '$lib/assets';
  import {
    clearDashboardCalendarAssignment,
    dismissCalendarReminderToday,
    dispatchCalendarChanged,
    formatReservationScheduleLabel,
    getAssignmentForEvent,
    getReservationOccurrenceTimestamp,
    getTodayCalendarEvents,
    reservationMatchesDayKey,
    saveDashboardCalendarAssignment,
    wasCalendarReminderDismissedToday,
    type DashboardCalendarAssignment,
    type DashboardCalendarEvent,
    type DashboardRaidReservation
  } from '$lib/services/dashboard-calendar';
  import type { DashboardCharacterData } from '$lib/components/dashboard/types';

  const calendarIcon = appAsset('calendar.png');

  export let events: DashboardCalendarEvent[] = [];
  export let assignments: DashboardCalendarAssignment[] = [];
  export let reservations: DashboardRaidReservation[] = [];
  export let characters: Character[] = [];
  export let loading = false;
  export let characterDataMap: Record<string, DashboardCharacterData> = {};
  export let inline = false; // If true, widget sits inline with other elements

  let open = false;
  let reminderDismissed = false;
  let visibleWeekStart = getWednesdayOfWeek(new Date());
  let selectedDayKey: string | null = null;
  let expandedRaidTrains = new Set<string>();

  $: todayEvents = getTodayCalendarEvents(events);
  $: showTodayReminder = todayEvents.length > 0 && !reminderDismissed;
  $: calendarDays = buildCalendarDays(visibleWeekStart, events, assignments, reservations);
  $: totalCalendarItems = events.filter((event) => !(event as any).isChildOfRaidTrain).length + reservations.filter((reservation) => reservation.scheduledAt).length;
  $: weekLabel = formatWeekLabel(visibleWeekStart);
  $: filteredEvents = selectedDayKey
    ? events.filter((event) => dayKey(event.startsAt) === selectedDayKey && !(event as any).isChildOfRaidTrain)
    : events.filter((event) => !(event as any).isChildOfRaidTrain);
  $: filteredReservations = reservations.filter((reservation) =>
    reservationMatchesDayKey(reservation, selectedDayKey || '', visibleWeekStart)
  );
  $: datedReservations = filteredReservations.filter((reservation) => reservation.scheduledAt);
  $: undatedReservations = filteredReservations.filter((reservation) => !reservation.scheduledAt);
  $: widthVar = inline ? '' : ' dashboard-calendar-full-width';

  function assignedCharacterName(event: DashboardCalendarEvent): string {
    return assignments.find((assignment) => assignment.eventKey === event.id)?.charName || '';
  }

  function toggleRaidTrainExpansion(eventId: string) {
    if (expandedRaidTrains.has(eventId)) {
      expandedRaidTrains.delete(eventId);
    } else {
      expandedRaidTrains.add(eventId);
    }
    expandedRaidTrains = new Set(expandedRaidTrains); // Trigger reactivity
  }

  function handleRaidTrainKeydown(eventId: string, event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      toggleRaidTrainExpansion(eventId);
    }
  }

  function isRaidTrain(event: DashboardCalendarEvent): boolean {
    return event.runType === 'raid-train' && !(event as any).isChildOfRaidTrain;
  }

  function getChildEvents(event: DashboardCalendarEvent): DashboardCalendarEvent[] {
    // First check if child events are stored as a property on the parent
    if ((event as any).childEvents) {
      return (event as any).childEvents;
    }
    // Otherwise find child events from the main events array
    return events.filter(e => (e as any).parentEventId === event.id);
  }

  function extractContentIdFromRaidName(raidName: string): string | null {
    // Match raid name to content_id using the RAIDS data
    const raid = RAIDS.find((r) => r.name.toLowerCase() === raidName.toLowerCase());
    if (raid) return raid.id;

    // Try to extract from common patterns
    const lowerName = raidName.toLowerCase();
    if (lowerName.includes('echidna')) return 'overture_echidna';
    if (lowerName.includes('behemoth')) return 'behemoth';
    if (lowerName.includes('aegir')) return 'act_1_aegir';
    if (lowerName.includes('brelshaza')) return 'act_2_brelshaza';
    if (lowerName.includes('mordum')) return 'act_3_mordum';
    if (lowerName.includes('armoche')) return 'act_4_armoche';
    if (lowerName.includes('kayangel')) return 'act_4_kayangel';
    if (lowerName.includes('kakul')) return 'act_4_kakul_saydon';
    if (lowerName.includes('kazeros')) return 'denouement_final_day';
    if (lowerName.includes('serca')) return 'shadow_serca';
    if (lowerName.includes('cathedral')) return 'horizon_cathedral';

    return null;
  }

  function isRaidCompletedForCharacter(charId: number, contentId: string): boolean {
    const charData = characterDataMap[String(charId)];
    if (!charData) return false;
    return charData.completionStatus.some(
      (entry) => entry.content_id === contentId && entry.is_completed === 1
    );
  }

  function getAvailableCharactersForEvent(event: DashboardCalendarEvent): Character[] {
    // For custom raids, show all characters with tracked raids available
    if (event.raidName.toLowerCase().includes('custom')) {
      return characters.filter((character) => {
        const charData = characterDataMap[String(character.char_id)];
        if (!charData) return false;
        return charData.trackingStatus.some((entry) => entry.is_tracked === 1);
      });
    }

    // For raid-train child events, use the child's raid name
    if (event.sectionCode) {
      const contentId = extractContentIdFromRaidName(event.raidName);
      if (contentId) {
        return characters.filter((character) =>
          !isRaidCompletedForCharacter(character.char_id, contentId) &&
          !isCharacterAssignedToRaidInCycle(character.char_id, contentId, event.id)
        );
      }
    }

    // For regular raids, filter by completion status using content_id matching
    const contentId = extractContentIdFromRaidName(event.raidName);
    if (contentId) {
      return characters.filter((character) =>
        !isRaidCompletedForCharacter(character.char_id, contentId) &&
        !isCharacterAssignedToRaidInCycle(character.char_id, contentId, event.id)
      );
    }

    return characters;
  }

  function isCharacterAssignedToRaidInCycle(charId: number, contentId: string, currentEventId: string): boolean {
    // Check all calendar assignments for this character
    for (const assignment of assignments) {
      if (assignment.charId !== charId) continue;
      if (assignment.eventKey === currentEventId) continue; // Skip current event

      // Check if this assignment is for the same content_id
      const assignedEvent = events.find(e => e.id === assignment.eventKey);
      if (!assignedEvent) continue;

      // For raid-train child events, we need to check the child's raid name
      if (assignedEvent.sectionCode && assignedEvent.runType === 'raid-train') {
        const assignedContentId = extractContentIdFromRaidName(assignedEvent.raidName);
        if (assignedContentId === contentId) return true;
      } else {
        // For regular events, check the event's raid name
        const assignedContentId = extractContentIdFromRaidName(assignedEvent.raidName);
        if (assignedContentId === contentId) return true;
      }
    }

    // Check all reservations for this character
    for (const reservation of reservations) {
      if (reservation.charId !== charId) continue;
      if (reservation.contentId === contentId) return true;
    }

    return false;
  }

  async function assignCharacter(event: DashboardCalendarEvent, charIdValue: string) {
    const charId = Number(charIdValue);
    const character = characters.find((entry) => entry.char_id === charId);
    if (!character) {
      clearDashboardCalendarAssignment(event.id);
      return;
    }
    await saveDashboardCalendarAssignment(event, character);
    dispatchCalendarChanged();
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
    // Calculate days to subtract to get to Wednesday (ongoing week)
    // Wednesday is day 3, so: if day >= 3, subtract (day - 3), if day < 3, subtract (day + 4)
    const daysToSubtract = day >= 3 ? day - 3 : day + 4;
    d.setDate(d.getDate() - daysToSubtract);
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
    if (isRaidTrain(event)) {
      const childEvents = getChildEvents(event);
      const assignedCount = childEvents.filter(child =>
        allAssignments.some(assignment => assignment.eventKey === child.id)
      ).length;
      return assignedCount > 0 ? `${event.title} (${assignedCount}/${childEvents.length})` : event.title;
    }
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
      // Skip child events of raid-trains - they'll be shown via the parent
      if ((event as any).isChildOfRaidTrain) continue;
      day.items.push({
        id: `signup-${event.id}`,
        time: formatTime(event.startsAt),
        label: signupCalendarLabel(event, allAssignments),
        kind: 'signup'
      });
    }

    for (const reservation of allReservations) {
      if (!reservation.scheduledAt) continue;
      for (const day of days) {
        const occurrence = getReservationOccurrenceTimestamp(reservation, day.date);
        if (!occurrence) continue;
        day.items.push({
          id: `reservation-${reservation.id}-${day.key}`,
          time: formatTime(occurrence),
          label: `${reservation.label || getRaidName(reservation.contentId)} · ${getRaidName(reservation.contentId)}`,
          kind: 'reservation'
        });
      }
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

<div class="dashboard-calendar{widthVar}">
  <button
    type="button"
    class="calendar-toggle"
    class:has-events={events.length > 0}
    on:click={() => open = !open}
    title="Open planned MeowGang raid calendar"
  >
    <img src={calendarIcon} alt="Calendar" aria-hidden="true" />
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
              : `${filteredEvents.length} signup${filteredEvents.length === 1 ? '' : 's'} | ${datedReservations.length} reservation${datedReservations.length === 1 ? '' : 's'}`}
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
            {#if isRaidTrain(event)}
              <article class="calendar-event raid-train-event">
                <button
                  type="button"
                  class="event-main raid-train-header"
                  aria-expanded={expandedRaidTrains.has(event.id)}
                  on:click={() => toggleRaidTrainExpansion(event.id)}
                  on:keydown={(e) => handleRaidTrainKeydown(event.id, e)}
                >
                  <div class="raid-train-info">
                    <strong>{event.title}</strong>
                    <span>{event.startsAtLabel} | {event.sectionLabel}</span>
                    <small>{event.runType.replace('-', ' ')} | {event.role.toUpperCase()} | {event.status.replace('_', ' ')}</small>
                  </div>
                  <span class="expand-toggle">
                    {expandedRaidTrains.has(event.id) ? '▼' : '▶'}
                  </span>
                </button>
                {#if expandedRaidTrains.has(event.id)}
                  <div class="raid-train-sections">
                    {#each getChildEvents(event) as childEvent}
                      <div class="raid-train-section">
                        <strong>{childEvent.sectionLabel || childEvent.raidName}</strong>
                        <label>
                          <span>Character</span>
                          <select
                            value={getAssignmentForEvent(childEvent.id)?.charId || ''}
                            on:change={(changeEvent) => assignCharacter(childEvent, (changeEvent.currentTarget as HTMLSelectElement).value)}
                          >
                            <option value="">Not assigned</option>
                            {#each getAvailableCharactersForEvent(childEvent) as character}
                              <option value={character.char_id}>{character.char_name}</option>
                            {/each}
                            {#if getAssignmentForEvent(childEvent.id) && !getAvailableCharactersForEvent(childEvent).find(c => c.char_id === getAssignmentForEvent(childEvent.id)?.charId)}
                              <option value={getAssignmentForEvent(childEvent.id)?.charId}>{getAssignmentForEvent(childEvent.id)?.charName} (already completed)</option>
                            {/if}
                          </select>
                        </label>
                        {#if assignedCharacterName(childEvent)}
                          <small class="assigned">Assigned: {assignedCharacterName(childEvent)}</small>
                        {/if}
                      </div>
                    {/each}
                  </div>
                {/if}
              </article>
            {:else}
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
                    {#each getAvailableCharactersForEvent(event) as character}
                      <option value={character.char_id}>{character.char_name}</option>
                    {/each}
                    {#if getAssignmentForEvent(event.id) && !getAvailableCharactersForEvent(event).find(c => c.char_id === getAssignmentForEvent(event.id)?.charId)}
                      <option value={getAssignmentForEvent(event.id)?.charId}>{getAssignmentForEvent(event.id)?.charName} (already completed)</option>
                    {/if}
                  </select>
                </label>
                {#if assignedCharacterName(event)}
                  <small class="assigned">Assigned: {assignedCharacterName(event)}</small>
                {/if}
              </article>
            {/if}
          {/each}
        </div>
      {:else if totalCalendarItems === 0 && undatedReservations.length === 0}
        <p class="empty">No active signup sheets or dated reservations found.</p>
      {/if}

      {#if datedReservations.length > 0}
        <h4>Local Reservations</h4>
        {#if selectedDayKey}
          <div class="filter-info">
            <span>Filtered by {selectedDayKey}</span>
            <button type="button" on:click={() => selectedDayKey = null}>Clear filter</button>
          </div>
        {/if}
        <div class="event-list">
          {#each datedReservations as reservation}
            <article class="calendar-event reservation-event">
              <div class="event-main">
                <strong>{getRaidName(reservation.contentId)}</strong>
                <span>{reservation.difficulty}</span>
                <small>{formatReservationScheduleLabel(reservation)}</small>
              </div>
              <div>
                <small>Character: {characters.find((char) => char.char_id === reservation.charId)?.char_name || 'Unknown'}</small>
              </div>
            </article>
          {/each}
        </div>
      {/if}

      {#if undatedReservations.length > 0}
        <h4>Undated Reservations</h4>
        <div class="event-list">
          {#each undatedReservations as reservation}
            <article class="calendar-event reservation-event">
              <div class="event-main">
                <strong>{getRaidName(reservation.contentId)}</strong>
                <span>{reservation.difficulty}</span>
                <small>{formatReservationScheduleLabel(reservation)}</small>
              </div>
              <div>
                <small>Character: {characters.find((char) => char.char_id === reservation.charId)?.char_name || 'Unknown'}</small>
              </div>
            </article>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .dashboard-calendar {
    position: relative;
    z-index: 12;
  }

  .dashboard-calendar-full-width {
    width: var(--dashboard-frame-width);
    margin: 0 auto 0.55rem;
    display: flex;
    justify-content: flex-end;
  }

  .calendar-toggle {
    appearance: none;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 34%, transparent);
    background: color-mix(in srgb, var(--md-sys-color-surface-container) 86%, transparent);
    color: var(--md-sys-color-on-surface);
    border-radius: 6px;
    height: 36px;
    min-width: 54px;
    padding: 0 0.65rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    cursor: pointer;
    font: inherit;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .calendar-toggle img {
    width: 20px;
    height: 20px;
    object-fit: contain;
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
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
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
    z-index: 100;
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
    position: fixed;
    top: 5rem;
    left: 50%;
    transform: translateX(-50%);
    width: min(46rem, calc(100vw - 2rem));
    max-height: calc(100vh - 6rem);
    overflow-y: auto;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 34%, transparent);
    background: var(--md-sys-color-surface-container);
    color: var(--md-sys-color-on-surface);
    border-radius: 8px;
    box-shadow: var(--app-shadow-md);
    padding: 0.75rem;
    z-index: 100;
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

  .raid-train-event {
    grid-template-columns: 1fr;
  }

  .raid-train-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    padding: 0.4rem;
    border-radius: 4px;
    transition: background 0.2s;
    appearance: none;
    border: none;
    background: transparent;
    font: inherit;
    text-align: left;
    width: 100%;
  }

  .raid-train-header:hover {
    background: color-mix(in srgb, var(--md-sys-color-primary) 8%, transparent);
  }

  .raid-train-info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .expand-toggle {
    width: 1.5rem;
    height: 1.5rem;
    display: grid;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-outline) 70%, transparent);
    background: var(--md-sys-color-surface);
    border-radius: 4px;
    font-size: 0.7rem;
    flex-shrink: 0;
  }

  .raid-train-sections {
    display: grid;
    gap: 0.4rem;
    padding: 0.4rem 0.4rem 0.4rem 1.2rem;
    border-left: 2px solid color-mix(in srgb, var(--md-sys-color-primary) 28%, transparent);
    margin-top: 0.4rem;
  }

  .raid-train-section {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(9rem, 12rem);
    gap: 0.5rem;
    align-items: center;
    padding: 0.4rem;
    border-radius: 4px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-high) 52%, transparent);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-outline) 35%, transparent);
  }

  .raid-train-section label {
    display: grid;
    gap: 0.25rem;
    font-size: 0.64rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .raid-train-section select {
    min-width: 0;
    height: 1.6rem;
    border-radius: 4px;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-outline) 70%, transparent);
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    font: inherit;
    font-size: 0.68rem;
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
