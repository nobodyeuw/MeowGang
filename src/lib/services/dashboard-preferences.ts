export type DashboardViewMode = 'cards' | 'compact';

const DASHBOARD_VIEW_STORAGE_KEY = 'dashboardView';
const DASHBOARD_STATIC_BADGES_STORAGE_KEY = 'showDashboardStaticBadges';

export function getDashboardViewPreference(): DashboardViewMode {
  return localStorage.getItem(DASHBOARD_VIEW_STORAGE_KEY) === 'cards' ? 'cards' : 'compact';
}

export function setDashboardViewPreference(view: DashboardViewMode) {
  localStorage.setItem(DASHBOARD_VIEW_STORAGE_KEY, view);
  window.dispatchEvent(new CustomEvent<DashboardViewMode>('dashboard-view:changed', { detail: view }));
}

export function getDashboardStaticBadgesPreference(): boolean {
  return localStorage.getItem(DASHBOARD_STATIC_BADGES_STORAGE_KEY) !== '0';
}

export function setDashboardStaticBadgesPreference(enabled: boolean) {
  localStorage.setItem(DASHBOARD_STATIC_BADGES_STORAGE_KEY, enabled ? '1' : '0');
  window.dispatchEvent(new CustomEvent<boolean>('dashboard-static-badges:changed', { detail: enabled }));
}
