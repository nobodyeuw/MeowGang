import { CALENDAR, type CalendarDay } from '../data/tasks';

const DAILY_RESET_UTC_HOUR = 10;
const CALENDAR_EVENT_AVAILABLE_MS = 17 * 60 * 60 * 1000;
const DAYS = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];

function getGameDayStart(now = new Date()): Date {
  const reset = new Date(now);
  reset.setUTCHours(DAILY_RESET_UTC_HOUR, 0, 0, 0);
  if (now.getTime() < reset.getTime()) {
    reset.setUTCDate(reset.getUTCDate() - 1);
  }
  return reset;
}

function isCalendarTaskScheduled(taskId: string, dayData?: CalendarDay): boolean {
  if (!dayData) return false;
  if (taskId === 'gate') return dayData.gate_available;
  if (taskId === 'boss') return dayData.boss_available;
  return true;
}

function isCalendarEventWindowOpen(now = new Date()): boolean {
  const gameDayStart = getGameDayStart(now);
  return now.getTime() < gameDayStart.getTime() + CALENDAR_EVENT_AVAILABLE_MS;
}

export function getCurrentDayName(): string {
  return DAYS[getGameDayStart().getUTCDay()];
}

export function isTaskAvailable(taskId: string): boolean {
  if (taskId !== 'gate' && taskId !== 'boss') return true;

  const currentDay = getCurrentDayName();
  const dayData = CALENDAR.find(day => day.day_name === currentDay);
  return isCalendarTaskScheduled(taskId, dayData) && isCalendarEventWindowOpen();
}

export function getNextAvailableTime(taskId: string): Date | null {
  if (isTaskAvailable(taskId)) return null;

  const now = new Date();
  const currentGameDayStart = getGameDayStart(now);

  for (let i = 0; i < 7; i++) {
    const checkDate = new Date(currentGameDayStart);
    checkDate.setUTCDate(currentGameDayStart.getUTCDate() + i);
    if (checkDate.getTime() <= now.getTime()) continue;

    const dayName = DAYS[checkDate.getUTCDay()];
    const dayData = CALENDAR.find(day => day.day_name === dayName);

    if (isCalendarTaskScheduled(taskId, dayData)) {
      return checkDate;
    }
  }

  return null;
}

export function getTimeUntilAvailable(taskId: string): string {
  const nextAvailable = getNextAvailableTime(taskId);
  if (!nextAvailable) return '';

  const now = new Date();
  const diff = nextAvailable.getTime() - now.getTime();
  if (diff <= 0) return '';

  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
  const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));

  if (days > 0) return `${days}d ${hours}h ${minutes}m`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

export function getCurrentAvailabilityStatus(): { gate: boolean; boss: boolean } {
  return {
    gate: isTaskAvailable('gate'),
    boss: isTaskAvailable('boss')
  };
}
