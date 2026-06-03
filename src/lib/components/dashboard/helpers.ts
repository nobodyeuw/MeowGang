import type { Character } from '$lib/store';
import { iconAsset } from '$lib/assets';
import { RAIDS, type Raid } from '$lib/data/raids';
import { getCurrentAvailabilityStatus } from '$lib/utils/availability';
import type {
  ArgeosStatusKind,
  CompletionStatusEntry,
  DashboardCharacterData,
  DashboardSnapshot,
  OpenStatusKind,
  RaidConfigEntry,
  RestedValueEntry,
  TrackingStatusEntry
} from '$lib/components/dashboard/types';

const raidLookup: Record<string, Raid> = {};
for (const raid of RAIDS) {
  raidLookup[`${raid.id}-${raid.difficulty}`] = raid;
}

export function getGateIdFromSession(sessionId?: string | null): string | null {
  if (!sessionId) return null;
  const parts = sessionId.split('_');
  return parts.length > 1 ? parts[parts.length - 1] : null;
}

export function findRaid(contentId: string, difficulty?: string | null): Raid | undefined {
  const normalizedDifficulty = (difficulty ?? '').trim().toLowerCase();
  return Object.values(raidLookup).find(
    (raid) => raid.id === contentId && raid.difficulty.trim().toLowerCase() === normalizedDifficulty
  );
}

export function getGateGoldBreakdown(
  contentId: string,
  difficulty: string | null | undefined,
  gateId: string,
  buyBox: number
): { boundGold: number; tradableGold: number; totalGold: number } {
  const raid = findRaid(contentId, difficulty);
  const gate = raid?.gates.find((candidate) => candidate.gate === gateId);
  if (!gate) return { boundGold: 0, tradableGold: 0, totalGold: 0 };

  const boundGold = gate.boundGold || 0;
  const boxPrice = buyBox === 1 ? (gate.boxPrice || 0) : 0;
  const tradableGold = (gate.tradableGold || 0) - boxPrice;
  return {
    boundGold,
    tradableGold,
    totalGold: boundGold + tradableGold
  };
}

export function calculateGoldProgress(
  characters: Character[],
  raidConfigsByCharacter: Record<string, RaidConfigEntry[]>,
  characterDataMap: Record<string, DashboardCharacterData>
): {
  plannedGold: number;
  clearedPlannedGold: number;
  actualGold: number;
  actualBoundGold: number;
  actualTradableGold: number;
  lostGold: number;
  bonusGold: number;
} {
  let plannedGold = 0;
  let clearedPlannedGold = 0;
  let actualGold = 0;
  let actualBoundGold = 0;
  let actualTradableGold = 0;
  let lostGold = 0;
  let bonusGold = 0;

  for (const character of characters) {
    if (!character.earns_gold) continue;

    try {
      const charKey = String(character.char_id);
      const raidConfigs = raidConfigsByCharacter[charKey] || [];
      const goldRaids = raidConfigs.filter((config) => config.take_gold === 1);
      const completionData = characterDataMap[charKey]?.completionStatus ?? [];
      const countedGates = new Set<string>();

      for (const config of goldRaids) {
        const gateId = config.gate;
        const gateKey = `${config.content_id}-${config.difficulty}-${gateId}`;
        if (countedGates.has(gateKey)) continue;
        countedGates.add(gateKey);

        const plannedGateGold = getGateGoldBreakdown(config.content_id, config.difficulty, gateId, config.buy_box);
        plannedGold += plannedGateGold.totalGold;

        const clearedEntry = completionData.find((entry: CompletionStatusEntry) =>
          entry.content_id === config.content_id &&
          entry.is_completed === 1 &&
          entry.details &&
          getGateIdFromSession(entry.session_id) === gateId
        );
        if (!clearedEntry) continue;

        clearedPlannedGold += plannedGateGold.totalGold;

        const actualGateGold = getGateGoldBreakdown(config.content_id, clearedEntry.details, gateId, config.buy_box);
        actualGold += actualGateGold.totalGold;
        actualBoundGold += actualGateGold.boundGold;
        actualTradableGold += actualGateGold.tradableGold;

        const diff = plannedGateGold.totalGold - actualGateGold.totalGold;
        if (diff > 0) {
          lostGold += diff;
        } else if (diff < 0) {
          bonusGold += Math.abs(diff);
        }
      }
    } catch (error) {
      console.error(`Failed to calculate gold for character ${character.char_id}:`, error);
    }
  }

  return { plannedGold, clearedPlannedGold, actualGold, actualBoundGold, actualTradableGold, lostGold, bonusGold };
}

export function getRestedValue(restedValues: RestedValueEntry[], contentId: string): number {
  return restedValues.find((value) => value.content_id === contentId)?.current_value || 0;
}

export function shouldCountDaily(
  trackingStatus: TrackingStatusEntry[],
  restedValues: RestedValueEntry[],
  contentId: string
): boolean {
  const tracking = trackingStatus.find((status) => status.content_id === contentId && Number(status.is_tracked) === 1);
  if (!tracking) return false;
  if (Number(tracking.lazy_daily) === 1) {
    return getRestedValue(restedValues, contentId) >= 20;
  }
  return true;
}

export function getOpenCount(completed: number, possible: number): number {
  return Math.max(possible - completed, 0);
}

export function getOpenStatusKind(completed: number, possible: number, configured = possible): OpenStatusKind {
  if (possible <= 0) return configured > 0 ? 'idle' : 'empty';
  return getOpenCount(completed, possible) === 0 ? 'done' : 'open';
}

export function isRosterTaskTracked(snapshot: DashboardSnapshot, taskId: string): boolean {
  // Roster-wide tasks, like temporary event tasks, are stored once per roster
  // conceptually even though the matrix expands rows per character.
  if (snapshot.roster_tracking_status?.some((entry) => entry.content_id === taskId && Number(entry.is_tracked) === 1)) {
    return true;
  }

  return Object.values(snapshot.tracking_by_character || {})
    .flat()
    .some((entry) => entry.content_id === taskId && Number(entry.is_tracked) === 1);
}

export function getArgeosStatusKind(
  totalArgeosTracked: number,
  totalArgeosAvailableToday: number,
  totalArgeosDoneToday: number,
  totalArgeosFullyDone: number
): ArgeosStatusKind {
  if (totalArgeosTracked <= 0) return 'empty';
  if (totalArgeosFullyDone >= totalArgeosTracked) return 'done';
  if (totalArgeosAvailableToday > 0) return 'open';
  if (totalArgeosDoneToday > 0) return 'today';
  return 'empty';
}

export function getCurrentCalendarEventLabel(): string {
  const availability = getCurrentAvailabilityStatus();

  if (availability.gate && availability.boss) {
    return 'Chaos Gate | Field Boss';
  }

  if (availability.gate) {
    return 'Chaos Gate';
  }

  if (availability.boss) {
    return 'Field Boss';
  }

  return 'No Event';
}

export function getCurrentCalendarEventIds(): string[] {
  const availability = getCurrentAvailabilityStatus();
  const eventIds: string[] = [];

  if (availability.gate) {
    eventIds.push('gate');
  }

  if (availability.boss) {
    eventIds.push('boss');
  }

  return eventIds;
}

export function getCurrentCalendarEventIcons(): string[] {
  const availability = getCurrentAvailabilityStatus();
  const icons: string[] = [];

  if (availability.gate) {
    icons.push(iconAsset('chaos_gate.png'));
  }

  if (availability.boss) {
    icons.push(iconAsset('boss.png'));
  }

  return icons.length > 0 ? icons : [iconAsset('calendar_7743808.png')];
}
