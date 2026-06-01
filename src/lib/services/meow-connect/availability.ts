import { RAIDS, type Raid } from '$lib/data/raids';
import { getMeowConnectFavoriteKey } from './preferences';
import type {
  MeowConnectAvailabilityRow,
  MeowConnectCharacterSnapshot,
  MeowConnectCompletionSnapshot,
  MeowConnectEncounterSnapshot,
  MeowConnectLocalSnapshot,
  MeowConnectProfile,
  MeowConnectRemoteSnapshot
} from './types';

type EncounterEvidenceByCharacterName = Map<string, MeowConnectEncounterSnapshot[]>;

export function getMeowConnectRaidOptions() {
  const seen = new Set<string>();
  return RAIDS.filter((raid) => {
    const key = raid.id;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

export function getMeowConnectRaidDifficulties(raidId: string): string[] {
  return RAIDS
    .filter((raid) => raid.id === raidId)
    .sort((a, b) => a.gates[0].minIlvl - b.gates[0].minIlvl)
    .map((raid) => raid.difficulty);
}

export function buildMeowConnectAvailabilityRows(
  localSnapshot: MeowConnectLocalSnapshot | null,
  remoteSnapshots: MeowConnectRemoteSnapshot[],
  raidId: string,
  difficulty: string,
  favoriteIds: Set<string>,
  localProfile?: MeowConnectProfile | null
): MeowConnectAvailabilityRow[] {
  const raids = RAIDS
    .filter((entry) => entry.id === raidId)
    .sort((a, b) => a.gates[0].minIlvl - b.gates[0].minIlvl);
  if (raids.length === 0) return [];

  const localRemoteSnapshot = localSnapshot
    ? {
        profile: {
          userId: localProfile?.userId || 'local',
          discordId: 'local',
          displayName: 'You',
          avatarUrl: localProfile?.avatarUrl
        },
        characters: localSnapshot.characters,
        completionSnapshots: localSnapshot.completionSnapshots,
        raidReservations: localSnapshot.raidReservations,
        encounterSnapshots: localSnapshot.encounterSnapshots,
        updatedAt: new Date(localSnapshot.generatedAt).toISOString()
      }
    : null;
  const allSnapshots = localRemoteSnapshot ? [localRemoteSnapshot, ...remoteSnapshots] : remoteSnapshots;
  const encounterEvidenceByCharacterName = buildEncounterEvidenceByCharacterName(allSnapshots, raidId);

  const localRows = localRemoteSnapshot
    ? buildSnapshotRows(localRemoteSnapshot, raids, difficulty, favoriteIds, encounterEvidenceByCharacterName)
    : [];

  return localRows
    .concat(remoteSnapshots.flatMap((snapshot) => buildSnapshotRows(snapshot, raids, difficulty, favoriteIds, encounterEvidenceByCharacterName)))
    .sort((a, b) =>
      Number(b.favorite) - Number(a.favorite) ||
      statusRank(a.status) - statusRank(b.status) ||
      b.character.itemLevel - a.character.itemLevel ||
      a.ownerName.localeCompare(b.ownerName) ||
      a.character.displayOrder - b.character.displayOrder ||
      a.character.charName.localeCompare(b.character.charName)
    );
}

function buildSnapshotRows(
  snapshot: MeowConnectRemoteSnapshot,
  raids: Raid[],
  selectedDifficulty: string,
  favoriteIds: Set<string>,
  encounterEvidenceByCharacterName: EncounterEvidenceByCharacterName
): MeowConnectAvailabilityRow[] {
  const raidId = raids[0].id;
  const completionByCharacter = new Map<number, MeowConnectCompletionSnapshot[]>();
  for (const completion of snapshot.completionSnapshots) {
    if (completion.contentId !== raidId) continue;
    const entries = completionByCharacter.get(completion.charId) || [];
    entries.push(completion);
    completionByCharacter.set(completion.charId, entries);
  }
  const encountersByCharacter = new Map<number, MeowConnectEncounterSnapshot[]>();
  for (const encounter of snapshot.encounterSnapshots || []) {
    if (!encounter.cleared || encounter.contentId !== raidId) continue;
    for (const charId of encounter.matchedCharacterIds || []) {
      const entries = encountersByCharacter.get(charId) || [];
      entries.push(encounter);
      encountersByCharacter.set(charId, entries);
    }
  }

  const reservedCharacterIds = new Set(
    (snapshot.raidReservations || [])
      .filter((reservation) =>
        reservation.contentId === raidId &&
        reservation.reservedForStatic &&
        (selectedDifficulty === 'all' || sameDifficulty(reservation.difficulty, selectedDifficulty))
      )
      .map((reservation) => reservation.charId)
  );

  return snapshot.characters
    .filter((character) => !character.hideFromDashboard)
    .map((character) => ({
      character,
      raid: selectRaidForCharacter(raids, selectedDifficulty, character.itemLevel)
    }))
    .filter((entry): entry is { character: MeowConnectCharacterSnapshot; raid: Raid } => Boolean(entry.raid))
    .map((character) => {
      const completions = completionByCharacter.get(character.character.charId) || [];
      const encounters = dedupeEncounterSnapshots(
        [
          ...(encountersByCharacter.get(character.character.charId) || []),
          ...(encounterEvidenceByCharacterName.get(normalizeCharacterName(character.character.charName)) || [])
        ]
      );
      const totalGates = character.raid.gates.length;
      const detailedReservation = reservedCharacterIds.has(character.character.charId);
      const clearedGates = countClearedGates(completions, totalGates, encounters);
      const status = clearedGates >= totalGates ? 'cleared' : 'open';
      const clearedDifficulty = status === 'cleared'
        ? getClearedEvidenceDifficulty(completions, encounters, selectedDifficulty)
        : undefined;

      return {
        ownerId: snapshot.profile.discordId || snapshot.profile.userId,
        ownerUserId: snapshot.profile.userId,
        ownerName: snapshot.profile.displayName,
        ownerAvatarUrl: snapshot.profile.avatarUrl,
        favoriteKey: getMeowConnectFavoriteKey(snapshot.profile.discordId || snapshot.profile.userId, character.character.charId),
        favorite: favoriteIds.has(getMeowConnectFavoriteKey(snapshot.profile.discordId || snapshot.profile.userId, character.character.charId)),
        character: character.character,
        raid: character.raid,
        clearedGates,
        totalGates,
        openGates: Math.max(0, totalGates - clearedGates),
        status,
        clearedDifficulty,
        reservedForStatic: detailedReservation,
        staticReservationDetailsVisible: detailedReservation,
        sources: Array.from(
          new Set(
            completions
              .map((entry) => [entry.source, entry.difficulty].filter(Boolean).join(' '))
              .filter(Boolean)
          )
        ).sort()
      };
    });
}

function buildEncounterEvidenceByCharacterName(
  snapshots: MeowConnectRemoteSnapshot[],
  raidId: string
): EncounterEvidenceByCharacterName {
  const evidenceByCharacterName: EncounterEvidenceByCharacterName = new Map();

  for (const snapshot of snapshots) {
    for (const encounter of snapshot.encounterSnapshots || []) {
      if (!encounter.cleared || encounter.contentId !== raidId) continue;
      for (const characterName of getEncounterPlayerNames(encounter)) {
        const key = normalizeCharacterName(characterName);
        if (!key) continue;
        const entries = evidenceByCharacterName.get(key) || [];
        entries.push(encounter);
        evidenceByCharacterName.set(key, entries);
      }
    }
  }

  return evidenceByCharacterName;
}

function getEncounterPlayerNames(encounter: MeowConnectEncounterSnapshot): string[] {
  const seen = new Set<string>();
  const names: string[] = [];
  for (const value of [encounter.localPlayer, ...(encounter.players || [])]) {
    const key = normalizeCharacterName(value);
    if (!key || seen.has(key)) continue;
    seen.add(key);
    names.push(value);
  }
  return names;
}

function normalizeCharacterName(value: string): string {
  return value.trim().toLowerCase();
}

function selectRaidForCharacter(raids: Raid[], selectedDifficulty: string, itemLevel: number): Raid | null {
  if (selectedDifficulty !== 'all') {
    const raid = raids.find((entry) => sameDifficulty(entry.difficulty, selectedDifficulty));
    return raid && itemLevel >= raid.gates[0].minIlvl ? raid : null;
  }

  return raids
    .filter((raid) => itemLevel >= raid.gates[0].minIlvl)
    .sort((a, b) => b.gates[0].minIlvl - a.gates[0].minIlvl)[0] || null;
}

function countClearedGates(
  completions: MeowConnectCompletionSnapshot[],
  totalGates: number,
  encounters: MeowConnectEncounterSnapshot[] = []
): number {
  const gates = new Set<string>();
  const numericGates: number[] = [];
  for (const completion of completions) {
    if (!completion.isCompleted) continue;
    const gate = normalizeGate(completion.gate || completion.sessionId || 'raid');
    if (gate === 'raid' || gate === 'clear' || gate === 'completed') {
      return totalGates;
    }
    const gateNumber = getGateNumber(gate);
    if (gateNumber) numericGates.push(gateNumber);
    gates.add(gate);
  }
  for (const encounter of encounters) {
    if (!encounter.cleared) continue;
    const gate = normalizeGate(encounter.gate || 'raid');
    if (gate === 'raid' || gate === 'clear' || gate === 'completed') {
      return totalGates;
    }
    const gateNumber = getGateNumber(gate);
    if (gateNumber) numericGates.push(gateNumber);
    gates.add(gate);
  }
  if (numericGates.length > 0) {
    return Math.min(totalGates, Math.max(...numericGates));
  }
  return gates.size;
}

function getGateNumber(value: string): number | null {
  const match = value.match(/(?:gate|g)\s*(\d+)/i) || value.match(/\b(\d+)\b/);
  return match ? Number(match[1]) : null;
}

function getClearedEvidenceDifficulty(
  completions: MeowConnectCompletionSnapshot[],
  encounters: MeowConnectEncounterSnapshot[],
  selectedDifficulty: string
): string | undefined {
  const difficulties = [...completions, ...encounters]
    .filter((entry) => 'cleared' in entry ? entry.cleared : entry.isCompleted)
    .map((entry) => entry.difficulty || '')
    .filter(Boolean);

  return difficulties.find((difficulty) => sameDifficulty(difficulty, selectedDifficulty)) || difficulties[0];
}

function dedupeEncounterSnapshots(encounters: MeowConnectEncounterSnapshot[]): MeowConnectEncounterSnapshot[] {
  const seen = new Set<string>();
  const result: MeowConnectEncounterSnapshot[] = [];
  for (const encounter of encounters) {
    const key = [
      encounter.contentId,
      normalizeGate(encounter.gate || ''),
      normalizeLogDifficulty(encounter.difficulty),
      encounter.localPlayer.trim().toLowerCase(),
      encounter.fightStart
    ].join(':');
    if (seen.has(key)) continue;
    seen.add(key);
    result.push(encounter);
  }
  return result;
}

function normalizeGate(value: string): string {
  return value.trim().toLowerCase().replace(/\s+/g, ' ');
}

function normalizeLogDifficulty(value?: string): string {
  return String(value || '').trim().toLowerCase();
}

function sameDifficulty(a: string, b: string): boolean {
  return a.trim().toLowerCase() === b.trim().toLowerCase();
}

function statusRank(status: MeowConnectAvailabilityRow['status']): number {
  if (status === 'open') return 0;
  if (status === 'too_low') return 1;
  return 2;
}
