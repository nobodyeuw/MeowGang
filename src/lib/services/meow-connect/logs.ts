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
  const participantIndex = buildLogParticipantIndex(localSnapshot, remoteSnapshots, localProfile);
  const temporaryPlayerIndex = buildTemporaryPlayerIndex(localSnapshot, remoteSnapshots, localProfile);
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

  return suppressCoveredEncounterLogEntries(combineEncounterGateLogEntries(
    suppressManualLogsCoveredByEncounterLogs(
      combineSharedEncounterLogEntries(
        normalizeEncounterLogParticipantLabels(
          enrichEncounterLogParticipants(
            localEntries.concat(remoteSnapshots.flatMap((snapshot) => buildSnapshotLogEntries(snapshot, allowedRaidIds))),
            participantIndex
          ),
          buildLogOwnerNameSet(localSnapshot, remoteSnapshots, localProfile),
          buildLogCharacterNameSet(localSnapshot, remoteSnapshots),
          participantIndex,
          temporaryPlayerIndex
        )
      )
    )
  )).sort((a, b) => getLogDisplayTimestamp(b) - getLogDisplayTimestamp(a));
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
      const localPlayer = resolveEncounterLocalPlayer(encounter, characterById);
      const participants = buildEncounterOwnerParticipants(encounter, snapshot.profile, ownerId, characterById, localPlayer);
      const temporaryPlayers = buildTemporaryEncounterPlayers(encounter, snapshot.profile, characterById, participants);
      return {
        ...encounter,
        localPlayer,
        clearedAt: encounter.clearedAt || matchingCompletionTime,
        ownerId,
        ownerName: snapshot.profile.displayName,
        ownerAvatarUrl: snapshot.profile.avatarUrl,
        participants,
        temporaryPlayers,
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
        participants: dedupeLogParticipants(entry.participants || [entryAsParticipant(entry)]),
        temporaryPlayers: dedupeTemporaryPlayers(entry.temporaryPlayers || []),
        bibleLogs: dedupeBibleLogs(entry.bibleLogs || [])
      });
      continue;
    }

    const participants = dedupeLogParticipants([
      ...(existing.participants || [entryAsParticipant(existing)]),
      ...(entry.participants || [entryAsParticipant(entry)])
    ]);
    const temporaryPlayers = dedupeTemporaryPlayers([
      ...(existing.temporaryPlayers || []),
      ...(entry.temporaryPlayers || [])
    ]);
    const bibleLogs = dedupeBibleLogs([
      ...(existing.bibleLogs || []),
      ...(entry.bibleLogs || [])
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
      participants,
      temporaryPlayers,
      bibleLogs
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
        participants: dedupeLogParticipants(entry.participants || [entryAsParticipant(entry)]),
        temporaryPlayers: dedupeTemporaryPlayers(entry.temporaryPlayers || []),
        bibleLogs: dedupeBibleLogs(entry.bibleLogs || [])
      });
      continue;
    }

    const participants = dedupeLogParticipants([
      ...(existing.participants || [entryAsParticipant(existing)]),
      ...(entry.participants || [entryAsParticipant(entry)])
    ]);
    const temporaryPlayers = dedupeTemporaryPlayers([
      ...(existing.temporaryPlayers || []),
      ...(entry.temporaryPlayers || [])
    ]);
    const bibleLogs = dedupeBibleLogs([
      ...(existing.bibleLogs || []),
      ...(entry.bibleLogs || [])
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
      participants,
      temporaryPlayers,
      bibleLogs
    });
  }

  return [...passthrough, ...Array.from(combined.values())];
}

function suppressManualLogsCoveredByEncounterLogs(entries: MeowConnectLogEntry[]): MeowConnectLogEntry[] {
  const encounterEntries = entries.filter((entry) => entry.source === 'LOA Logs');

  return entries.filter((entry) => {
    if (entry.source !== 'Manual') return true;
    return !encounterEntries.some((encounterEntry) => coversManualLogEntry(encounterEntry, entry));
  });
}

function suppressCoveredEncounterLogEntries(entries: MeowConnectLogEntry[]): MeowConnectLogEntry[] {
  return entries.filter((entry) => {
    if (entry.source !== 'LOA Logs') return true;
    return !entries.some((candidate) => candidate !== entry && coversEncounterLogEntry(candidate, entry));
  });
}

