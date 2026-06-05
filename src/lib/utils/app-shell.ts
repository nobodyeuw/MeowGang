import type { AppTab, MeowConnectHeaderState } from '$lib/types/app-shell';

export const VALID_APP_TABS: AppTab[] = ['dashboard', 'todo', 'marketplace', 'settings', 'meow-connect', 'updates'];

export function isAppTab(value: string): value is AppTab {
  return (VALID_APP_TABS as string[]).includes(value);
}

export function getMeowConnectHeaderLabel(state: MeowConnectHeaderState): string {
  if (state === 'active') return 'Active';
  if (state === 'connecting') return 'Connecting';
  if (state === 'sleeping') return 'Sleeping';
  if (state === 'offline') return 'Offline';
  if (state === 'login_required') return 'Login required';
  return 'Inactive';
}

export function getWeeklyResetCountdownTarget(now = new Date()): Date | null {
  const day = now.getUTCDay();
  const currentDailyReset = new Date(now);
  currentDailyReset.setUTCHours(10, 0, 0, 0);

  const isAfterTuesdayDailyReset = day === 2 && now.getTime() >= currentDailyReset.getTime();
  const isBeforeWednesdayWeeklyReset = day === 3 && now.getTime() < currentDailyReset.getTime();

  if (!isAfterTuesdayDailyReset && !isBeforeWednesdayWeeklyReset) {
    return null;
  }

  const weeklyReset = new Date(now);
  weeklyReset.setUTCHours(10, 0, 0, 0);
  if (day === 2) {
    weeklyReset.setUTCDate(weeklyReset.getUTCDate() + 1);
  }

  return weeklyReset;
}

export function formatResetCountdown(nextResetTime: string, now = new Date()): string {
  const weeklyResetWindow = getWeeklyResetCountdownTarget(now);
  const targetResetTime = weeklyResetWindow ? weeklyResetWindow.toISOString() : nextResetTime;
  const resetLabel = weeklyResetWindow ? 'Next weekly reset in' : 'Next daily reset in';

  if (!targetResetTime) {
    return 'Reset timer unavailable';
  }

  const reset = new Date(targetResetTime);
  const diff = reset.getTime() - now.getTime();

  if (diff <= 0) {
    return weeklyResetWindow ? 'Weekly reset should have occurred!' : 'Daily reset should have occurred!';
  }

  const totalMinutes = Math.ceil(diff / (1000 * 60));
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  const formatTimePart = (value: number) => value.toString().padStart(2, '0');

  if (hours > 0) {
    return `${resetLabel}: ${formatTimePart(hours)}H ${formatTimePart(minutes)}M`;
  }

  return `${resetLabel}: ${formatTimePart(minutes)}M`;
}
