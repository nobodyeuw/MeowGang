// Accessory roll source of truth for progression analysis.
// Values are what the game displays; damagePercent is the player-calculated DPS gain.

export type AccessorySlot = 'necklace' | 'earring' | 'ring';
export type AccessoryRole = 'dps' | 'support';
export type AccessoryRollGrade = 'low' | 'mid' | 'high';
export type AccessoryRollColor = 'blue' | 'purple' | 'gold';

export interface AccessoryRollValue {
  grade: AccessoryRollGrade;
  color: AccessoryRollColor;
  value: number;
  damagePercent: number;
}

export interface AccessoryRollDefinition {
  id: string;
  label: string;
  slots: AccessorySlot[];
  rolls: Record<AccessoryRollGrade, AccessoryRollValue>;
}

export interface AccessoryCombinationSummary {
  slug: string;
  role: AccessoryRole;
  slot: AccessorySlot;
  stats: Array<{
    statId: string;
    label: string;
    grade: AccessoryRollGrade;
    color: AccessoryRollColor;
    value: number;
    damagePercent: number;
  }>;
  combinedDamagePercent: number;
}

export interface AccessoryUpgradeCandidate {
  slug: string;
  role: AccessoryRole;
  slot: AccessorySlot;
  stats: AccessoryCombinationSummary['stats'];
  combinedDamagePercent: number;
  damageGainPercent: number;
}

const GRADE_COLORS: Record<AccessoryRollGrade, AccessoryRollColor> = {
  low: 'blue',
  mid: 'purple',
  high: 'gold'
};

function roll(grade: AccessoryRollGrade, value: number, damagePercent: number): AccessoryRollValue {
  return {
    grade,
    color: GRADE_COLORS[grade],
    value,
    damagePercent
  };
}

export const ACCESSORY_SLOT_STATS: Record<AccessorySlot, [string, string]> = {
  necklace: ['additional-damage', 'outgoing-damage'],
  earring: ['attack-power-percent', 'weapon-power-percent'],
  ring: ['crit-rate', 'crit-damage']
};

export const ACCESSORY_ROLL_DEFINITIONS: Record<string, AccessoryRollDefinition> = {
  'additional-damage': {
    id: 'additional-damage',
    label: 'Additional Damage',
    slots: ['necklace'],
    rolls: {
      low: roll('low', 0.7, 0.5),
      mid: roll('mid', 1.6, 1.15),
      high: roll('high', 2.6, 1.86)
    }
  },
  'outgoing-damage': {
    id: 'outgoing-damage',
    label: 'Outgoing Damage',
    slots: ['necklace'],
    rolls: {
      low: roll('low', 0.55, 0.55),
      mid: roll('mid', 1.2, 1.2),
      high: roll('high', 2, 2)
    }
  },
  'attack-power-percent': {
    id: 'attack-power-percent',
    label: 'Attack Power',
    slots: ['earring'],
    rolls: {
      // Lostark.bible DPS example: Atk. Power +0.95% -> +0.95%.
      low: roll('low', 0.4, 0.4),
      mid: roll('mid', 0.95, 0.95),
      high: roll('high', 1.55, 1.55)
    }
  },
  'weapon-power-percent': {
    id: 'weapon-power-percent',
    label: 'Weapon Power',
    slots: ['earring'],
    rolls: {
      // Weapon-power lines are Base Stats in Lostark.bible.
      // Estimated as Base Atk Power gain: sqrt(1 + weaponPowerPercent) - 1.
      low: roll('low', 0.8, 0.4),
      mid: roll('mid', 1.8, 0.9),
      high: roll('high', 3, 1.49)
    }
  },
  'crit-rate': {
    id: 'crit-rate',
    label: 'Crit Rate',
    slots: ['ring'],
    rolls: {
      // Lostark.bible DPS example: Crit Rate +0.95% -> +0.74%.
      // Low/high scale by the same factor until exact lines are collected.
      low: roll('low', 0.4, 0.31),
      mid: roll('mid', 0.95, 0.74),
      high: roll('high', 1.55, 1.21)
    }
  },
  'crit-damage': {
    id: 'crit-damage',
    label: 'Crit Damage',
    slots: ['ring'],
    rolls: {
      // Lostark.bible DPS examples: Crit Damage +2.4% -> +0.72%, +4% -> +1.20%.
      // Low scales by the same 0.3 factor until its exact line is collected.
      low: roll('low', 1.1, 0.33),
      mid: roll('mid', 2.4, 0.72),
      high: roll('high', 4, 1.2)
    }
  }
};

