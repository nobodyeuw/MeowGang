const FAVORITES_STORAGE_KEY = 'meowConnect.favoriteCharacterKeys';
const CONSENT_STORAGE_KEY = 'meowConnect.consentAccepted';
const FEATURE_ENABLED_STORAGE_KEY = 'meowConnect.featureEnabled';
const REALTIME_ENABLED_STORAGE_KEY = 'meowConnect.realtimeEnabled';
const FRIEND_CLEAR_HINTS_ENABLED_STORAGE_KEY = 'meowConnect.friendClearHintsEnabled';
const UNSYNCED_CHANGES_STORAGE_KEY = 'meowConnect.unsyncedChanges';

export const LAST_UPLOAD_STORAGE_KEY = 'meowConnect.lastUploadAt';
export const LAST_UPLOAD_SIGNATURE_STORAGE_KEY = 'meowConnect.lastUploadSignature';

export function getStoredTimestamp(key: string): number {
  const value = Number(localStorage.getItem(key) || 0);
  return Number.isFinite(value) ? value : 0;
}

export function setStoredTimestamp(key: string, value: number) {
  localStorage.setItem(key, String(value));
}

export function hasStoredMeowConnectConsent(): boolean {
  if (typeof localStorage === 'undefined') return false;
  return localStorage.getItem(CONSENT_STORAGE_KEY) === '1';
}

export function setStoredMeowConnectConsent(accepted: boolean) {
  if (typeof localStorage === 'undefined') return;
  if (accepted) {
    localStorage.setItem(CONSENT_STORAGE_KEY, '1');
  } else {
    localStorage.removeItem(CONSENT_STORAGE_KEY);
  }
}

export function hasStoredMeowConnectFeatureEnabled(): boolean {
  if (typeof localStorage === 'undefined') return false; // Disabled by default
  return localStorage.getItem(FEATURE_ENABLED_STORAGE_KEY) === '1'; // Only enable if explicitly set to '1'
}

export function setStoredMeowConnectFeatureEnabled(enabled: boolean) {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(FEATURE_ENABLED_STORAGE_KEY, enabled ? '1' : '0');
}

export function isStoredMeowConnectRealtimeEnabled(): boolean {
  if (typeof localStorage === 'undefined') return true;
  return localStorage.getItem(REALTIME_ENABLED_STORAGE_KEY) !== '0';
}

export function setStoredMeowConnectRealtimeEnabled(enabled: boolean) {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(REALTIME_ENABLED_STORAGE_KEY, enabled ? '1' : '0');
}

export function isStoredMeowConnectFriendClearHintsEnabled(): boolean {
  if (typeof localStorage === 'undefined') return false;
  return localStorage.getItem(FRIEND_CLEAR_HINTS_ENABLED_STORAGE_KEY) === '1';
}

export function setStoredMeowConnectFriendClearHintsEnabled(enabled: boolean) {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(FRIEND_CLEAR_HINTS_ENABLED_STORAGE_KEY, enabled ? '1' : '0');
}

export function hasStoredUnsyncedChanges(): boolean {
  if (typeof localStorage === 'undefined') return false;
  return localStorage.getItem(UNSYNCED_CHANGES_STORAGE_KEY) === '1';
}

export function setStoredUnsyncedChanges() {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(UNSYNCED_CHANGES_STORAGE_KEY, '1');
}

export function clearStoredUnsyncedChanges() {
  if (typeof localStorage === 'undefined') return;
  localStorage.removeItem(UNSYNCED_CHANGES_STORAGE_KEY);
}

export function getMeowConnectFavoriteKey(ownerId: string, charId: number): string {
  return `${ownerId}:${charId}`;
}

export function loadFavoritePlayerIds(): Set<string> {
  try {
    const values = JSON.parse(localStorage.getItem(FAVORITES_STORAGE_KEY) || '[]');
    return new Set(Array.isArray(values) ? values.map((value) => String(value)).filter(Boolean) : []);
  } catch {
    return new Set();
  }
}

export function saveFavoritePlayerIds(ids: Set<string>) {
  localStorage.setItem(FAVORITES_STORAGE_KEY, JSON.stringify(Array.from(ids).sort()));
}

export function toggleFavoritePlayerId(favoriteKey: string): Set<string> {
  const ids = loadFavoritePlayerIds();
  if (ids.has(favoriteKey)) {
    ids.delete(favoriteKey);
  } else {
    ids.add(favoriteKey);
  }
  saveFavoritePlayerIds(ids);
  return ids;
}
