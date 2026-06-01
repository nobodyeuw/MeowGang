import { RAIDS } from '$lib/data/raids';
import type {
  MeowConnectCharacterSnapshot,
  MeowConnectCompletionSnapshot,
  MeowConnectEncounterSnapshot,
  MeowConnectLocalSnapshot,
  MeowConnectLogEntry,
  MeowConnectLogParticipant,
  MeowConnectProfile,
  MeowConnectRemoteSnapshot
} from './types';

export function buildMeowConnectLogEntries(
  localSnapshot: MeowConnectLocalSnapshot | null,
  remoteSnapshots: MeowConnectRemoteSnapshot[],
  raidIds: string[],
  localProfile?: MeowConnectProfile | null
): MeowConnectLogEntry[] {
  const allowedRaidIds = new Set(raidIds);
  const localEntries = localSnapshot
    ? buildSnapshotLogEntries({
        profile: {
          userId: 'local',
          discordId: 'local',
          displayName: 'You',
          avatarUrl: localProfile?.avatarUrl
        },
        characters: localSnapshot.characters,
        completionSnapshots: localSnapshot.completionSnapshots,
        raidReservations: localSnapshot.raidReservations,
        encounterSnapshots: localSnapshot.encounterSnapshots,
        updatedAt: new Date(localSnapshot.generatedAt).toISOString()
      }, allowedRaidIds)
    : [];

  return combineEncounterGateLogEntries(
    combineSharedEncounterLogEntries(
      enrichEncounterLogParticipants(
        localEntries.concat(remoteSnapshots.flatMap((snapshot) => buildSnapshotLogEntries(snapshot, allowedRaidIds))),
        buildLogParticipantIndex(localSnapshot, remoteSnapshots, localProfile)
      )
    )
  ).sort((a, b) => getLogDisplayTimestamp(b) - getLogDisplayTimestamp(a));
}

function buildSnapshotLogEntries(
  snapshot: MeowConnectRemoteSnapshot,
  allowedRaidIds: Set<string>
): MeowConnectLogEntry[] {
  const ownerId = snapshot.profile.discordId || snapshot.profile.userId;
  const characterById = new Map(snapshot.characters.map((character) => [character.charId, character]));
  const encounterLogs: MeowConnectLogEntry[] = (snapshot.encounterSnapshots || [])
    .filter((encounter) => encounter.cleared && allowedRaidIds.has(encounter.contentId))
    .map((encounter) => {
      const matchingCompletionTime = findMatchingEncounterCompletionTime(encounter, snapshot.completionSnapshots, characterById);
      return {
        ...encounter,
        clearedAt: encounter.clearedAt || matchingCompletionTime,
        ownerId,
        ownerName: snapshot.profile.displayName,
        ownerAvatarUrl: snapshot.profile.avatarUrl,
        participants: [{
          ownerId,
          ownerName: snapshot.profile.displayName,
          ownerAvatarUrl: snapshot.profile.avatarUrl,
          localPlayer: encounter.localPlayer
        }],
        source: 'LOA Logs' as const
      };
    });

  const encounterKeys = new Set(
    encounterLogs.map((entry) =>
      `${entry.contentId}:${normalizeLogDifficulty(entry.difficulty)}:${entry.localPlayer.trim().toLowerCase()}`
    )
  );
  const completionGroups = new Map<string, MeowConnectLogEntry>();

  for (const completion of snapshot.completionSnapshots || []) {
    if (!completion.isCompleted || !allowedRaidIds.has(completion.contentId)) continue;

    const character = characterById.get(completion.charId);
    const characterName = character?.charName || completion.charId.toString();
    const difficulty = completion.difficulty || '';
    const source = normalizeLogSource(completion.source);
    const key = `${completion.contentId}:${normalizeLogDifficulty(difficulty)}:${characterName.trim().toLowerCase()}`;

    if (source === 'LOA Logs' && encounterKeys.has(key)) continue;

    const completedAt = completion.completedAt || 0;
    const gate = normalizeGateLabel(completion.gate || completion.sessionId);
    const groupKey = `${key}:${source}:${completion.resetCycle || ''}`;
    const existing = completionGroups.get(groupKey);

    if (!existing) {
      completionGroups.set(groupKey, {
        ownerId,
        ownerName: snapshot.profile.displayName,
        ownerAvatarUrl: snapshot.profile.avatarUrl,
        localPlayer: characterName,
        contentId: completion.contentId,
        raidName: getRaidName(completion.contentId, completion.contentId),
        difficulty,
        gate,
        cleared: true,
        fightStart: completedAt,
        duration: 0,
        clearedAt: completedAt,
        players: [],
        matchedCharacterIds: [completion.charId],
        resetCycle: completion.resetCycle,
        participants: [{
          ownerId,
          ownerName: snapshot.profile.displayName,
          ownerAvatarUrl: snapshot.profile.avatarUrl,
          localPlayer: characterName
        }],
        source
      });
      continue;
    }

    existing.gate = formatCombinedGateLabel([existing.gate, gate]);
    if (completedAt > existing.fightStart) existing.fightStart = completedAt;
    if (completedAt > (existing.clearedAt || 0)) existing.clearedAt = completedAt;
  }

  return [...encounterLogs, ...Array.from(completionGroups.values())];
}