export const SUPPORT_ACCESSORY_ROLL_DEFINITIONS: Record<string, AccessoryRollDefinition> = {
  'brand-power': {
    id: 'brand-power',
    label: 'Brand Power',
    slots: ['necklace'],
    rolls: {
      // Lostark.bible combat-power breakdown shows high Brand Power +8% as about +4.8%.
      // Low/mid currently scale from that known high value until exact breakdown values are collected.
      low: roll('low', 2.15, 1.29),
      mid: roll('mid', 4.8, 2.88),
      high: roll('high', 8, 4.8)
    }
  },
  'identity-gain': {
    id: 'identity-gain',
    label: 'Identity Gain',
    slots: ['necklace'],
    rolls: {
      // Lostark.bible combat-power example: +6% meter gain -> +3.00%.
      // Low/mid scale by the same 0.5 factor until exact lines are collected.
      low: roll('low', 1.6, 0.8),
      mid: roll('mid', 3.6, 1.8),
      high: roll('high', 6, 3)
    }
  },
  'support-weapon-power-percent': {
    id: 'support-weapon-power-percent',
    label: 'Weapon Power',
    slots: ['earring'],
    rolls: {
      // Support weapon-power lines are Base Stats in Lostark.bible.
      // Estimated as Base Atk Power gain: sqrt(1 + weaponPowerPercent) - 1.
      low: roll('low', 0.8, 0.4),
      mid: roll('mid', 1.8, 0.9),
      high: roll('high', 3, 1.49)
    }
  },
  'support-flat-weapon-power': {
    id: 'support-flat-weapon-power',
    label: 'Weapon Power Flat',
    slots: ['earring'],
    rolls: {
      // Flat weapon power is also Base Stats. Estimated against a 200k weapon-power baseline:
      // sqrt((200000 + flatWeaponPower) / 200000) - 1.
      low: roll('low', 195, 0.05),
      mid: roll('mid', 480, 0.12),
      high: roll('high', 960, 0.24)
    }
  },
  'ally-attack-power': {
    id: 'ally-attack-power',
    label: 'Ally Attack Power',
    slots: ['ring'],
    rolls: {
      // Lostark.bible combat-power examples: +3% -> +2.25%, +5% -> +3.75%.
      // Low scales by the same 0.75 factor until its exact line is collected.
      low: roll('low', 1.35, 1.01),
      mid: roll('mid', 3, 2.25),
      high: roll('high', 5, 3.75)
    }
  },
  'ally-damage': {
    id: 'ally-damage',
    label: 'Ally Damage',
    slots: ['ring'],
    rolls: {
      // Lostark.bible combat-power examples: +2% -> +1.00%, +4.5% -> +2.25%.
      // High scales by the same 0.5 factor until its exact line is collected.
      low: roll('low', 2, 1),
      mid: roll('mid', 4.5, 2.25),
      high: roll('high', 7.5, 3.75)
    }
  }
};

export const COMMON_ACCESSORY_ROLL_DEFINITIONS: Record<string, AccessoryRollDefinition> = {
  'flat-weapon-power': {
    id: 'flat-weapon-power',
    label: 'Weapon Power',
    slots: ['necklace', 'earring', 'ring'],
    rolls: {
      // Flat weapon power is Base Stats. Estimated against a 220k weapon-power DPS baseline:
      // sqrt((220000 + flatWeaponPower) / 220000) - 1.
      low: roll('low', 195, 0.04),
      mid: roll('mid', 480, 0.11),
      high: roll('high', 960, 0.22)
    }
  },
  'flat-attack-power': {
    id: 'flat-attack-power',
    label: 'Attack Power',
    slots: ['necklace', 'earring', 'ring'],
    rolls: {
      low: roll('low', 80, 0.07),
      mid: roll('mid', 195, 0.17),
      high: roll('high', 390, 0.34)
    }
  }
};

