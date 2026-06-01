// Source of truth for game class display names, icon ids, and class-name aliases.
export interface GameClass {
  id: string;
  displayName: string;
  iconId: string;
}

export const GAME_CLASSES: Record<string, GameClass> = {
  "berserker": { id: "berserker", displayName: "Berserker", iconId: "102" },
  "destroyer": { id: "destroyer", displayName: "Destroyer", iconId: "103" },
  "warlord": { id: "warlord", displayName: "Gunlancer", iconId: "104" },
  "holyknight": { id: "holyknight", displayName: "Paladin", iconId: "105" },
  "berserker_female": { id: "berserker_female", displayName: "Slayer", iconId: "112" },
  "summoner": { id: "summoner", displayName: "Summoner", iconId: "203" },
  "arcana": { id: "arcana", displayName: "Arcanist", iconId: "202" },
  "bard": { id: "bard", displayName: "Bard", iconId: "204" },
  "elemental_master": { id: "elemental_master", displayName: "Sorceress", iconId: "205" },
  "battle_master": { id: "battle_master", displayName: "Wardancer", iconId: "302" },
  "force_master": { id: "force_master", displayName: "Soulfist", iconId: "304" },
  "lance_master": { id: "lance_master", displayName: "Glaivier", iconId: "305" },
  "striker": { id: "striker", displayName: "Striker", iconId: "312" },
  "infighter_male": { id: "infighter_male", displayName: "Breaker", iconId: "313" },
  "blade": { id: "blade", displayName: "Deathblade", iconId: "402" },
  "demonic": { id: "demonic", displayName: "Shadowhunter", iconId: "403" },
  "reaper": { id: "reaper", displayName: "Reaper", iconId: "404" },
  "soul_eater": { id: "soul_eater", displayName: "Souleater", iconId: "405" },
  "sharpshooter": { id: "sharpshooter", displayName: "Sharpshooter", iconId: "502" },
  "devil_hunter": { id: "devil_hunter", displayName: "Deadeye", iconId: "503" },
  "blaster": { id: "blaster", displayName: "Artillerist", iconId: "504" },
  "scouter": { id: "scouter", displayName: "Machinist", iconId: "505" },
  "devil_hunter_female": { id: "devil_hunter_female", displayName: "Gunslinger", iconId: "512" },
  "yinyangshi": { id: "yinyangshi", displayName: "Artist", iconId: "602" },
  "weather_artist": { id: "weather_artist", displayName: "Aeromancer", iconId: "603" },
  "alchemist": { id: "alchemist", displayName: "Wildsoul", iconId: "604" },
  "dragon_knight": { id: "dragon_knight", displayName: "Guardianknight", iconId: "702" },
  "holyknight_female": { id: "holyknight_female", displayName: "Valkyrie", iconId: "113" }
};

const CLASS_ALIASES: Record<string, string> = {
  gunlancer: 'warlord',
  paladin: 'holyknight',
  slayer: 'berserker_female',
  arcanist: 'arcana',
  sorceress: 'elemental_master',
  wardancer: 'battle_master',
  soulfist: 'force_master',
  glaivier: 'lance_master',
  breaker: 'infighter_male',
  deathblade: 'blade',
  shadowhunter: 'demonic',
  souleater: 'soul_eater',
  deadeye: 'devil_hunter',
  artillerist: 'blaster',
  machinist: 'scouter',
  scouter: 'scouter',
  artist: 'yinyangshi',
  aeromancer: 'weather_artist',
  wildsoul: 'alchemist',
  guardianknight: 'dragon_knight',
  valkyrie: 'holyknight_female'
};

export function normalizeClassId(classId: string): string {
  return classId.trim().toLowerCase().replace(/\s+/g, '_');
}

export function getGameClassInfo(classId: string): GameClass | undefined {
  const normalized = normalizeClassId(classId);

  return GAME_CLASSES[normalized] ||
    GAME_CLASSES[CLASS_ALIASES[normalized]] ||
    Object.values(GAME_CLASSES).find((entry) => entry.displayName.toLowerCase() === classId.trim().toLowerCase());
}

export function getGameClassDisplayName(classId: string): string {
  return getGameClassInfo(classId)?.displayName || classId;
}

export function getGameClassIconId(classId: string): string {
  return getGameClassInfo(classId)?.iconId || '0';
}
