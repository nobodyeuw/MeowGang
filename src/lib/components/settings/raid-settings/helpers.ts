import { RAIDS } from '$lib/data/raids';
import { getGameClassIconId } from '$lib/data/classes';
import type { CharacterRaidConfig, RaidBulkToggleType, RaidConfig, RaidGroup, RaidMatrixData } from './types';

export const COLLAPSE_UNTRACKED_RAID_ROWS_STORAGE_KEY = 'raidSettings.collapseUntrackedRows';

export function loadCollapseUntrackedRaidRows(): boolean {
  try {
    return localStorage.getItem(COLLAPSE_UNTRACKED_RAID_ROWS_STORAGE_KEY) === '1';
  } catch {
    return false;
  }
}

export function saveCollapseUntrackedRaidRows(value: boolean) {
  try {
    localStorage.setItem(COLLAPSE_UNTRACKED_RAID_ROWS_STORAGE_KEY, value ? '1' : '0');
  } catch {
    // Ignore storage failures; the in-memory view state still updates.
  }
}

export function isRaidRowEnabled(raid: RaidMatrixData): boolean {
  return raid.characters.some((char) => {
    if ((char.tracked_raid_ids || []).includes(raid.content_id)) return true;
    const config = char.raid_configs.find((raidConfig) => raidConfig.content_id === raid.content_id);
    return Boolean(config?.gates?.some((gate) => gate.take_gold || gate.buy_box || gate.reserved_for_static));
  });
}

export function getTrackedRaidIdsForCharacter(trackingStates: Array<{ char_id: number; tracked?: boolean; content_id: string }>, charId: number): string[] {
  return trackingStates
    .filter((state) =>
      state.char_id === charId &&
      state.tracked === true &&
      RAIDS.some((raid) => raid.id === state.content_id)
    )
    .map((state) => state.content_id);
}

export function buildRaidGroups(): RaidGroup[] {
  const raidsMap = new Map<string, typeof RAIDS>();
  [...RAIDS].forEach((raid) => {
    const baseName = raid.name;
    if (!raidsMap.has(baseName)) {
      raidsMap.set(baseName, []);
    }
    raidsMap.get(baseName)!.push(raid);
  });

  const sortedRaidGroups = Array.from(raidsMap.entries()).map(([baseName, raids]) => {
    const sortedRaids = raids.sort((a, b) => {
      const aMinIlvl = a.gates[0]?.minIlvl || 0;
      const bMinIlvl = b.gates[0]?.minIlvl || 0;
      return aMinIlvl - bMinIlvl;
    });
    return { baseName, raids: sortedRaids };
  }).sort((a, b) => {
    const aMinIlvl = a.raids[0]?.gates[0]?.minIlvl || 0;
    const bMinIlvl = b.raids[0]?.gates[0]?.minIlvl || 0;
    return aMinIlvl - bMinIlvl;
  });

  return sortedRaidGroups.map(({ raids, baseName }) => {
    const group: RaidGroup = {
      content_id: raids[0].id,
      raid_name: baseName,
      difficulties: new Map(),
      gates: new Map()
    };

    raids.forEach((raid) => {
      group.difficulties.set(raid.difficulty, raid);
      raid.gates.forEach((gate) => {
        if (!group.gates.has(gate.gate)) {
          group.gates.set(gate.gate, new Map());
        }
        group.gates.get(gate.gate)!.set(raid.difficulty, gate);
      });
    });

    return group;
  });
}