const ACCESSORY_MARKET_SLUG_PATTERNS: Array<{
  role: AccessoryRole;
  slot: AccessorySlot;
  regex: RegExp;
  statIds: [string, string];
  definitions: Record<string, AccessoryRollDefinition>;
}> = [
  {
    role: 'dps',
    slot: 'necklace',
    regex: /^accessory-necklace-add-(low|mid|high|none)-out-(low|mid|high|none)$/,
    statIds: ['additional-damage', 'outgoing-damage'],
    definitions: ACCESSORY_ROLL_DEFINITIONS
  },
  {
    role: 'dps',
    slot: 'earring',
    regex: /^accessory-earring-atk-(low|mid|high|none)-weapon-(low|mid|high|none)$/,
    statIds: ['attack-power-percent', 'weapon-power-percent'],
    definitions: ACCESSORY_ROLL_DEFINITIONS
  },
  {
    role: 'dps',
    slot: 'ring',
    regex: /^accessory-ring-crit-rate-(low|mid|high|none)-crit-damage-(low|mid|high|none)$/,
    statIds: ['crit-rate', 'crit-damage'],
    definitions: ACCESSORY_ROLL_DEFINITIONS
  },
  {
    role: 'support',
    slot: 'necklace',
    regex: /^support-accessory-necklace-brand-(low|mid|high|none)-identity-(low|mid|high|none)$/,
    statIds: ['brand-power', 'identity-gain'],
    definitions: SUPPORT_ACCESSORY_ROLL_DEFINITIONS
  },
  {
    role: 'support',
    slot: 'earring',
    regex: /^support-accessory-earring-weapon-(low|mid|high|none)-flat-weapon-(low|mid|high|none)$/,
    statIds: ['support-weapon-power-percent', 'support-flat-weapon-power'],
    definitions: SUPPORT_ACCESSORY_ROLL_DEFINITIONS
  },
  {
    role: 'support',
    slot: 'ring',
    regex: /^support-accessory-ring-ally-atk-(low|mid|high|none)-ally-dmg-(low|mid|high|none)$/,
    statIds: ['ally-attack-power', 'ally-damage'],
    definitions: SUPPORT_ACCESSORY_ROLL_DEFINITIONS
  }
];

const ACCESSORY_MARKET_SLUG_BUILDERS: Record<AccessoryRole, Record<AccessorySlot, {
  buildSlug: (primary: AccessoryRollGrade | 'none', secondary: AccessoryRollGrade | 'none') => string;
}>> = {
  dps: {
    necklace: {
      buildSlug: (additional, outgoing) => `accessory-necklace-add-${additional}-out-${outgoing}`
    },
    earring: {
      buildSlug: (attackPower, weaponPower) => `accessory-earring-atk-${attackPower}-weapon-${weaponPower}`
    },
    ring: {
      buildSlug: (critRate, critDamage) => `accessory-ring-crit-rate-${critRate}-crit-damage-${critDamage}`
    }
  },
  support: {
    necklace: {
      buildSlug: (brand, identity) => `support-accessory-necklace-brand-${brand}-identity-${identity}`
    },
    earring: {
      buildSlug: (weaponPower, flatWeaponPower) => `support-accessory-earring-weapon-${weaponPower}-flat-weapon-${flatWeaponPower}`
    },
    ring: {
      buildSlug: (allyAttackPower, allyDamage) => `support-accessory-ring-ally-atk-${allyAttackPower}-ally-dmg-${allyDamage}`
    }
  }
};