function coversEncounterLogEntry(candidate: MeowConnectLogEntry, covered: MeowConnectLogEntry): boolean {
  if (candidate.source !== 'LOA Logs' || covered.source !== 'LOA Logs') return false;
  if (candidate.contentId !== covered.contentId) return false;
  if (!sameDifficulty(candidate.difficulty, covered.difficulty)) return false;
  if (candidate.resetCycle && covered.resetCycle && candidate.resetCycle !== covered.resetCycle) return false;
  if (!isGateSuperset(candidate.gate, covered.gate)) return false;
  if (!isParticipantSuperset(candidate, covered)) return false;
  return logTimesOverlap(candidate, covered);
}

function coversManualLogEntry(encounterEntry: MeowConnectLogEntry, manualEntry: MeowConnectLogEntry): boolean {
  if (encounterEntry.contentId !== manualEntry.contentId) return false;
  if (!sameDifficulty(encounterEntry.difficulty, manualEntry.difficulty)) return false;
  if (encounterEntry.resetCycle && manualEntry.resetCycle && encounterEntry.resetCycle !== manualEntry.resetCycle) return false;
  if (!sameOrUnknownGate(encounterEntry.gate, manualEntry.gate)) return false;

  const encounterPlayers = new Set(
    (encounterEntry.participants || [entryAsParticipant(encounterEntry)])
      .flatMap((participant) => splitLogPlayers(participant.localPlayer))
      .map(normalizeSingleLogPlayer)
      .filter(Boolean)
  );

  return (manualEntry.participants || [entryAsParticipant(manualEntry)])
    .flatMap((participant) => splitLogPlayers(participant.localPlayer))
    .map(normalizeSingleLogPlayer)
    .some((player) => encounterPlayers.has(player));
}

function isGateSuperset(candidateGate?: string, coveredGate?: string): boolean {
  const candidateGateNumbers = getGateNumbers(candidateGate);
  const coveredGateNumbers = getGateNumbers(coveredGate);
  if (candidateGateNumbers.length === 0 || coveredGateNumbers.length === 0) return false;
  if (candidateGateNumbers.length < coveredGateNumbers.length) return false;
  return coveredGateNumbers.every((gateNumber) => candidateGateNumbers.includes(gateNumber));
}

function isParticipantSuperset(candidate: MeowConnectLogEntry, covered: MeowConnectLogEntry): boolean {
  const candidateParticipants = getNormalizedLogParticipantPlayers(candidate);
  const coveredParticipants = getNormalizedLogParticipantPlayers(covered);
  if (candidateParticipants.size === 0 || coveredParticipants.size === 0) return false;
  if (candidateParticipants.size < coveredParticipants.size) return false;
  return Array.from(coveredParticipants).every((participant) => candidateParticipants.has(participant));
}

function getNormalizedLogParticipantPlayers(entry: MeowConnectLogEntry): Set<string> {
  return new Set(
    (entry.participants || [entryAsParticipant(entry)])
      .flatMap((participant) => splitLogPlayers(participant.localPlayer))
      .map(normalizeSingleLogPlayer)
      .filter(Boolean)
  );
}

function logTimesOverlap(a: MeowConnectLogEntry, b: MeowConnectLogEntry): boolean {
  const aStart = a.fightStart || 0;
  const bStart = b.fightStart || 0;
  const aEnd = getLogEndTimestamp(a);
  const bEnd = getLogEndTimestamp(b);
  if (!aStart || !bStart || !aEnd || !bEnd) return true;
  return aStart <= bEnd && bStart <= aEnd;
}

