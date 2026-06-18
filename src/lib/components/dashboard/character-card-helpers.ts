import { GAME_TASKS, RAIDS } from '$lib/data';
import { classAsset, iconAsset } from '$lib/assets';

export interface CharacterCardCompletionEntry {
  content_id: string;
  is_completed: number;
  details?: string | null;
  session_id?: string | null;
}

export interface CharacterCardRaidConfig {
  content_id: string;
  gate?: string;
  difficulty: string;
  take_gold: number;
  buy_box?: number;
  reserved_for_static?: number;
  static_group_tag?: string;
  is_tracked?: number;
}

export interface CharacterCardTrackingEntry {
  content_id: string;
  is_tracked: number;
  lazy_daily?: number;
}

export interface CharacterCardWeeklyTask {
  content_id: string;
  name: string;
  completed: boolean;
}

export interface CharacterCardGroupedRaid {
  content_id: string;
  difficulty: string;
  gate_configs: CharacterCardRaidConfig[];
  take_gold: number;
  reserved_for_static: number;
  static_group_tag: string;
  is_tracked: number;
}

export interface CharacterCardDisplayRaid extends CharacterCardGroupedRaid {
  isGoldRaid: boolean;
  isStaticReserved: boolean;
  staticBadgeText: string;
  isTrackedRaid: boolean;
  completed: boolean;
  gateProgress: { completed: number; total: number };
  completionMismatch: boolean;
  completionTooltip?: string;
}

export function getCompletionStatus(completionStatus: CharacterCardCompletionEntry[], contentId: string): boolean {
  const completion = completionStatus.find((entry) => entry.content_id === contentId);
  return completion?.is_completed === 1;
}

export function getDailyIconTitle(contentId: string, completed: boolean, lazyWaiting: boolean): string {
  const label = contentId === 'chaos' ? 'Chaos Dungeon' : 'Guardian Raid';
  if (completed) return `${label}: done`;
  if (lazyWaiting) return `${label}: resting until 20+ rested`;
  return `${label}: available`;
}

export function normalizeDifficulty(difficulty: string): string {
  const normalized = difficulty.trim().toLowerCase();
  if (normalized.includes('mixed')) return 'Mixed';
  if (normalized.includes('hard')) return 'Hard';
  if (normalized.includes('nightmare')) return 'Nightmare';
  if (normalized.includes('solo')) return 'Solo';
  if (normalized.includes('normal')) return 'Normal';
  return difficulty.charAt(0).toUpperCase() + difficulty.slice(1);
}

export function getAggregatedRaidDifficulty(configs: Array<{ difficulty: string }>): string {
  const difficulties = [...new Set(configs.map((config) => normalizeDifficulty(config.difficulty)).filter(Boolean))];
  if (difficulties.length === 0) return '';
  if (difficulties.length === 1) return difficulties[0];
  if (difficulties.includes('Solo')) return 'Solo';
  return 'Mixed';
}

export function getRaidDefinition(raidId: string, difficulty: string) {
  return RAIDS.find((raid) => raid.id === raidId && raid.difficulty === difficulty)
    ?? RAIDS.find((raid) => raid.id === raidId);
}

export function isRaidGateCompleted(
  completionStatus: CharacterCardCompletionEntry[],
  raidId: string,
  gate: string
): boolean {
  return completionStatus.some((entry) =>
    entry.content_id === raidId &&
    Number(entry.is_completed) === 1 &&
    (entry.session_id ?? '') === `${raidId}_${gate}`
  );
}

export function getRaidGateProgress(
  completionStatus: CharacterCardCompletionEntry[],
  raidId: string,
  difficulty: string
): { completed: number; total: number } {
  const raidDef = getRaidDefinition(raidId, difficulty);
  const total = raidDef?.gates.length ?? 0;

  const completed = completionStatus.filter((entry) =>
    entry.content_id === raidId &&
    entry.is_completed === 1 &&
    (entry.session_id ?? '').startsWith(raidId + '_') &&
    (entry.session_id ?? '').includes('_Gate ')
  ).length;

  return { completed, total };
}

