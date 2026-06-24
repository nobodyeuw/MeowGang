import type { Character } from '$lib/store';
import { invoke } from '@tauri-apps/api/core';
import {
  getRaidSignupSelectedRaids,
  type RaidSignupPreRegisteredMember,
  type RaidSignupSheet
} from '$lib/data/raid-management';
import { loadRaidSignupSheetsFromSupabase } from '$lib/services/raid-management';

const ASSIGNMENTS_STORAGE_KEY = 'dashboardCalendar.assignments.v1';
const RESERVATIONS_STORAGE_KEY = 'dashboardCalendar.reservations.v1';
const DISMISSED_REMINDER_STORAGE_KEY = 'dashboardCalendar.dismissedReminderDates.v1';

export interface DashboardCalendarEvent {
  id: string;
  sheetId: string;
  eventId: string;
  title: string;
  startsAt: number;
  startsAtLabel: string;
  runType: RaidSignupSheet['runType'];
  raidName: string;
  sectionCode?: string;
  sectionLabel?: string;
  role: RaidSignupPreRegisteredMember['role'];
  status: RaidSignupPreRegisteredMember['status'];
}

export interface DashboardCalendarAssignment {
  eventKey: string;
  sheetId: string;
  eventId: string;
  sectionCode?: string;
  charId: number;
  charName: string;
  raidContentId?: string;
  updatedAt: number;
}

export interface DashboardRaidReservation {
  id: string;
  charId: number;
  contentId: string;
  difficulty: string;
  label: string;
  reservedAt: number;
  scheduledAt?: number;
  recurringWeekly: boolean;
}

function readJson<T>(key: string, fallback: T): T {
  if (typeof localStorage === 'undefined') return fallback;
  try {
    return JSON.parse(localStorage.getItem(key) || '') as T;
  } catch {
    return fallback;
  }
}

function writeJson<T>(key: string, value: T) {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(key, JSON.stringify(value));
}

export function dispatchCalendarChanged() {
  if (typeof window === 'undefined') return;
  window.dispatchEvent(new CustomEvent('dashboard-calendar:changed'));
}

