import type { Raid } from '$lib/data/raids';
import type { MeowConnectAvailabilityRow } from '$lib/services/meow-connect';

export interface ProfileRaidGroup {
  key: string;
  raidId: string;
  ownerId: string;
  ownerName: string;
  ownerAvatarUrl?: string;
  rows: MeowConnectAvailabilityRow[];
  openCount: number;
  clearedCount: number;
  reservedCount: number;
  favoriteCount: number;
  raidName: string;
  minIlvl: number;
}

export interface RaidTogetherRow {
  key: string;
  raidName: string;
  minIlvl: number;
  togetherCount: number;
  groups: ProfileRaidGroup[];
}

export interface RaidDifficultyFilterItem {
  raid: Raid;
  difficulties: string[];
  selectedDifficulty: string;
}

export interface FriendOption {
  id: string;
  name: string;
}