export function getNextOpenGate(
  completionStatus: CharacterCardCompletionEntry[],
  raidId: string,
  difficulty: string
): string | null {
  const raid = getRaidDefinition(raidId, difficulty);
  return raid?.gates.find((gate) => !isRaidGateCompleted(completionStatus, raidId, gate.gate))?.gate ?? null;
}

export function getLastCompletedGate(
  completionStatus: CharacterCardCompletionEntry[],
  raidId: string,
  difficulty: string
): string | null {
  const raid = getRaidDefinition(raidId, difficulty);
  const completedGates = [...(raid?.gates ?? [])].reverse();
  return completedGates.find((gate) => isRaidGateCompleted(completionStatus, raidId, gate.gate))?.gate ?? null;
}

export function getCompletedRaidDetails(
  completionStatus: CharacterCardCompletionEntry[],
  contentId: string
): string | undefined {
  const completedDifficulties = completionStatus
    .filter((candidate) => candidate.content_id === contentId && candidate.is_completed === 1 && candidate.details)
    .map((entry) => normalizeDifficulty(entry.details || ''))
    .filter(Boolean);
  const uniqueDifficulties = [...new Set(completedDifficulties)];

  if (uniqueDifficulties.length > 1) return 'Mixed';
  return uniqueDifficulties[0];
}

export function getRaidSortOrder(contentId: string, difficulty: string): number {
  const raid = getRaidDefinition(contentId, difficulty);
  return raid?.sortOrder || 0;
}

export function getConfiguredRaidOrder(raidConfigs: CharacterCardRaidConfig[], contentId: string): number {
  const index = raidConfigs.findIndex((config) => config.content_id === contentId);
  return index === -1 ? Number.MAX_SAFE_INTEGER : index;
}

export function buildTrackedRaidIds(trackingStatus: CharacterCardTrackingEntry[]): Set<string> {
  return new Set(
    trackingStatus
      .filter((entry) => Number(entry.is_tracked) === 1 && RAIDS.some((raid) => raid.id === entry.content_id))
      .map((entry) => entry.content_id)
  );
}

export function buildGroupedRaids(
  raidConfigs: CharacterCardRaidConfig[],
  trackedRaidIds: Set<string>
): Record<string, CharacterCardGroupedRaid> {
  return raidConfigs.reduce((groups: Record<string, CharacterCardGroupedRaid>, raid) => {
    const key = raid.content_id;
    if (!groups[key]) {
      groups[key] = {
        content_id: raid.content_id,
        difficulty: normalizeDifficulty(raid.difficulty),
        gate_configs: [raid],
        take_gold: Number(raid.take_gold) === 1 ? 1 : 0,
        reserved_for_static: Number(raid.reserved_for_static) === 1 ? 1 : 0,
        static_group_tag: String(raid.static_group_tag || '').trim(),
        is_tracked: trackedRaidIds.has(raid.content_id) ? 1 : 0
      };
    } else {
      groups[key].gate_configs.push(raid);
      groups[key].difficulty = getAggregatedRaidDifficulty(groups[key].gate_configs);
      if (Number(raid.take_gold) === 1) {
        groups[key].take_gold = 1;
      }
    }

    if (Number(raid.reserved_for_static) === 1) {
      groups[key].reserved_for_static = 1;
    }
    if (String(raid.static_group_tag || '').trim()) {
      groups[key].static_group_tag = String(raid.static_group_tag).trim();
    }

    return groups;
  }, {});
}

export function getRaidDisplayName(contentId: string, difficulty: string): string {
  const raid = getRaidDefinition(contentId, difficulty);
  const raidName = raid ? raid.name : contentId;
  const formattedDifficulty = normalizeDifficulty(difficulty);
  return `${raidName} ${formattedDifficulty}`;
}

