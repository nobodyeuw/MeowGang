import { invoke } from '@tauri-apps/api/core';
import {
  calculateGoldProgress,
  getCurrentCalendarEventIds,
  isRosterTaskTracked,
  shouldCountDaily
} from '$lib/components/dashboard/helpers';
import type {
  DashboardCharacterData,
  DashboardSnapshot,
  RaidConfigEntry,
  RosterEventProgress
} from '$lib/components/dashboard/types';
import type { Character, Roster } from '$lib/types/store';

export interface DashboardStatsResult {
  totalRaidsCompleted: number;
  totalDailiesCompleted: number;
  totalWeekliesCompleted: number;
  totalRaidsPossible: number;
  totalDailiesTracked: number;
  totalDailiesPossible: number;
  totalWeekliesPossible: number;
  totalCalendarEventsCompleted: number;
  totalCalendarEventsPossible: number;
  totalArgeosTracked: number;
  totalArgeosAvailableToday: number;
  totalArgeosDoneToday: number;
  totalArgeosFullyDone: number;
  progressPercentage: number;
  earnedGoldPercentage: number;
  actualGoldDisplay: number;
  actualBoundGoldDisplay: number;
  actualTradableGoldDisplay: number;
  estimatedGoldDisplay: number;
  remainingGoldDisplay: number;
  mismatchGoldLost: number;
  characterDataMap: Record<string, DashboardCharacterData>;
}

export function loadDashboardSnapshot(rosterId: string): Promise<DashboardSnapshot> {
  return invoke<DashboardSnapshot>('get_dashboard_snapshot', { rosterId });
}

export function loadRosterEventProgress(rosterId: string, taskId: string): Promise<RosterEventProgress> {
  return invoke<RosterEventProgress>('get_roster_event_progress', { rosterId, taskId });
}

