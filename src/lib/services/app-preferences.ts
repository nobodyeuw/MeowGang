import { invoke } from '@tauri-apps/api/core';

const HEADER_COUNTDOWN_STORAGE_KEY = 'showHeaderCountdown';

export interface AppSystemPreferences {
  showSetupGuideButton: boolean;
  showAuthWelcome: boolean;
  showHaalsHourglassReminder: boolean;
  startWithLoaLogs: boolean;
  loaLogsPathConfigured: boolean;
}

export async function loadAppSystemPreferences(): Promise<AppSystemPreferences> {
  const settings: any = await invoke('get_system_settings');
  const loaLogsPath = settings.loaLogsExePath ?? settings.loa_logs_exe_path ?? '';

  return {
    showSetupGuideButton: settings.showSetupGuideButton ?? settings.show_setup_guide_button ?? true,
    showAuthWelcome: settings.showAuthWelcome ?? settings.show_auth_welcome ?? true,
    showHaalsHourglassReminder: settings.showHaalsHourglassReminder ?? settings.show_haals_hourglass_reminder ?? true,
    startWithLoaLogs: settings.startWithLoaLogs ?? settings.start_with_loa_logs ?? false,
    loaLogsPathConfigured: typeof loaLogsPath === 'string' && loaLogsPath.trim().length > 0
  };
}

export async function getLoaLogsReminderMessage(startWithLoaLogs: boolean, loaLogsPathConfigured: boolean): Promise<string> {
  if (!startWithLoaLogs) {
    return '';
  }

  const isRunning = await invoke<boolean>('is_loa_logs_running');
  if (isRunning) {
    return '';
  }

  return loaLogsPathConfigured
    ? 'Do not forget to start LOA Logs.exe for maximum efficiency.'
    : 'For better QoL you should install LOA Logs.exe or set the path manually in Settings.';
}

export async function isLoaLogsRunning(): Promise<boolean> {
  return invoke<boolean>('is_loa_logs_running');
}

export function getHeaderCountdownPreference(): boolean {
  return localStorage.getItem(HEADER_COUNTDOWN_STORAGE_KEY) !== '0';
}

export function setHeaderCountdownPreference(enabled: boolean) {
  localStorage.setItem(HEADER_COUNTDOWN_STORAGE_KEY, enabled ? '1' : '0');
  window.dispatchEvent(new CustomEvent<boolean>('header-countdown:changed', { detail: enabled }));
}
