import { invoke } from '@tauri-apps/api/core';
import {
  calculateGoldProgress,
  getCurrentCalendarEventIds,
  isRosterTaskTracked,
  shouldCountDaily
} from '$lib/components/dashboard/helpers';
import { GAME_TASKS } from '$lib/data/tasks';
import { RAIDS } from '$lib/data/raids';
import { getTaskIcon } from '$lib/components/dashboard/character-card-helpers';
import type {
  DashboardCharacterData,
  DashboardDailyDetail,
  DashboardFocusEntry,
  DashboardRaidDetail,
  DashboardSnapshot,
  DashboardRosterEventDetail,
  DashboardWeeklyTaskDetail,
  RaidConfigEntry,
  RosterEventProgress
} from '$lib/components/dashboard/types';
import type { Character, Roster } from '$lib/types/store';

export interface DashboardStatsResult {
  totalRaidsCompleted: number;
  totalAdditionalRaidsCompleted: number;
  totalDailiesCompleted: number;
  totalWeekliesCompleted: number;
  totalRaidsPossible: number;
  totalAdditionalRaidsPossible: number;
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
  mismatchGoldBonus: number;
  characterDataMap: Record<string, DashboardCharacterData>;
  raidDetails: DashboardRaidDetail[];
  additionalRaidDetails: DashboardRaidDetail[];
  dailyDetails: DashboardDailyDetail[];
  weeklyTaskDetails: DashboardWeeklyTaskDetail[];
  calendarEventDetails: DashboardRosterEventDetail[];
  argeosDetails: DashboardRosterEventDetail[];
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
  let additionalRaidsCompleted = 0;
  let dailiesCompleted = 0;
  let weekliesCompleted = 0;
  let raidsPossible = 0;
  let additionalRaidsPossible = 0;
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
  const raidDetails: DashboardRaidDetail[] = [];
  const additionalRaidDetails: DashboardRaidDetail[] = [];
  const dailyDetails: DashboardDailyDetail[] = [];
  const weeklyDetailMap = new Map<string, DashboardWeeklyTaskDetail>();
  const calendarEventDetails: DashboardRosterEventDetail[] = [];
  const argeosDetails: DashboardRosterEventDetail[] = [];

  const getRosterName = (rosterId: string) => rosters.find((roster) => roster.id === rosterId)?.roster_name || rosterId;
  const makeFocusEntry = (character: Character): DashboardFocusEntry => ({
    charId: character.char_id,
    charName: character.char_name,
    rosterId: character.roster_id,
    rosterName: getRosterName(character.roster_id)
  });
  const getWeeklyDetail = (taskId: string): DashboardWeeklyTaskDetail => {
    let detail = weeklyDetailMap.get(taskId);
    if (!detail) {
      detail = {
        taskId,
        name: GAME_TASKS[taskId]?.name || taskId,
        icon: getTaskIcon(taskId),
        completed: 0,
        total: 0,
        openCharacters: []
      };
      weeklyDetailMap.set(taskId, detail);
    }
    return detail;
  };

