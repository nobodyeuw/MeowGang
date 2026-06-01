import { invoke } from '@tauri-apps/api/core';
import type { ChangelogData, KnownBugsData, UpdateCheckResult } from '$lib/components/updates/types';

// Tauri update/changelog command boundary for the Updates tab.
export async function getAppVersion(): Promise<string> {
  return invoke<string>('get_app_version');
}

export async function checkForAppUpdates(): Promise<UpdateCheckResult> {
  return invoke<UpdateCheckResult>('check_for_updates');
}

export async function installAppUpdate(): Promise<string> {
  return invoke<string>('install_update');
}

export async function loadUpdateResources(): Promise<{
  changelogs: ChangelogData;
  knownBugs: KnownBugsData;
}> {
  const [changelogs, knownBugs] = await Promise.all([
    invoke<ChangelogData>('get_changelogs'),
    invoke<KnownBugsData>('get_known_bugs')
  ]);

  return { changelogs, knownBugs };
}