export async function buildDashboardStats(
  rosters: Roster[],
  characters: Character[],
  visibleCharacters: Character[]
): Promise<DashboardStatsResult> {
  let raidsCompleted = 0;
  let dailiesCompleted = 0;
  let weekliesCompleted = 0;
  let raidsPossible = 0;
  let dailiesTracked = 0;
  let dailiesPossible = 0;
  let weekliesPossible = 0;
  let calendarEventsCompleted = 0;
  let calendarEventsPossible = 0;
  let argeosTracked = 0;
  let argeosAvailableToday = 0;
  let argeosDoneToday = 0;
  let argeosFullyDone = 0;
  const currentCalendarEventIds = getCurrentCalendarEventIds();
  const characterDataMap: Record<string, DashboardCharacterData> = {};
  const allRaidConfigsByCharacter: Record<string, RaidConfigEntry[]> = {};
  const rosterDataMap: Record<string, DashboardSnapshot> = {};

  for (const roster of rosters) {
    const snapshot = await loadDashboardSnapshot(roster.id);
    rosterDataMap[roster.id] = snapshot;

    if (currentCalendarEventIds.length > 0 && snapshot.characters?.length > 0) {
      const rosterCompletionStatus = Object.values(snapshot.completion_by_character || {}).flat();

      for (const eventId of currentCalendarEventIds) {
        calendarEventsPossible++;

        if (rosterCompletionStatus.some((completion: any) => completion.content_id === eventId && completion.is_completed === 1)) {
          calendarEventsCompleted++;
        }
      }
    }

    if (isRosterTaskTracked(snapshot, 'event_argeos_winter')) {
      argeosTracked++;
      const progress = await loadRosterEventProgress(roster.id, 'event_argeos_winter');

      if (progress.available) argeosAvailableToday++;
      if (progress.completed_today) argeosDoneToday++;
      if (progress.completed_this_week >= progress.weekly_limit) argeosFullyDone++;
    }
  }

  for (const character of characters) {
    try {
      const key = String(character.char_id);
      const rosterSnapshot = rosterDataMap[character.roster_id];

      if (!rosterSnapshot) continue;

      const completionStatus = rosterSnapshot.completion_by_character?.[key] || [];
      const trackingStatus = rosterSnapshot.tracking_by_character?.[key] || [];
      const restedValues = rosterSnapshot.rested_by_character?.[key] || [];
      const characterRaidConfigs = rosterSnapshot.raid_configs_by_character?.[key] || [];

      characterDataMap[key] = {
        restedValues,
        completionStatus,
        raidConfigs: characterRaidConfigs,
        trackingStatus
      };

      if (!character.hide_from_dashboard && characterRaidConfigs.length > 0) {
        allRaidConfigsByCharacter[key] = characterRaidConfigs;
      }

      const chaosConfigured = trackingStatus.some((tracking: any) => tracking.content_id === 'chaos' && Number(tracking.is_tracked) === 1);
      const guardianConfigured = trackingStatus.some((tracking: any) => tracking.content_id === 'guardian' && Number(tracking.is_tracked) === 1);
      const chaosTracked = shouldCountDaily(trackingStatus, restedValues, 'chaos');
      const guardianTracked = shouldCountDaily(trackingStatus, restedValues, 'guardian');

      if (chaosConfigured) dailiesTracked++;
      if (guardianConfigured) dailiesTracked++;

      if (chaosTracked) {
        dailiesPossible++;
        const chaosCompleted = completionStatus.some((completion: any) => completion.content_id === 'chaos' && completion.is_completed === 1);
        if (chaosCompleted) dailiesCompleted++;
      }

      if (guardianTracked) {
        dailiesPossible++;
        const guardianCompleted = completionStatus.some((completion: any) => completion.content_id === 'guardian' && completion.is_completed === 1);
        if (guardianCompleted) dailiesCompleted++;
      }

      for (const weeklyTask of ['cube', 'paradise', 'shop', 'guild']) {
        const weeklyTracked = trackingStatus.some((tracking: any) => tracking.content_id === weeklyTask && Number(tracking.is_tracked) === 1);
        if (weeklyTracked) {
          weekliesPossible++;
          const weeklyCompleted = completionStatus.some((completion: any) => completion.content_id === weeklyTask && completion.is_completed === 1);
          if (weeklyCompleted) weekliesCompleted++;
        }
      }

      if (!character.hide_from_dashboard) {
        const raidConfigs = rosterSnapshot.raid_configs_by_character?.[key] || [];
        const goldRaids = raidConfigs.filter((raid: any) => raid.take_gold === 1);
        const uniqueRaidIds = [...new Set(goldRaids.map((raid: any) => raid.content_id))];
        raidsPossible += uniqueRaidIds.length;

        for (const raidId of uniqueRaidIds) {
          const isCompleted = completionStatus.some((completion: any) => completion.content_id === raidId && completion.is_completed === 1);
          if (isCompleted) raidsCompleted++;
        }
      }
    } catch (error) {
      console.error(`Failed to load stats for character ${character.char_id}:`, error);
    }
  }

  const goldProgress = calculateGoldProgress(visibleCharacters, allRaidConfigsByCharacter, characterDataMap);
  const resolvedGold = goldProgress.actualGold + goldProgress.lostGold;
  const remainingGoldDisplay = Math.max(goldProgress.plannedGold - resolvedGold, 0);
  const earnedGoldPercentage = goldProgress.plannedGold > 0
    ? Math.min((goldProgress.actualGold / goldProgress.plannedGold) * 100, 100)
    : 0;
  const progressPercentage = goldProgress.plannedGold > 0
    ? Math.min((resolvedGold / goldProgress.plannedGold) * 100, 100)
    : 0;

  return {
    totalRaidsCompleted: raidsCompleted,
    totalDailiesCompleted: dailiesCompleted,
    totalWeekliesCompleted: weekliesCompleted,
    totalRaidsPossible: raidsPossible,
    totalDailiesTracked: dailiesTracked,
    totalDailiesPossible: dailiesPossible,
    totalWeekliesPossible: weekliesPossible,
    totalCalendarEventsCompleted: calendarEventsCompleted,
    totalCalendarEventsPossible: calendarEventsPossible,
    totalArgeosTracked: argeosTracked,
    totalArgeosAvailableToday: argeosAvailableToday,
    totalArgeosDoneToday: argeosDoneToday,
    totalArgeosFullyDone: argeosFullyDone,
    progressPercentage,
    earnedGoldPercentage,
    actualGoldDisplay: goldProgress.actualGold,
    actualBoundGoldDisplay: goldProgress.actualBoundGold,
    actualTradableGoldDisplay: goldProgress.actualTradableGold,
    estimatedGoldDisplay: goldProgress.plannedGold,
    remainingGoldDisplay,
    mismatchGoldLost: goldProgress.lostGold,
    characterDataMap
  };
}
