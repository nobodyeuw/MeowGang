const ACTIVE_ROSTER_STORAGE_KEY = 'activeRosterId';

export function getActiveRosterPreference(): string {
  return localStorage.getItem(ACTIVE_ROSTER_STORAGE_KEY) || '';
}

export function setActiveRosterPreference(rosterId: string) {
  localStorage.setItem(ACTIVE_ROSTER_STORAGE_KEY, rosterId);
}

export function clearActiveRosterPreference() {
  localStorage.removeItem(ACTIVE_ROSTER_STORAGE_KEY);
}
