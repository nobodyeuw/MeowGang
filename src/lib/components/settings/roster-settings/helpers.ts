import { getGameClassDisplayName, getGameClassIconId } from '$lib/data/classes';

export interface SyncMetadata {
  timestamp: number;
  sync_status: string;
  data?: string | null;
}

const DAILY_UPDATE_COOLDOWN_MS = 24 * 60 * 60 * 1000;

export function getDailyUpdateBadge(history: SyncMetadata[], now: number, loading: boolean) {
  if (loading) {
    return {
      label: 'Checking update',
      state: 'loading'
    };
  }

  const latest = history[0];
  const latestCompleted = history.find((entry) => entry.sync_status === 'completed');

  if (latest?.sync_status === 'started' && (!latestCompleted || latest.timestamp > latestCompleted.timestamp)) {
    return {
      label: 'Updating now',
      state: 'running'
    };
  }

  if (!latestCompleted?.timestamp) {
    return {
      label: 'Daily update available',
      state: 'ready'
    };
  }

  const remainingMs = latestCompleted.timestamp + DAILY_UPDATE_COOLDOWN_MS - now;
  if (remainingMs <= 0) {
    return {
      label: 'Daily update available',
      state: 'ready'
    };
  }

  return {
    label: `Next update in ${formatUpdateCountdown(remainingMs)}`,
    state: 'cooldown'
  };
}

export function formatUpdateCountdown(ms: number): string {
  const totalMinutes = Math.max(0, Math.ceil(ms / 60000));
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return hours > 0 ? `${hours}h ${minutes.toString().padStart(2, '0')}m` : `${minutes}m`;
}

export function getClassIcon(classId: string): string {
  return getGameClassIconId(classId);
}

export function getClassName(classId: string): string {
  return getGameClassDisplayName(classId);
}