export function buildRaidMatrixData(
  characters: any[],
  raidConfigs: any[],
  trackingStates: Array<{ char_id: number; tracked?: boolean; content_id: string }>,
  expandedRaids: Set<string>
): RaidMatrixData[] {
  return buildRaidGroups().map((raidGroup) => {
    const characterConfigs = characters.map((char: any) => {
      const charRaidConfig = raidConfigs.find((config: any) => config.char_id === char.char_id);

      if (!charRaidConfig) {
        return {
          char_id: char.char_id,
          char_name: char.char_name,
          item_level: char.item_level,
          combat_power: char.combat_power || 0,
          class_id: char.class_id,
          earns_gold: char.earns_gold,
          raid_configs: [],
          tracked_raid_ids: getTrackedRaidIdsForCharacter(trackingStates, char.char_id),
          available_difficulties: [],
          master_difficulty: '',
          is_locked: true,
          gold_values: { totalGold: 0, tradableGold: 0, boundGold: 0, boxPrice: 0 }
        };
      }

      const availableDifficulties = getAvailableDifficulties(raidGroup, char.item_level);
      const masterDifficulty = getMasterDifficulty(raidGroup, charRaidConfig.raid_configs);
      const currentRaidConfig = charRaidConfig.raid_configs.find((config: RaidConfig) => config.content_id === raidGroup.content_id);
      const goldValues = currentRaidConfig
        ? calculateRaidGoldValuesFromConfig(currentRaidConfig, raidGroup.content_id)
        : { totalGold: 0, tradableGold: 0, boundGold: 0, boxPrice: 0 };

      return {
        char_id: char.char_id,
        char_name: char.char_name,
        item_level: char.item_level,
        combat_power: char.combat_power || 0,
        class_id: char.class_id,
        earns_gold: char.earns_gold,
        raid_configs: charRaidConfig.raid_configs.map((config: any) => ({
          ...config,
          gates: config.gates ? config.gates.map((gate: any) => ({ ...gate })) : []
        })),
        tracked_raid_ids: getTrackedRaidIdsForCharacter(trackingStates, char.char_id),
        available_difficulties: availableDifficulties,
        master_difficulty: masterDifficulty,
        is_locked: availableDifficulties.length === 0,
        gold_values: goldValues
      };
    });

    return {
      ...raidGroup,
      unique_key: raidGroup.content_id,
      characters: characterConfigs,
      is_expanded: expandedRaids.has(raidGroup.content_id)
    };
  });
}

export function getAvailableDifficulties(raidGroup: RaidGroup, itemLevel: number): string[] {
  const difficulties: string[] = [];

  raidGroup.difficulties.forEach((raid, difficulty) => {
    const canDoRaid = raid.gates.some((gate) => gate.minIlvl <= itemLevel);
    if (canDoRaid) {
      difficulties.push(difficulty);
    }
  });

  const priority: Record<string, number> = { Hard: 3, Normal: 2, Solo: 1, Nightmare: 4 };
  return difficulties.sort((a, b) => (priority[b] || 0) - (priority[a] || 0));
}

export function getMasterDifficulty(raidGroup: RaidGroup, raidConfigs: RaidConfig[]): string {
  const raidConfig = raidConfigs.find((config) => config.content_id === raidGroup.content_id);

  if (!raidConfig || raidConfig.gates.length === 0) {
    return '';
  }

  const difficulties = raidConfig.gates.map((gate) => gate.difficulty);
  const uniqueDifficulties = [...new Set(difficulties)];

  if (uniqueDifficulties.length === 1) {
    return uniqueDifficulties[0] || '';
  }

  if (uniqueDifficulties.includes('Solo')) {
    return 'Solo';
  }

  return 'Mixed';
}

export function hasMixedDifficulties(char: Pick<CharacterRaidConfig, 'raid_configs'>, raidId: string): boolean {
  const raidConfig = char.raid_configs.find((raid) => raid.content_id === raidId);
  if (!raidConfig || !raidConfig.gates || raidConfig.gates.length === 0) return false;

  const difficulties = raidConfig.gates.map((gate) => gate.difficulty);
  const uniqueDifficulties = [...new Set(difficulties)];

  if (uniqueDifficulties.includes('Solo')) return false;

  return uniqueDifficulties.length > 1;
}

export function calculateRaidGoldValuesFromConfig(raidConfig: RaidConfig, raidId: string) {
  let totalGold = 0;
  let tradableGold = 0;
  let boundGold = 0;
  let boxPrice = 0;

  raidConfig.gates.forEach((gate) => {
    const gateData = getGateData(raidId, gate.difficulty || 'Solo', gate.gate);
    if (gateData) {
      totalGold += gateData.tradableGold + gateData.boundGold;
      tradableGold += gateData.tradableGold;
      boundGold += gateData.boundGold;
      if (gate.buy_box) {
        boxPrice += gateData.boxPrice;
      }
    }
  });

  return { totalGold, tradableGold, boundGold, boxPrice };
}

