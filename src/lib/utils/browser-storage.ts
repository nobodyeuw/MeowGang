const LEGACY_BROWSER_STORAGE_PATTERNS = [
  'party_plans',
  'partyplanner',
  'party-planner',
  'google_script',
  'google-script',
  'apps_script',
  'apps-script',
  'script.google.com'
];

export function cleanupLegacyBrowserStorage() {
  cleanupLegacyStorageArea(localStorage);
  cleanupLegacyStorageArea(sessionStorage);
}

function cleanupLegacyStorageArea(storage: Storage) {
  const keysToRemove: string[] = [];
  for (let index = 0; index < storage.length; index += 1) {
    const key = storage.key(index);
    if (!key) continue;
    const value = storage.getItem(key) || '';
    const combined = `${key}\n${value}`.toLowerCase();
    if (LEGACY_BROWSER_STORAGE_PATTERNS.some((pattern) => combined.includes(pattern))) {
      keysToRemove.push(key);
    }
  }

  for (const key of keysToRemove) {
    storage.removeItem(key);
  }
}
