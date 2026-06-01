import { APP_THEME_OPTIONS, DEFAULT_APP_THEME, isAppThemeId, type AppThemeId } from '$lib/data/themes';

const THEME_PREFERENCE_KEY = 'loa-tracker-theme';

export function getThemePreference(): AppThemeId {
  if (typeof localStorage === 'undefined') return DEFAULT_APP_THEME;

  const storedTheme = localStorage.getItem(THEME_PREFERENCE_KEY);
  const normalizedTheme = storedTheme === 'cyberpunk' ? 'powi' : storedTheme;
  return isAppThemeId(normalizedTheme) ? normalizedTheme : DEFAULT_APP_THEME;
}

export function setThemePreference(theme: AppThemeId) {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(THEME_PREFERENCE_KEY, theme);
  }

  applyTheme(theme);
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('app-theme:changed', { detail: theme }));
  }
}

export function applyTheme(theme = getThemePreference()) {
  if (typeof document === 'undefined') return;
  document.documentElement.dataset.theme = theme;
}

export function getThemeLabel(theme: AppThemeId): string {
  return APP_THEME_OPTIONS.find((option) => option.id === theme)?.label ?? 'Outlaw';
}