const ACCESSORY_GRADE_SCORE: Record<AccessoryRollGrade, number> = {
  low: 0,
  mid: 1,
  high: 2
};

export function combineDamagePercents(damagePercents: number[]): number {
  const multiplier = damagePercents.reduce((product, damagePercent) => product * (1 + damagePercent / 100), 1);
  return (multiplier - 1) * 100;
}

export function getAccessoryCombinationSummary(slug: string): AccessoryCombinationSummary | undefined {
  for (const pattern of ACCESSORY_MARKET_SLUG_PATTERNS) {
    const match = slug.match(pattern.regex);
    if (!match) continue;

    const grades = [match[1], match[2]] as Array<AccessoryRollGrade | 'none'>;
    const stats = grades.flatMap((grade, index) => {
      if (grade === 'none') return [];
      const statId = pattern.statIds[index];
      const definition = pattern.definitions[statId];
      const rollValue = definition.rolls[grade];
      return [{
        statId,
        label: definition.label,
        grade,
        color: rollValue.color,
        value: rollValue.value,
        damagePercent: rollValue.damagePercent
      }];
    });

    return {
      slug,
      role: pattern.role,
      slot: pattern.slot,
      stats,
      combinedDamagePercent: combineDamagePercents(stats.map((stat) => stat.damagePercent))
    };
  }

  return undefined;
}

export function listAccessoryUpgradeCandidates(
  currentSlug: string,
  minimumGainPercent = 0.01
): AccessoryUpgradeCandidate[] {
  const current = getAccessoryCombinationSummary(currentSlug);
  if (!current) return [];

  const builder = ACCESSORY_MARKET_SLUG_BUILDERS[current.role][current.slot];
  const grades: Array<AccessoryRollGrade | 'none'> = ['low', 'mid', 'high', 'none'];

  return grades
    .flatMap((primary) => grades.map((secondary) => builder.buildSlug(primary, secondary)))
    .filter((candidateSlug) => candidateSlug !== currentSlug)
    .map((candidateSlug) => getAccessoryCombinationSummary(candidateSlug))
    .filter((candidate): candidate is AccessoryCombinationSummary =>
      candidate !== undefined && candidate.stats.length > 0
    )
    .map((candidate) => ({
      slug: candidate.slug,
      role: candidate.role,
      slot: candidate.slot,
      stats: candidate.stats,
      combinedDamagePercent: candidate.combinedDamagePercent,
      damageGainPercent: candidate.combinedDamagePercent - current.combinedDamagePercent
    }))
    .filter((candidate) => candidate.damageGainPercent >= minimumGainPercent)
    .sort((a, b) => b.damageGainPercent - a.damageGainPercent);
}

export function listNextAccessoryUpgradeCandidates(
  currentSlug: string,
  minimumGainPercent = 0.01
): AccessoryUpgradeCandidate[] {
  const current = getAccessoryCombinationSummary(currentSlug);
  if (!current) return [];

  const currentGradeScore = current.stats.reduce((total, stat) => total + ACCESSORY_GRADE_SCORE[stat.grade], 0);
  const currentStatCount = current.stats.length;
  const candidates = listAccessoryUpgradeCandidates(currentSlug, minimumGainPercent)
    .map((candidate) => ({
      ...candidate,
      gradeStepGain: candidate.stats.reduce((total, stat) => total + ACCESSORY_GRADE_SCORE[stat.grade], 0) - currentGradeScore,
      statCountDelta: candidate.stats.length - currentStatCount
    }))
    .filter((candidate) => candidate.gradeStepGain > 0 || candidate.statCountDelta > 0);

  const smallestUsefulStep = Math.min(...candidates.map((candidate) => candidate.gradeStepGain + Math.max(0, candidate.statCountDelta)));
  if (!Number.isFinite(smallestUsefulStep)) return [];

  return candidates
    .filter((candidate) => candidate.gradeStepGain + Math.max(0, candidate.statCountDelta) === smallestUsefulStep)
    .sort((a, b) => b.damageGainPercent - a.damageGainPercent);
}
