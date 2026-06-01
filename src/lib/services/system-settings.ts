import { invoke } from '@tauri-apps/api/core';

export interface SystemSettingsPayload {
  encountersDbPath?: string;
  encounters_db_path?: string;
  lostArkExePath?: string;
  lost_ark_exe_path?: string;
  loaLogsExePath?: string;
  loa_logs_exe_path?: string;
  startWithWindows?: boolean;
  start_with_windows?: boolean;
  startWithLostArk?: boolean;
  start_with_lost_ark?: boolean;
  startWithLoaLogs?: boolean;
  start_with_loa_logs?: boolean;
  hideOnLaunch?: boolean;
  hide_on_launch?: boolean;
  showSetupGuideButton?: boolean;
  show_setup_guide_button?: boolean;
  showAuthWelcome?: boolean;
  show_auth_welcome?: boolean;
  showHaalsHourglassReminder?: boolean;
  show_haals_hourglass_reminder?: boolean;
}

// Tauri command boundary for Settings > General/System.
export function loadSystemSettingsCommand(): Promise<SystemSettingsPayload> {
  return invoke<SystemSettingsPayload>('get_system_settings');
}

export function setEncountersDbPathCommand(path: string): Promise<void> {
  return invoke('set_encounters_db_path', { path });
}

export function setLostArkExePathCommand(path: string): Promise<void> {
  return invoke('set_lost_ark_exe_path', { path });
}

export function setLoaLogsExePathCommand(path: string): Promise<void> {
  return invoke('set_loa_logs_exe_path', { path });
}

export function setStartWithWindowsCommand(enabled: boolean): Promise<void> {
  return invoke('set_start_with_windows', { enabled });
}

export function setStartWithLostArkCommand(enabled: boolean): Promise<void> {
  return invoke('set_start_with_lost_ark', { enabled });
}

export function setStartWithLoaLogsCommand(enabled: boolean): Promise<void> {
  return invoke('set_start_with_loa_logs', { enabled });
}

export function setShowSetupGuideButtonCommand(enabled: boolean): Promise<void> {
  return invoke('set_show_setup_guide_button', { enabled });
}

export function setShowAuthWelcomeCommand(enabled: boolean): Promise<void> {
  return invoke('set_show_auth_welcome', { enabled });
}

export function setShowHaalsHourglassReminderCommand(enabled: boolean): Promise<void> {
  return invoke('set_show_haals_hourglass_reminder', { enabled });
}

export function setHideOnLaunchCommand(enabled: boolean): Promise<void> {
  return invoke('set_hide_on_launch', { enabled });
}

export function clearUserDataCommand(): Promise<string> {
  return invoke<string>('clear_user_data');
}

export function isLostArkRunningCommand(): Promise<boolean> {
  return invoke<boolean>('is_lost_ark_running');
}

export function isLoaLogsRunningCommand(): Promise<boolean> {
  return invoke<boolean>('is_loa_logs_running');
}

export function getLogContentCommand(): Promise<string> {
  return invoke<string>('get_log_content');
}

export function clearLogCommand(): Promise<void> {
  return invoke('clear_log');
}

export function sendLogReportCommand(): Promise<string> {
  return invoke<string>('send_log_report');
}