function combineSharedEncounterLogEntries(entries: MeowConnectLogEntry[]): MeowConnectLogEntry[] {
  const combined = new Map<string, MeowConnectLogEntry>();
  const passthrough: MeowConnectLogEntry[] = [];

  for (const entry of entries) {
    if (entry.source !== 'LOA Logs' || !entry.fightStart || entry.players.length === 0) {
      passthrough.push(entry);
      continue;
    }

    const key = [
      entry.contentId,
      normalizeGate(entry.gate || 'raid'),
      normalizePlayerList(entry.players)
    ].join(':');

    const existing = combined.get(key);
    if (!existing) {
      combined.set(key, {
        ...entry,
        players: dedupeStrings(entry.players),
        participants: dedupeLogParticipants(entry.participants || [entryAsParticipant(entry)])
      });
      continue;
    }

    const participants = dedupeLogParticipants([
      ...(existing.participants || [entryAsParticipant(existing)]),
      ...(entry.participants || [entryAsParticipant(entry)])
    ]);
    const players = dedupeStrings([...existing.players, ...entry.players]);

    const fightStart = getEarliestLogStart(existing, entry);
    const clearedAt = Math.max(getLogEndTimestamp(existing), getLogEndTimestamp(entry)) || undefined;

    combined.set(key, {
      ...existing,
      difficulty: existing.difficulty || entry.difficulty,
      gate: existing.gate || entry.gate,
      fightStart,
      duration: clearedAt && fightStart ? Math.max(clearedAt - fightStart, 0) : Math.max(existing.duration || 0, entry.duration || 0),
      clearedAt,
      ownerId: participants.map((participant) => participant.ownerId).join('+'),
      ownerName: formatParticipantNames(participants),
      ownerAvatarUrl: existing.ownerAvatarUrl || entry.ownerAvatarUrl,
      localPlayer: participants.map((participant) => participant.localPlayer).join(', '),
      players,
      matchedCharacterIds: dedupeNumbers([...existing.matchedCharacterIds, ...entry.matchedCharacterIds]),
      participants
    });
  }

  return [...passthrough, ...Array.from(combined.values())];
}

function combineEncounterGateLogEntries(entries: MeowConnectLogEntry[]): MeowConnectLogEntry[] {
  const combined = new Map<string, MeowConnectLogEntry>();
  const passthrough: MeowConnectLogEntry[] = [];

  for (const entry of entries) {
    if (!canCombineGateLogEntry(entry)) {
      passthrough.push(entry);
      continue;
    }

    const key = [
      entry.source,
      entry.ownerId || entry.ownerName,
      normalizeLogPlayer(entry.localPlayer),
      entry.contentId,
      normalizeLogDifficulty(entry.difficulty),
      entry.resetCycle || ''
    ].join(':');
    const existing = combined.get(key);

    if (!existing) {
      combined.set(key, {
        ...entry,
        players: dedupeStrings(entry.players),
        participants: dedupeLogParticipants(entry.participants || [entryAsParticipant(entry)])
      });
      continue;
    }

    const participants = dedupeLogParticipants([
      ...(existing.participants || [entryAsParticipant(existing)]),
      ...(entry.participants || [entryAsParticipant(entry)])
    ]);
    const players = dedupeStrings([...existing.players, ...entry.players]);
    const gate = formatCombinedGateLabel([existing.gate, entry.gate]);

    const fightStart = getEarliestLogStart(existing, entry);
    const clearedAt = Math.max(getLogEndTimestamp(existing), getLogEndTimestamp(entry)) || undefined;

    combined.set(key, {
      ...existing,
      difficulty: existing.difficulty || entry.difficulty,
      gate,
      fightStart,
      duration: clearedAt && fightStart ? Math.max(clearedAt - fightStart, 0) : Math.max(existing.duration || 0, entry.duration || 0),
      clearedAt,
      ownerId: participants.map((participant) => participant.ownerId).join('+'),
      ownerName: formatParticipantNames(participants),
      ownerAvatarUrl: existing.ownerAvatarUrl || entry.ownerAvatarUrl,
      localPlayer: participants.map((participant) => participant.localPlayer).join(', '),
      players,
      matchedCharacterIds: dedupeNumbers([...existing.matchedCharacterIds, ...entry.matchedCharacterIds]),
      participants
    });
  }

  return [...passthrough, ...Array.from(combined.values())];
}