export function getRaidName(contentId: string, difficulty: string): string {
  const raid = getRaidDefinition(contentId, difficulty);
  return raid ? raid.name : contentId;
}

export function buildDisplayRaids(options: {
  groupedRaids: Record<string, CharacterCardGroupedRaid>;
  completionStatus: CharacterCardCompletionEntry[];
  characterEarnsGold: boolean;
  showStaticBadges: boolean;
  raidConfigs: CharacterCardRaidConfig[];
}): CharacterCardDisplayRaid[] {
  const raids = Object.values(options.groupedRaids)
    .sort((a, b) => {
      return getRaidSortOrder(a.content_id, a.difficulty) - getRaidSortOrder(b.content_id, b.difficulty);
    })
    .map((raid) => {
    const actualDifficulty = getCompletedRaidDetails(options.completionStatus, raid.content_id);
    const plannedDifficulty = normalizeDifficulty(raid.difficulty);
    const mismatch = plannedDifficulty !== 'Mixed' && actualDifficulty != null && actualDifficulty !== plannedDifficulty;

    const gateProgress = getRaidGateProgress(options.completionStatus, raid.content_id, raid.difficulty);
    const fullyCompleted = gateProgress.total > 0 && gateProgress.completed >= gateProgress.total;

    return {
      ...raid,
      isGoldRaid: Number(raid.take_gold) === 1 && options.characterEarnsGold,
      isStaticReserved: options.showStaticBadges && Number(raid.reserved_for_static) === 1,
      staticBadgeText: String(raid.static_group_tag || '').trim() || 'Static',
      isTrackedRaid: Number(raid.is_tracked) === 1 && !options.characterEarnsGold,
      completed: fullyCompleted,
      gateProgress,
      completionMismatch: mismatch,
      completionTooltip: mismatch
        ? `Planned to run ${plannedDifficulty} mode but finished in ${actualDifficulty} mode`
        : undefined
    };
  });

  if (options.characterEarnsGold) {
    return raids
      .filter((raid) => Number(raid.take_gold) === 1)
      .slice(0, 3);
  }

  return raids
    .filter((raid) => Number(raid.is_tracked) === 1);
}

export function buildTrackedWeeklyTasks(
  trackingStatus: CharacterCardTrackingEntry[],
  completionStatus: CharacterCardCompletionEntry[]
): CharacterCardWeeklyTask[] {
  return ['cube', 'paradise', 'shop', 'guild']
    .filter((contentId) => trackingStatus.some((entry) => entry.content_id === contentId && Number(entry.is_tracked) === 1))
    .map((contentId) => ({
      content_id: contentId,
      name: GAME_TASKS[contentId]?.name ?? contentId,
      completed: completionStatus.some((entry) => entry.content_id === contentId && Number(entry.is_completed) === 1)
    }))
    .slice(0, 4);
}

export function formatItemLevel(itemLevel: number): string {
  return itemLevel.toLocaleString('de-DE', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
    useGrouping: false
  });
}

export function formatCombatPower(combatPower: number): string {
  return combatPower.toLocaleString('de-DE', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
    useGrouping: false
  });
}

export function getClassIconUrl(iconId: string): string {
  return classAsset(iconId);
}

export function getTaskIcon(taskId: string): string {
  if (taskId.startsWith('event_')) {
    return iconAsset('event_quest.webp');
  }

  const iconMap: Record<string, string> = {
    chaos: iconAsset('chaos-dungeon.webp'),
    guardian: iconAsset('guardian.png'),
    cube: iconAsset('ebony1720.png'),
    paradise: iconAsset('paradise.webp'),
    shop: iconAsset('daily.webp'),
    guild: iconAsset('guild.webp'),
    gate: iconAsset('chaos_gate.png'),
    boss: iconAsset('boss.png'),
    ship_shop: iconAsset('ship.png')
  };

  return iconMap[taskId] || iconAsset('daily.webp');
}
