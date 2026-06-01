import type { Raid } from '$lib/data/raids';
import type { MeowConnectAvailabilityRow } from '$lib/services/meow-connect';
import type { ProfileRaidGroup } from './types';

export function getAvailabilityDifficultyLabel(row: MeowConnectAvailabilityRow): string {
  return row.status === 'cleared'
    ? row.clearedDifficulty || row.raid.difficulty
    : row.raid.difficulty;
}

export function getProfileGroupIlvlLabel(
  group: ProfileRaidGroup,
  raids: Raid[],
  raidDifficultyFilters: Record<string, string>
): string {
  const selectedDifficulty = raidDifficultyFilters[group.raidId];
  const raid = selectedDifficulty
    ? raids.find((entry) => entry.id === group.raidId && entry.difficulty === selectedDifficulty)
    : raids
      .filter((entry) => entry.id === group.raidId && entry.difficulty.toLowerCase() !== 'solo')
      .sort((a, b) => a.gates[0].minIlvl - b.gates[0].minIlvl)[0];

  return `${raid?.gates[0]?.minIlvl || group.minIlvl}+`;
}