function canCombineGateLogEntry(entry: MeowConnectLogEntry): boolean {
  const gateNumber = normalizeGateLabel(entry.gate)?.match(/\d+/)?.[0];
  return Boolean(
    gateNumber &&
    (entry.source === 'LOA Logs' || entry.source === 'Manual') &&
    entry.contentId &&
    normalizeLogDifficulty(entry.difficulty) &&
    normalizeLogPlayer(entry.localPlayer)
  );
}

function buildLogParticipantIndex(
  localSnapshot: MeowConnectLocalSnapshot | null,
  remoteSnapshots: MeowConnectRemoteSnapshot[],
  localProfile?: MeowConnectProfile | null
): Map<string, MeowConnectLogParticipant> {
  const participantsByCharacter = new Map<string, MeowConnectLogParticipant>();

  const addSnapshot = (snapshot: MeowConnectRemoteSnapshot) => {
    for (const character of snapshot.characters || []) {
      participantsByCharacter.set(character.charName.trim().toLowerCase(), {
        ownerId: snapshot.profile.discordId || snapshot.profile.userId,
        ownerName: snapshot.profile.displayName,
        ownerAvatarUrl: snapshot.profile.avatarUrl,
        localPlayer: character.charName
      });
    }
  };

  if (localSnapshot) {
    addSnapshot({
      profile: {
        userId: 'local',
        discordId: 'local',
        displayName: 'You',
        avatarUrl: localProfile?.avatarUrl
      },
      characters: localSnapshot.characters,
      completionSnapshots: localSnapshot.completionSnapshots,
      raidReservations: localSnapshot.raidReservations,
      encounterSnapshots: localSnapshot.encounterSnapshots,
      updatedAt: new Date(localSnapshot.generatedAt).toISOString()
    });
  }

  for (const snapshot of remoteSnapshots) {
    addSnapshot(snapshot);
  }

  return participantsByCharacter;
}

function enrichEncounterLogParticipants(
  entries: MeowConnectLogEntry[],
  participantsByCharacter: Map<string, MeowConnectLogParticipant>
): MeowConnectLogEntry[] {
  return entries.map((entry) => {
    if (entry.source !== 'LOA Logs' || entry.players.length === 0) return entry;

    const inferredParticipants = entry.players
      .map((player) => participantsByCharacter.get(player.trim().toLowerCase()))
      .filter((participant): participant is MeowConnectLogParticipant => Boolean(participant));
    const participants = dedupeLogParticipants([
      ...(entry.participants || [entryAsParticipant(entry)]),
      ...inferredParticipants
    ]);

    if (participants.length <= (entry.participants || []).length) return entry;

    return {
      ...entry,
      ownerId: participants.map((participant) => participant.ownerId).join('+'),
      ownerName: formatParticipantNames(participants),
      ownerAvatarUrl: entry.ownerAvatarUrl || participants.find((participant) => participant.ownerAvatarUrl)?.ownerAvatarUrl,
      localPlayer: participants.map((participant) => participant.localPlayer).join(', '),
      participants
    };
  });
}

function entryAsParticipant(entry: MeowConnectLogEntry): MeowConnectLogParticipant {
  return {
    ownerId: entry.ownerId,
    ownerName: entry.ownerName,
    ownerAvatarUrl: entry.ownerAvatarUrl,
    localPlayer: entry.localPlayer
  };
}

function dedupeLogParticipants(participants: MeowConnectLogParticipant[]): MeowConnectLogParticipant[] {
  const byOwner = new Map<string, MeowConnectLogParticipant>();
  for (const participant of participants) {
    const key = participant.ownerId || participant.ownerName;
    if (!byOwner.has(key)) {
      byOwner.set(key, participant);
    }
  }
  return Array.from(byOwner.values()).sort((a, b) => a.ownerName.localeCompare(b.ownerName));
}

function formatParticipantNames(participants: MeowConnectLogParticipant[]): string {
  if (participants.length <= 2) {
    return participants.map((participant) => participant.ownerName).join(' and ');
  }
  return `${participants.slice(0, -1).map((participant) => participant.ownerName).join(', ')} and ${participants[participants.length - 1].ownerName}`;
}