export function getRaidGoldValues(char: CharacterRaidConfig, raidId: string) {
  const raidConfig = char.raid_configs.find((raid) => raid.content_id === raidId);
  if (!raidConfig || !raidConfig.gates || raidConfig.gates.length === 0) {
    return { totalGold: 0, tradableGold: 0, boundGold: 0, boxPrice: 0 };
  }

  return calculateRaidGoldValuesFromConfig(raidConfig, raidId);
}

export function getPotentialBoxPrice(char: CharacterRaidConfig, raidId: string): number {
  const raidConfig = char.raid_configs.find((raid) => raid.content_id === raidId);
  if (!raidConfig || !raidConfig.gates || raidConfig.gates.length === 0) {
    return 0;
  }

  let boxPrice = 0;
  raidConfig.gates.forEach((gate) => {
    const gateData = getGateData(raidId, gate.difficulty || 'Solo', gate.gate);
    if (gateData) {
      boxPrice += gateData.boxPrice;
    }
  });

  return boxPrice;
}

export function getPotentialGoldAmount(char: CharacterRaidConfig, raidId: string): number {
  const raidConfig = char.raid_configs.find((raid) => raid.content_id === raidId);
  if (!raidConfig || !raidConfig.gates || raidConfig.gates.length === 0) {
    return 0;
  }

  let goldAmount = 0;
  raidConfig.gates.forEach((gate) => {
    const gateData = getGateData(raidId, gate.difficulty || 'Solo', gate.gate);
    if (gateData) {
      goldAmount += gateData.tradableGold + gateData.boundGold;
    }
  });

  return goldAmount;
}

export function getGateData(raidId: string, difficulty: string, gateName: string) {
  const raid = RAIDS.find((entry) => entry.id === raidId && entry.difficulty === difficulty);
  return raid?.gates.find((gate) => gate.gate === gateName);
}

export function getRaidDefinition(raidId: string, difficulty: string) {
  return RAIDS.find((entry) => entry.id === raidId && entry.difficulty === difficulty);
}

export function isMasterActive(
  char: CharacterRaidConfig,
  raidId: string,
  _difficulty: string,
  type: 'take_gold' | 'buy_box' | 'reserved_for_static'
): boolean {
  const raidConfig = char.raid_configs.find((raid) => raid.content_id === raidId);
  if (!raidConfig || !raidConfig.gates || raidConfig.gates.length === 0) return false;

  return raidConfig.gates.every((gate) => gate[type] === true);
}

export function cloneRaidConfigs(raidConfigs: RaidConfig[]): RaidConfig[] {
  return raidConfigs.map((config) => ({
    ...config,
    gates: config.gates ? config.gates.map((gate) => ({ ...gate })) : []
  }));
}

export function hasReachedGoldLimit(char: CharacterRaidConfig): boolean {
  if (!char.earns_gold) return false;

  const currentGoldRaidsCount = char.raid_configs.filter((raid) =>
    raid.gates && raid.gates.some((gate) => gate.take_gold === true)
  ).length;

  return currentGoldRaidsCount >= 3;
}

export function isRaidAlreadyActive(char: CharacterRaidConfig, raidId: string): boolean {
  const raidConfig = char.raid_configs.find((raid) => raid.content_id === raidId);
  return Boolean(raidConfig?.gates?.some((gate) => gate.take_gold === true));
}

export function getBulkToggleCharacters(raid: RaidMatrixData, type: RaidBulkToggleType, targetValue?: boolean): CharacterRaidConfig[] {
  return raid.characters.filter((char) => {
    if (char.is_locked) return false;
    if (type === 'take_gold' && !char.earns_gold) return false;
    if (type === 'take_gold' && targetValue === true && hasReachedGoldLimit(char) && !isRaidAlreadyActive(char, raid.content_id)) {
      return false;
    }
    return Boolean(getRaidDefinition(raid.content_id, char.master_difficulty));
  });
}

export function getTotalGold(raidGroup: RaidGroup, difficulty: string): number {
  const raid = raidGroup.difficulties.get(difficulty);
  if (!raid) return 0;

  return raid.gates.reduce((total, gate) => total + gate.tradableGold + gate.boundGold, 0);
}

export function getTotalBoxPrice(raidGroup: RaidGroup, difficulty: string): number {
  const raid = raidGroup.difficulties.get(difficulty);
  if (!raid) return 0;

  return raid.gates.reduce((total, gate) => total + gate.boxPrice, 0);
}

export function getClassIcon(classId: string): string {
  return getGameClassIconId(classId);
}