function canCombineGateLogEntry(entry: MeowConnectLogEntry): boolean {
  return Boolean(
    getGateNumbers(entry.gate).length > 0 &&
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

function buildLogOwnerNameSet(
  localSnapshot: MeowConnectLocalSnapshot | null,
  remoteSnapshots: MeowConnectRemoteSnapshot[],
  localProfile?: MeowConnectProfile | null
): Set<string> {
  return new Set([
    localProfile?.displayName,
    'You',
    ...remoteSnapshots.map((snapshot) => snapshot.profile.displayName)
  ].map((name) => normalizeSingleLogPlayer(name || '')).filter(Boolean));
}

function buildLogCharacterNameSet(
  localSnapshot: MeowConnectLocalSnapshot | null,
  remoteSnapshots: MeowConnectRemoteSnapshot[]
): Set<string> {
  return new Set([
    ...(localSnapshot?.characters || []),
    ...remoteSnapshots.flatMap((snapshot) => snapshot.characters)
  ].map((character) => normalizeSingleLogPlayer(character.charName)).filter(Boolean));
}

function buildTemporaryPlayerIndex(
  localSnapshot: MeowConnectLocalSnapshot | null,
  remoteSnapshots: MeowConnectRemoteSnapshot[],
  localProfile?: MeowConnectProfile | null
): Map<string, { name: string; playedBy: string }> {
  const temporaryPlayers = new Map<string, { name: string; playedBy: string }>();
  const snapshots: MeowConnectRemoteSnapshot[] = [
    ...(localSnapshot
      ? [{
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
        }]
      : []),
    ...remoteSnapshots
  ];

  for (const snapshot of snapshots) {
    const rosterCharacterNames = new Set(snapshot.characters.map((character) => normalizeSingleLogPlayer(character.charName)));

    for (const encounter of snapshot.encounterSnapshots || []) {
      const localPlayer = encounter.localPlayer.trim();
      const normalizedLocalPlayer = normalizeSingleLogPlayer(localPlayer);
      if (!localPlayer || isGenericLogPlayer(localPlayer) || rosterCharacterNames.has(normalizedLocalPlayer)) continue;

      temporaryPlayers.set(normalizedLocalPlayer, {
        name: localPlayer,
        playedBy: snapshot.profile.displayName
      });
    }
  }

  return temporaryPlayers;
}

function buildEncounterOwnerParticipants(
  encounter: MeowConnectEncounterSnapshot,
  profile: MeowConnectProfile,
  ownerId: string,
  characterById: Map<number, MeowConnectCharacterSnapshot>,
  resolvedLocalPlayer: string
): MeowConnectLogParticipant[] {
  const ownerCharacters = Array.from(characterById.values());
  const primaryCharacter = ownerCharacters.find((character) => sameCharacterName(character.charName, resolvedLocalPlayer));
  const matchedCharacters = (encounter.matchedCharacterIds || [])
    .map((charId) => characterById.get(charId))
    .filter((character): character is MeowConnectCharacterSnapshot => Boolean(character));
  const namedCharacters = [resolvedLocalPlayer, encounter.localPlayer, ...(encounter.players || [])]
    .flatMap(splitLogPlayers)
    .map((name) => ownerCharacters.find((character) => sameCharacterName(name, character.charName)))
    .filter((character): character is MeowConnectCharacterSnapshot => Boolean(character));
  const characters = dedupeCharacters([...matchedCharacters, ...namedCharacters]);

  return characters.map((character) => {
    const isPrimaryPlayer = sameCharacterName(character.charName, resolvedLocalPlayer);
    const isSameRosterAsPrimary = primaryCharacter && character.rosterId === primaryCharacter.rosterId;
    const shouldUseRosterLabel = !isPrimaryPlayer && !isSameRosterAsPrimary;

    return {
      ownerId: shouldUseRosterLabel ? `${ownerId}:roster:${character.rosterId}` : ownerId,
      ownerName: shouldUseRosterLabel ? (character.rosterName || profile.displayName) : profile.displayName,
      ownerAvatarUrl: shouldUseRosterLabel ? undefined : profile.avatarUrl,
      localPlayer: character.charName
    };
  });
}

function buildTemporaryEncounterPlayers(
  encounter: MeowConnectEncounterSnapshot,
  profile: MeowConnectProfile,
  characterById: Map<number, MeowConnectCharacterSnapshot>,
  participants: MeowConnectLogParticipant[]
): Array<{ name: string; playedBy: string }> | undefined {
  const localPlayer = encounter.localPlayer.trim();
  if (!localPlayer || isGenericLogPlayer(localPlayer)) return undefined;

  const isRosterCharacter = Array.from(characterById.values())
    .some((character) => sameCharacterName(character.charName, localPlayer));
  if (isRosterCharacter) return undefined;

  const alreadyRepresented = participants.some((participant) => sameCharacterName(participant.localPlayer, localPlayer));
  if (alreadyRepresented) return undefined;

  return [{ name: localPlayer, playedBy: profile.displayName }];
}

function enrichEncounterLogParticipants(
  entries: MeowConnectLogEntry[],
  participantsByCharacter: Map<string, MeowConnectLogParticipant>
): MeowConnectLogEntry[] {
  return entries.map((entry) => {
    if (entry.source !== 'LOA Logs' || entry.players.length === 0) return entry;

    const inferredParticipants = [entry.localPlayer, ...entry.players]
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

function normalizeEncounterLogParticipantLabels(
  entries: MeowConnectLogEntry[],
  ownerNames: Set<string>,
  characterNames: Set<string>,
  participantsByCharacter: Map<string, MeowConnectLogParticipant>,
  temporaryPlayersByName: Map<string, { name: string; playedBy: string }>
): MeowConnectLogEntry[] {
  return entries.map((entry) => {
    if (entry.source !== 'LOA Logs') return entry;

    const playerCharacters = dedupeStrings(
      (entry.players || []).filter((player) => {
        const normalized = normalizeSingleLogPlayer(player);
        return characterNames.has(normalized);
      })
    );
    const temporaryPlayers = dedupeTemporaryPlayers([
      ...(entry.temporaryPlayers || []),
      ...(entry.players || [])
        .map((player) => temporaryPlayersByName.get(normalizeSingleLogPlayer(player)))
        .filter((player): player is { name: string; playedBy: string } => Boolean(player))
    ]);
    const cleanedParticipants = (entry.participants || [entryAsParticipant(entry)])
      .map((participant) => {
        const playerNames = splitLogPlayers(participant.localPlayer);
        const characterPlayerNames = playerNames.filter((name) => {
          const normalized = normalizeSingleLogPlayer(name);
          return characterNames.has(normalized);
        });

        return {
          ...participant,
          localPlayer: characterPlayerNames.length > 0 ? characterPlayerNames.join(', ') : participant.localPlayer
        };
      })
      .filter((participant) => {
        const normalizedPlayer = normalizeSingleLogPlayer(participant.localPlayer);
        return characterNames.has(normalizedPlayer) || !ownerNames.has(normalizedPlayer);
      });

    const participants = dedupeLogParticipants([
      ...cleanedParticipants,
      ...playerCharacters
        .map((player) => participantsByCharacter.get(normalizeSingleLogPlayer(player)))
        .filter((participant): participant is MeowConnectLogParticipant => Boolean(participant))
    ]);
    if (participants.length === 0) {
      return undefined;
    }

    const participantPlayers = participants.flatMap((participant) => splitLogPlayers(participant.localPlayer));
    const players = dedupeStrings([...playerCharacters, ...participantPlayers]);

    return {
      ...entry,
      players,
      participants,
      temporaryPlayers,
      bibleLogs: dedupeBibleLogs(entry.bibleLogs || []),
      ownerId: participants.map((participant) => participant.ownerId).join('+'),
      ownerName: formatParticipantNames(participants),
      localPlayer: participants.map((participant) => participant.localPlayer).join(', ')
    };
  }).filter((entry): entry is MeowConnectLogEntry => Boolean(entry));
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
    const existing = byOwner.get(key);

    if (!existing || shouldReplaceParticipantLabel(existing.localPlayer, participant.localPlayer)) {
      byOwner.set(key, participant);
    }
  }
  return Array.from(byOwner.values()).sort((a, b) => a.ownerName.localeCompare(b.ownerName));
}

function shouldReplaceParticipantLabel(currentValue: string, nextValue: string): boolean {
  return isGenericLogPlayer(currentValue) && !isGenericLogPlayer(nextValue);
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

function normalizeSingleLogPlayer(value: string): string {
  return String(value || '').trim().toLowerCase();
}

function splitLogPlayers(value: string): string[] {
  return String(value || '')
    .split(',')
    .map((part) => part.trim())
    .filter(Boolean);
}

function sameOrUnknownGate(a?: string | null, b?: string | null): boolean {
  const gateNumbersA = getGateNumbers(a);
  const gateNumbersB = getGateNumbers(b);
  return gateNumbersA.length === 0 ||
    gateNumbersB.length === 0 ||
    gateNumbersA.some((gateNumber) => gateNumbersB.includes(gateNumber));
}

function formatCombinedGateLabel(values: Array<string | undefined>): string | undefined {
  const gateNumbers = Array.from(new Set(
    values
      .flatMap((value) => getGateNumbers(value))
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

function resolveEncounterLocalPlayer(
  encounter: MeowConnectEncounterSnapshot,
  characterById: Map<number, MeowConnectCharacterSnapshot>
): string {
  if (!isGenericLogPlayer(encounter.localPlayer)) return encounter.localPlayer;

  const matchedCharacter = (encounter.matchedCharacterIds || [])
    .map((charId) => characterById.get(charId))
    .find((character): character is MeowConnectCharacterSnapshot => Boolean(character));

  if (matchedCharacter) return matchedCharacter.charName;

  const characterByName = Array.from(characterById.values())
    .find((character) => (encounter.players || []).some((player) => sameCharacterName(player, character.charName)));

  return characterByName?.charName || encounter.localPlayer;
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

function dedupeTemporaryPlayers(players: Array<{ name: string; playedBy: string }>): Array<{ name: string; playedBy: string }> {
  const byKey = new Map<string, { name: string; playedBy: string }>();
  for (const player of players) {
    const key = `${normalizeSingleLogPlayer(player.name)}:${normalizeSingleLogPlayer(player.playedBy)}`;
    if (!byKey.has(key)) byKey.set(key, player);
  }
  return Array.from(byKey.values());
}

function dedupeBibleLogs(logs: Array<{ gate?: string; upstreamId: string }>): Array<{ gate?: string; upstreamId: string }> {
  const byKey = new Map<string, { gate?: string; upstreamId: string }>();
  for (const log of logs) {
    if (!log.upstreamId) continue;
    const gateLabel = normalizeGateLabel(log.gate) || log.gate || 'raid';
    const key = gateLabel.toLowerCase();
    if (!byKey.has(key)) {
      byKey.set(key, {
        gate: gateLabel,
        upstreamId: log.upstreamId
      });
    }
  }
  return Array.from(byKey.values()).sort((a, b) => {
    const aGate = getGateNumbers(a.gate)[0] || 0;
    const bGate = getGateNumbers(b.gate)[0] || 0;
    return aGate - bGate || a.upstreamId.localeCompare(b.upstreamId);
  });
}

function dedupeCharacters(characters: MeowConnectCharacterSnapshot[]): MeowConnectCharacterSnapshot[] {
  const byId = new Map<number, MeowConnectCharacterSnapshot>();
  for (const character of characters) {
    byId.set(character.charId, character);
  }
  return Array.from(byId.values());
}

function getRaidName(contentId: string, fallback: string): string {
  return RAIDS.find((raid) => raid.id === contentId)?.name || fallback;
}

function normalizeGateLabel(value?: string | null): string | undefined {
  const gateNumbers = getGateNumbers(value);
  if (gateNumbers.length === 0) return undefined;
  if (gateNumbers.length === 1) return `Gate ${gateNumbers[0]}`;
  return `Gates ${gateNumbers.join(' + ')}`;
}

function getGateNumbers(value?: string | null): number[] {
  return Array.from(new Set(
    Array.from(String(value || '').matchAll(/(?:gate|gates|g)\s*(\d+)|\b(\d+)\b/gi))
      .map((match) => Number(match[1] || match[2] || 0))
      .filter((gateNumber) => gateNumber > 0 && gateNumber <= 8)
  )).sort((a, b) => a - b);
}

function normalizeLogSource(source?: string): 'Manual' | 'LOA Logs' | string {
  const normalized = String(source || '').trim().toLowerCase();
  if (!normalized || normalized === 'manual') return 'Manual';
  if (normalized === 'loalogs' || normalized === 'loa logs' || normalized === 'meow_connect') return 'LOA Logs';
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

function sameCharacterName(a: string, b: string): boolean {
  return a.trim().toLowerCase() === b.trim().toLowerCase();
}

function isGenericLogPlayer(value: string): boolean {
  const normalized = value.trim().toLowerCase();
  return !normalized || normalized === 'you' || normalized === 'unknown';
}