function parseDiscordTimestamp(value: string): number {
  const match = String(value || '').match(/<t:(\d+):[a-zA-Z]>/);
  if (match) return Number(match[1]) * 1000;
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function formatDateTime(timestamp: number): string {
  if (!timestamp) return 'No time set';
  return new Intl.DateTimeFormat(undefined, {
    weekday: 'short',
    day: '2-digit',
    month: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  }).format(new Date(timestamp));
}

function buildEventKey(sheet: RaidSignupSheet, sectionCode = ''): string {
  return sectionCode ? `${sheet.id}:${sectionCode}` : sheet.id;
}

function getSignedMember(sheet: RaidSignupSheet, discordId: string): RaidSignupPreRegisteredMember | undefined {
  const normalizedId = String(discordId || '').trim();
  if (!normalizedId) return undefined;
  return (sheet.preRegisteredMembers || []).find((member) => member.discordId === normalizedId);
}

function isActiveEvent(timestamp: number): boolean {
  if (!timestamp) return false;
  const now = Date.now();
  return timestamp >= now - 2 * 60 * 60 * 1000;
}

export async function loadUserDashboardCalendarEvents(discordId: string): Promise<DashboardCalendarEvent[]> {
  const normalizedDiscordId = String(discordId || '').trim();
  if (!normalizedDiscordId) return [];

  const sheets = await loadRaidSignupSheetsFromSupabase();
  const events: DashboardCalendarEvent[] = [];

  for (const sheet of sheets) {
    const member = getSignedMember(sheet, normalizedDiscordId);
    if (!member) continue;

    const startsAt = parseDiscordTimestamp(sheet.startsAt);
    if (!isActiveEvent(startsAt)) continue;

    const selectedRaids = getRaidSignupSelectedRaids(sheet.raidIds, sheet.customRaids || []);

    // For raid-train, create a single grouped event with all sections
    if (sheet.runType === 'raid-train') {
      // Auto-generate section codes based on number of raids if not set
      const sections = member.raidSections && member.raidSections.length > 0
        ? member.raidSections
        : selectedRaids.map((_, index) => String.fromCharCode(65 + index)); // A, B, C, etc.
      const sectionEvents: DashboardCalendarEvent[] = [];

      for (const sectionCode of sections) {
        const sectionIndex = sectionCode ? sectionCode.charCodeAt(0) - 65 : -1;
        const sectionRaid = sectionIndex >= 0 && sectionIndex < selectedRaids.length ? selectedRaids[sectionIndex] : undefined;
        const raidName = sectionRaid?.name || selectedRaids.map((raid) => raid.name).join(', ') || 'Raid signup';
        const sectionLabel = sectionCode && sectionRaid ? `${sectionCode} - ${sectionRaid.name}` : undefined;

        sectionEvents.push({
          id: buildEventKey(sheet, sectionCode),
          sheetId: sheet.id,
          eventId: sheet.eventId,
          title: sheet.title,
          startsAt,
          startsAtLabel: formatDateTime(startsAt),
          runType: sheet.runType,
          raidName,
          sectionCode: sectionCode || undefined,
          sectionLabel,
          role: member.role,
          status: member.status
        });
      }

      // Create a single parent event for the raid-train
      const parentEvent: DashboardCalendarEvent = {
        id: buildEventKey(sheet, ''), // Use empty section code for parent
        sheetId: sheet.id,
        eventId: sheet.eventId,
        title: sheet.title,
        startsAt,
        startsAtLabel: formatDateTime(startsAt),
        runType: sheet.runType,
        raidName: 'Raid Train',
        sectionCode: undefined,
        sectionLabel: `${selectedRaids.length} sections`,
        role: member.role,
        status: member.status
      };

      // Store child events as a property (this is a workaround since we can't modify the interface)
      (parentEvent as any).childEvents = sectionEvents;
      events.push(parentEvent);

      // Also add child events to the main events array so they can be assigned individually
      for (const sectionEvent of sectionEvents) {
        (sectionEvent as any).isChildOfRaidTrain = true;
        (sectionEvent as any).parentEventId = parentEvent.id;
        events.push(sectionEvent);
      }
    } else {
      // For non-raid-train events, create individual events as before
      events.push({
        id: buildEventKey(sheet, ''),
        sheetId: sheet.id,
        eventId: sheet.eventId,
        title: sheet.title,
        startsAt,
        startsAtLabel: formatDateTime(startsAt),
        runType: sheet.runType,
        raidName: selectedRaids.map((raid) => raid.name).join(', ') || 'Raid signup',
        sectionCode: undefined,
        sectionLabel: undefined,
        role: member.role,
        status: member.status
      });
    }
  }

  // Remove assignments for events that no longer exist (user signed off)
  const currentAssignments = getDashboardCalendarAssignments();
  const validEventKeys = new Set(events.map((event) => event.id));

  // Also include child event keys from raid-trains
  for (const event of events) {
    if ((event as any).childEvents) {
      for (const childEvent of (event as any).childEvents) {
        validEventKeys.add(childEvent.id);
      }
    }
  }

  const assignmentsToRemove = currentAssignments.filter((assignment) => !validEventKeys.has(assignment.eventKey));
  if (assignmentsToRemove.length > 0) {
    for (const assignment of assignmentsToRemove) {
      void invoke('clear_dashboard_calendar_assignment', { eventKey: assignment.eventKey });
    }
    writeJson(
      ASSIGNMENTS_STORAGE_KEY,
      currentAssignments.filter((assignment) => validEventKeys.has(assignment.eventKey))
    );
  }

  return events.sort((a, b) => a.startsAt - b.startsAt || a.title.localeCompare(b.title));
}

export function getDashboardCalendarAssignments(): DashboardCalendarAssignment[] {
  return readJson<DashboardCalendarAssignment[]>(ASSIGNMENTS_STORAGE_KEY, []);
}

export async function loadDashboardCalendarAssignments(): Promise<DashboardCalendarAssignment[]> {
  const assignments = await invoke<DashboardCalendarAssignment[]>('get_dashboard_calendar_assignments');
  writeJson(ASSIGNMENTS_STORAGE_KEY, assignments);
  return assignments;
}

export function getAssignmentForEvent(eventKey: string): DashboardCalendarAssignment | undefined {
  return getDashboardCalendarAssignments().find((assignment) => assignment.eventKey === eventKey);
}

export async function saveDashboardCalendarAssignment(
  event: DashboardCalendarEvent,
  character: Pick<Character, 'char_id' | 'char_name'>,
  raidContentId?: string
): Promise<DashboardCalendarAssignment> {
  const assignment = await invoke<DashboardCalendarAssignment>('save_dashboard_calendar_assignment', {
    input: {
      eventKey: event.id,
      sheetId: event.sheetId,
      eventId: event.eventId,
      sectionCode: event.sectionCode,
      charId: character.char_id,
      charName: character.char_name,
      raidContentId
    }
  });
  const next = [
    ...getDashboardCalendarAssignments().filter((item) => item.eventKey !== event.id),
    assignment
  ];
  writeJson(ASSIGNMENTS_STORAGE_KEY, next);
  dispatchCalendarChanged();
  return assignment;
}

export function saveDashboardCalendarAssignmentLocal(
  event: DashboardCalendarEvent,
  character: Pick<Character, 'char_id' | 'char_name'>,
  raidContentId?: string
): DashboardCalendarAssignment {
  const assignment = {
    eventKey: event.id,
    sheetId: event.sheetId,
    eventId: event.eventId,
    sectionCode: event.sectionCode,
    charId: character.char_id,
    charName: character.char_name,
    raidContentId,
    updatedAt: Date.now()
  };
  const next = [
    ...getDashboardCalendarAssignments().filter((item) => item.eventKey !== event.id),
    assignment
  ];
  writeJson(ASSIGNMENTS_STORAGE_KEY, next);
  dispatchCalendarChanged();
  return assignment;
}

export function clearDashboardCalendarAssignment(eventKey: string) {
  void invoke('clear_dashboard_calendar_assignment', { eventKey });
  writeJson(
    ASSIGNMENTS_STORAGE_KEY,
    getDashboardCalendarAssignments().filter((item) => item.eventKey !== eventKey)
  );
  dispatchCalendarChanged();
}

export function getDashboardRaidReservations(): DashboardRaidReservation[] {
  return readJson<DashboardRaidReservation[]>(RESERVATIONS_STORAGE_KEY, []);
}

export async function loadDashboardRaidReservations(): Promise<DashboardRaidReservation[]> {
  const reservations = await invoke<DashboardRaidReservation[]>('get_dashboard_raid_reservations');
  writeJson(RESERVATIONS_STORAGE_KEY, reservations);
  return reservations;
}

export function getReservationForRaid(charId: number, contentId: string, difficulty: string): DashboardRaidReservation | undefined {
  return getDashboardRaidReservations().find((reservation) =>
    reservation.charId === charId &&
    reservation.contentId === contentId &&
    reservation.difficulty === difficulty
  );
}

export async function saveDashboardRaidReservation(
  reservation: Omit<DashboardRaidReservation, 'id' | 'reservedAt'>
): Promise<DashboardRaidReservation> {
  const nextReservation = await invoke<DashboardRaidReservation>('save_dashboard_raid_reservation', {
    input: reservation
  });
  writeJson(RESERVATIONS_STORAGE_KEY, [
    ...getDashboardRaidReservations().filter((item) => item.id !== nextReservation.id),
    nextReservation
  ]);
  dispatchCalendarChanged();
  return nextReservation;
}

export function clearDashboardRaidReservation(charId: number, contentId: string, difficulty: string) {
  const id = `${charId}:${contentId}:${difficulty}`;
  void invoke('clear_dashboard_raid_reservation', { charId, contentId, difficulty });
  writeJson(
    RESERVATIONS_STORAGE_KEY,
    getDashboardRaidReservations().filter((reservation) => reservation.id !== id)
  );
  dispatchCalendarChanged();
}

export function clearDashboardRaidReservationNoDispatch(charId: number, contentId: string, difficulty: string) {
  const id = `${charId}:${contentId}:${difficulty}`;
  void invoke('clear_dashboard_raid_reservation', { charId, contentId, difficulty });
  writeJson(
    RESERVATIONS_STORAGE_KEY,
    getDashboardRaidReservations().filter((reservation) => reservation.id !== id)
  );
}

export function cleanupExpiredDashboardRaidReservations(now = Date.now()) {
  void invoke('cleanup_dashboard_raid_reservations', { now });
  const next = getDashboardRaidReservations().filter((reservation) =>
    reservation.recurringWeekly || !reservation.scheduledAt || reservation.scheduledAt >= now
  );
  writeJson(RESERVATIONS_STORAGE_KEY, next);
}

export function getTodayCalendarEvents(events: DashboardCalendarEvent[]): DashboardCalendarEvent[] {
  const now = new Date();
  return events.filter((event) => {
    const date = new Date(event.startsAt);
    return date.getFullYear() === now.getFullYear()
      && date.getMonth() === now.getMonth()
      && date.getDate() === now.getDate();
  });
}

export function wasCalendarReminderDismissedToday(): boolean {
  const todayKey = new Date().toISOString().slice(0, 10);
  return readJson<string[]>(DISMISSED_REMINDER_STORAGE_KEY, []).includes(todayKey);
}

export function dismissCalendarReminderToday() {
  const todayKey = new Date().toISOString().slice(0, 10);
  const next = Array.from(new Set([...readJson<string[]>(DISMISSED_REMINDER_STORAGE_KEY, []), todayKey]));
  writeJson(DISMISSED_REMINDER_STORAGE_KEY, next.slice(-14));
}

export function dayKeyFromTimestamp(timestamp: number): string {
  const date = new Date(timestamp);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

export function getReservationOccurrenceTimestamp(
  reservation: DashboardRaidReservation,
  weekDay: Date
): number | null {
  if (!reservation.scheduledAt) return null;

  if (reservation.recurringWeekly) {
    const template = new Date(reservation.scheduledAt);
    if (weekDay.getDay() !== template.getDay()) return null;
    const occurrence = new Date(weekDay);
    occurrence.setHours(template.getHours(), template.getMinutes(), 0, 0);
    return occurrence.getTime();
  }

  return dayKeyFromTimestamp(reservation.scheduledAt) === dayKeyFromTimestamp(weekDay.getTime())
    ? reservation.scheduledAt
    : null;
}

export function reservationMatchesDayKey(
  reservation: DashboardRaidReservation,
  selectedDayKey: string,
  weekStart: Date
): boolean {
  if (!reservation.scheduledAt) return !selectedDayKey;

  if (!selectedDayKey) {
    for (let index = 0; index < 7; index += 1) {
      const date = new Date(weekStart);
      date.setDate(weekStart.getDate() + index);
      if (getReservationOccurrenceTimestamp(reservation, date)) return true;
    }
    return false;
  }

  const [year, month, day] = selectedDayKey.split('-').map(Number);
  const date = new Date(year, month - 1, day);
  return Boolean(getReservationOccurrenceTimestamp(reservation, date));
}

export function formatReservationScheduleLabel(reservation: DashboardRaidReservation): string {
  if (!reservation.scheduledAt) {
    return reservation.recurringWeekly ? 'Weekly/static reservation' : 'One-time reservation';
  }

  const schedule = formatDateTime(reservation.scheduledAt);
  return reservation.recurringWeekly ? `Every week · ${schedule}` : schedule;
}

export function mergeDashboardRaidReservations(
  remoteReservations: DashboardRaidReservation[] = [],
  localReservations: DashboardRaidReservation[] = getDashboardRaidReservations()
): DashboardRaidReservation[] {
  const byId = new Map<string, DashboardRaidReservation>();
  for (const reservation of remoteReservations) byId.set(reservation.id, reservation);
  for (const reservation of localReservations) byId.set(reservation.id, reservation);
  return Array.from(byId.values()).sort((a, b) =>
    (a.scheduledAt || a.reservedAt) - (b.scheduledAt || b.reservedAt)
  );
}