function normalizePlayerList(players: string[]): string {
  return dedupeStrings(players)
    .map(normalizeLogPlayer)
    .sort()
    .join('|');
}

function normalizeLogPlayer(value: string): string {
  return (value || '')
    .split(',')
    .map((part) => part.trim().toLowerCase())
    .filter(Boolean)
    .sort()
    .join('|');
}

function formatCombinedGateLabel(values: Array<string | undefined>): string | undefined {
  const gateNumbers = Array.from(new Set(
    values
      .map((value) => normalizeGateLabel(value))
      .map((value) => value?.match(/\d+/)?.[0])
      .filter((value): value is string => Boolean(value))
      .map(Number)
  )).sort((a, b) => a - b);

  if (gateNumbers.length === 0) return values.find(Boolean);
  if (gateNumbers.length === 1) return `Gate ${gateNumbers[0]}`;
  return `Gates ${gateNumbers.join(' + ')}`;
}

function findMatchingEncounterCompletionTime(
  encounter: MeowConnectEncounterSnapshot,
  completions: MeowConnectCompletionSnapshot[],
  characterById: Map<number, MeowConnectCharacterSnapshot>
): number | undefined {
  const encounterGate = normalizeGateLabel(encounter.gate);
  const encounterPlayerNames = new Set(
    [encounter.localPlayer, ...encounter.players]
      .map((player) => player.trim().toLowerCase())
      .filter(Boolean)
  );

  const candidates = completions.filter((completion) => {
    if (!completion.isCompleted || completion.contentId !== encounter.contentId || !completion.completedAt) return false;
    if (encounter.difficulty && completion.difficulty && !sameDifficulty(encounter.difficulty, completion.difficulty)) return false;

    const completionGate = normalizeGateLabel(completion.gate || completion.sessionId);
    if (encounterGate && completionGate && encounterGate !== completionGate) return false;

    const characterName = characterById.get(completion.charId)?.charName.trim().toLowerCase();
    return characterName ? encounterPlayerNames.has(characterName) : (encounter.matchedCharacterIds || []).includes(completion.charId);
  });

  return Math.max(...candidates.map((completion) => completion.completedAt || 0), 0) || undefined;
}

function getLogDisplayTimestamp(entry: MeowConnectLogEntry): number {
  return getLogEndTimestamp(entry);
}

function getEncounterClearedAt(fightStart: number, duration: number): number | undefined {
  if (!fightStart) return undefined;
  return fightStart + Math.max(duration || 0, 0);
}

function getLogEndTimestamp(entry: Pick<MeowConnectEncounterSnapshot, 'fightStart' | 'duration' | 'clearedAt'>): number {
  return entry.clearedAt || getEncounterClearedAt(entry.fightStart || 0, entry.duration || 0) || entry.fightStart || 0;
}

function getEarliestLogStart(...entries: Pick<MeowConnectEncounterSnapshot, 'fightStart'>[]): number {
  const starts = entries.map((entry) => entry.fightStart || 0).filter(Boolean);
  return starts.length ? Math.min(...starts) : 0;
}

function dedupeStrings(values: string[]): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const value of values) {
    const trimmed = value.trim();
    if (!trimmed) continue;
    const key = trimmed.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    result.push(trimmed);
  }
  return result;
}

function dedupeNumbers(values: number[]): number[] {
  return Array.from(new Set(values.map((value) => Number(value || 0)).filter(Boolean)));
}

function getRaidName(contentId: string, fallback: string): string {
  return RAIDS.find((raid) => raid.id === contentId)?.name || fallback;
}

function normalizeGateLabel(value?: string | null): string | undefined {
  const match = String(value || '').match(/gate\s*(\d+)|g\s*(\d+)/i);
  const gateNumber = match?.[1] ?? match?.[2];
  return gateNumber ? `Gate ${gateNumber}` : undefined;
}

function normalizeLogSource(source?: string): 'Manual' | 'LOA Logs' | string {
  const normalized = String(source || '').trim().toLowerCase();
  if (!normalized || normalized === 'manual') return 'Manual';
  if (normalized === 'loalogs' || normalized === 'loa logs') return 'LOA Logs';
  return source || 'Manual';
}

function normalizeLogDifficulty(value?: string): string {
  return String(value || '').trim().toLowerCase();
}

function normalizeGate(value: string): string {
  return value.trim().toLowerCase().replace(/\s+/g, ' ');
}

function sameDifficulty(a: string, b: string): boolean {
  return a.trim().toLowerCase() === b.trim().toLowerCase();
}