  for (const roster of rosters) {
    const snapshot = await loadDashboardSnapshot(roster.id);
    rosterDataMap[roster.id] = snapshot;

    if (currentCalendarEventIds.length > 0 && snapshot.characters?.length > 0) {
      const rosterCompletionStatus = Object.values(snapshot.completion_by_character || {}).flat();

      for (const eventId of currentCalendarEventIds) {
        const eventProgress = await loadRosterEventProgress(roster.id, eventId);
        calendarEventDetails.push({
          taskId: eventId,
          name: GAME_TASKS[eventId]?.name || eventId,
          icon: getTaskIcon(eventId),
          rosterId: roster.id,
          rosterName: roster.roster_name,
          completedToday: eventProgress.completed_today,
          completedThisWeek: eventProgress.completed_this_week,
          weeklyLimit: eventProgress.weekly_limit,
          available: eventProgress.available
        });
        calendarEventsPossible++;

        if (rosterCompletionStatus.some((completion: any) => completion.content_id === eventId && completion.is_completed === 1)) {
          calendarEventsCompleted++;
        }
      }
    }

    const argeosProgress = await loadRosterEventProgress(roster.id, 'event_argeos_winter');
    const argeosIsTracked = isRosterTaskTracked(snapshot, 'event_argeos_winter');
    const argeosHasProgress = argeosProgress.completed_this_week > 0 || argeosProgress.completed_today;

    if (argeosIsTracked || argeosHasProgress) {
      argeosDetails.push({
        taskId: 'event_argeos_winter',
        name: GAME_TASKS.event_argeos_winter?.name || 'Stoopid Argeos',
        icon: getTaskIcon('event_argeos_winter'),
        rosterId: roster.id,
        rosterName: roster.roster_name,
        completedToday: argeosProgress.completed_today,
        completedThisWeek: argeosProgress.completed_this_week,
        weeklyLimit: argeosProgress.weekly_limit,
        available: argeosProgress.available
      });
      argeosTracked++;

      if (argeosProgress.available) argeosAvailableToday++;
      if (argeosProgress.completed_today) argeosDoneToday++;
      if (argeosProgress.completed_this_week >= argeosProgress.weekly_limit) argeosFullyDone++;
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
      const chaosCompleted = completionStatus.some((completion: any) => completion.content_id === 'chaos' && Number(completion.is_completed) === 1);
      const guardianCompleted = completionStatus.some((completion: any) => completion.content_id === 'guardian' && Number(completion.is_completed) === 1);

      if (chaosConfigured) dailiesTracked++;
      if (guardianConfigured) dailiesTracked++;

      if (chaosTracked || (chaosConfigured && chaosCompleted)) {
        dailiesPossible++;
        if (chaosCompleted) dailiesCompleted++;
      }

      if (guardianTracked || (guardianConfigured && guardianCompleted)) {
        dailiesPossible++;
        if (guardianCompleted) dailiesCompleted++;
      }

      const openDailyTasks = [
        chaosTracked && !chaosCompleted ? 'chaos' : null,
        guardianTracked && !guardianCompleted ? 'guardian' : null
      ].filter(Boolean) as string[];
      if (openDailyTasks.length > 0) {
        dailyDetails.push({
          ...makeFocusEntry(character),
          openTasks: openDailyTasks
        });
      }

      for (const weeklyTask of ['cube', 'paradise', 'shop', 'guild']) {
        const weeklyTracked = trackingStatus.some((tracking: any) => tracking.content_id === weeklyTask && Number(tracking.is_tracked) === 1);
        if (weeklyTracked) {
          const weeklyDetail = getWeeklyDetail(weeklyTask);
          weeklyDetail.total++;
          weekliesPossible++;
          const weeklyCompleted = completionStatus.some((completion: any) => completion.content_id === weeklyTask && completion.is_completed === 1);
          if (weeklyCompleted) {
            weekliesCompleted++;
            weeklyDetail.completed++;
          } else {
            weeklyDetail.openCharacters.push(makeFocusEntry(character));
          }
        }
      }

      if (!character.hide_from_dashboard) {
        const raidConfigs = rosterSnapshot.raid_configs_by_character?.[key] || [];
        const goldRaids = raidConfigs.filter((raid: any) => raid.take_gold === 1);
        const additionalTrackedRaidIds = character.earns_gold
          ? []
          : trackingStatus
              .filter((tracking: any) => Number(tracking.is_tracked) === 1 && RAIDS.some((raid) => raid.id === tracking.content_id))
              .map((tracking: any) => tracking.content_id);
        const uniqueRaidIds = [...new Set(goldRaids.map((raid: any) => raid.content_id))];
        const uniqueAdditionalRaidIds = [...new Set(additionalTrackedRaidIds)];
        raidsPossible += uniqueRaidIds.length;
        additionalRaidsPossible += uniqueAdditionalRaidIds.length;

        for (const raidId of uniqueRaidIds) {
          const isCompleted = completionStatus.some((completion: any) => completion.content_id === raidId && completion.is_completed === 1);
          if (isCompleted) raidsCompleted++;
          raidDetails.push({
            ...makeFocusEntry(character),
            completed: isCompleted
          });
        }

        for (const raidId of uniqueAdditionalRaidIds) {
          const isCompleted = completionStatus.some((completion: any) => completion.content_id === raidId && completion.is_completed === 1);
          if (isCompleted) additionalRaidsCompleted++;
          additionalRaidDetails.push({
            ...makeFocusEntry(character),
            completed: isCompleted
          });
        }
      }
    } catch (error) {
      console.error(`Failed to load stats for character ${character.char_id}:`, error);
    }
  }

  const goldProgress = calculateGoldProgress(visibleCharacters, allRaidConfigsByCharacter, characterDataMap);
  const remainingGoldDisplay = Math.max(goldProgress.plannedGold - goldProgress.clearedPlannedGold, 0);
  const earnedGoldPercentage = goldProgress.plannedGold > 0
    ? Math.min((goldProgress.clearedPlannedGold / goldProgress.plannedGold) * 100, 100)
    : 0;
  const progressPercentage = goldProgress.plannedGold > 0
    ? Math.min((goldProgress.clearedPlannedGold / goldProgress.plannedGold) * 100, 100)
    : 0;

  return {
    totalRaidsCompleted: raidsCompleted,
    totalAdditionalRaidsCompleted: additionalRaidsCompleted,
    totalDailiesCompleted: dailiesCompleted,
    totalWeekliesCompleted: weekliesCompleted,
    totalRaidsPossible: raidsPossible,
    totalAdditionalRaidsPossible: additionalRaidsPossible,
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
    mismatchGoldBonus: goldProgress.bonusGold,
    characterDataMap,
    raidDetails,
    additionalRaidDetails,
    dailyDetails,
    weeklyTaskDetails: Array.from(weeklyDetailMap.values()),
    calendarEventDetails,
    argeosDetails
  };
}
