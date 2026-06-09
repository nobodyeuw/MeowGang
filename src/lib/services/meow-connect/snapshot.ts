import { invoke } from '@tauri-apps/api/core';
import { RAIDS } from '$lib/data/raids';
import { encounterMap } from '$lib/data/encounters';
import { getCurrentSupabaseDiscordProfile, resolveDiscordWhitelistDisplayName, supabase } from '$lib/services/supabase-auth';
import type {
  MeowConnectCharacterConflict,
  MeowConnectCharacterSnapshot,
  MeowConnectEncounterSnapshot,
  MeowConnectLocalSnapshot,
  MeowConnectRemoteSnapshot
} from './types';

const BLOCKED_DUPLICATE_CHARACTERS_STORAGE_KEY = 'meowConnect.blockedDuplicateCharacters';
const DUPLICATE_CHARACTER_RECHECK_MS = 24 * 60 * 60 * 1000;

interface EncounterPreview {
  current_boss: string;
  local_player: string;
  difficulty: string;
  fight_start: number;
  duration?: number;
  cleared: boolean;
  players: string[];
  bible_logs?: Array<{ upstream_id: string }>;
}

export interface MeowConnectClearHintInput {
  charId: number;
  contentId: string;
  gate: string;
  difficulty?: string;
  completedAt: number;
  sourceOwnerName?: string;
}

export async function loadMeowConnectLocalSnapshot(): Promise<MeowConnectLocalSnapshot> {
  const snapshot = await invoke<MeowConnectLocalSnapshot>('get_meow_connect_local_snapshot');
  const encounterSnapshots = await loadLocalEncounterSnapshots(snapshot);
  return { ...snapshot, encounterSnapshots };
}

export async function applyMeowConnectClearHints(hints: MeowConnectClearHintInput[]): Promise<number> {
  if (hints.length === 0) return 0;
  return invoke<number>('apply_meow_connect_clear_hints', { hints });
}

/// Applies friend LOA Logs evidence to local MeowConnect-enabled characters.
export async function applyFriendClearHintsToLocalSnapshot(
  snapshot: MeowConnectLocalSnapshot,
  snapshots: MeowConnectRemoteSnapshot[]
): Promise<number> {
  const hints = buildFriendClearHints(snapshot, snapshots);
  if (hints.length === 0) return 0;
  return applyMeowConnectClearHints(hints);
}

/// Builds one latest clear hint per local character/content/gate from remote encounter evidence.
export function buildFriendClearHints(
  snapshot: MeowConnectLocalSnapshot,
  snapshots: MeowConnectRemoteSnapshot[]
): MeowConnectClearHintInput[] {
  const charactersByName = new Map(
    snapshot.characters.map((character) => [normalizeCharacterName(character.charName), character])
  );
  const latestByGate = new Map<string, MeowConnectClearHintInput>();

  const evidenceSnapshots = [
    {
      ownerName: 'Local LOA Logs',
      encounterSnapshots: snapshot.encounterSnapshots || []
    },
    ...snapshots.map((remoteSnapshot) => ({
      ownerName: remoteSnapshot.profile.displayName,
      encounterSnapshots: remoteSnapshot.encounterSnapshots || []
    }))
  ];

  for (const evidenceSnapshot of evidenceSnapshots) {
    for (const encounter of evidenceSnapshot.encounterSnapshots) {
      if (!encounter.cleared || !encounter.contentId || !encounter.gate) continue;
      const completedAt = Number(encounter.clearedAt || encounter.fightStart || 0);
      if (completedAt > 0 && completedAt < snapshot.weeklyResetMs) continue;

      for (const playerName of [encounter.localPlayer, ...(encounter.players || [])]) {
        const character = charactersByName.get(normalizeCharacterName(playerName));
        if (!character) continue;

        const key = `${character.charId}:${encounter.contentId}:${normalizeCharacterName(encounter.gate)}`;
        const current = latestByGate.get(key);
        if (current && current.completedAt >= completedAt) continue;

        latestByGate.set(key, {
          charId: character.charId,
          contentId: encounter.contentId,
          gate: encounter.gate,
          difficulty: encounter.difficulty,
          completedAt,
          sourceOwnerName: evidenceSnapshot.ownerName
        });
      }
    }
  }

  return Array.from(latestByGate.values());
}

function normalizeCharacterName(value?: string | null): string {
  return String(value || '').trim().toLowerCase();
}

export async function syncMeowConnectSnapshot(snapshot: MeowConnectLocalSnapshot): Promise<{
  syncedSnapshot: MeowConnectLocalSnapshot;
  duplicateCharacters: MeowConnectCharacterConflict[];
}> {
  const profile = await getCurrentSupabaseDiscordProfile();
  const snapshotCharacterIds = new Set(snapshot.characters.map((character) => character.charId));
  const blockedConflicts = getStoredBlockedDuplicateCharacters();
  const storedBlockedConflicts = blockedConflicts.filter((conflict) => snapshotCharacterIds.has(conflict.charId));
  const freshBlockedConflicts = storedBlockedConflicts.filter((conflict) => !shouldRecheckBlockedDuplicate(conflict));
  const recheckBlockedConflicts = storedBlockedConflicts.filter(shouldRecheckBlockedDuplicate);
  const freshBlockedCharacterIds = new Set(freshBlockedConflicts.map((conflict) => conflict.charId));
  const duplicateCharacters = await findMeowConnectCharacterConflicts(
    snapshot.characters.filter((character) => !freshBlockedCharacterIds.has(character.charId))
  );
  const duplicateCharacterIds = new Set(duplicateCharacters.map((conflict) => conflict.charId));
  const releasedBlockedConflicts = recheckBlockedConflicts.filter((conflict) => !duplicateCharacterIds.has(conflict.charId));
  forgetBlockedDuplicateCharacters(releasedBlockedConflicts);
  logBlockedDuplicateCharacters('info', freshBlockedConflicts, 'Skipped locally blocked duplicate character upload');
  logBlockedDuplicateCharacters('warn', duplicateCharacters, 'Blocked duplicate character upload');
  logReleasedDuplicateCharacters(releasedBlockedConflicts);
  rememberBlockedDuplicateCharacters(duplicateCharacters);
  const blockedCharacterIds = new Set(freshBlockedConflicts.map((conflict) => conflict.charId));
  for (const duplicate of duplicateCharacters) {
    blockedCharacterIds.add(duplicate.charId);
  }
  const blockedDuplicateCharacters = dedupeCharacterConflicts([...freshBlockedConflicts, ...duplicateCharacters]);

  const syncedSnapshot = filterSnapshotForBlockedCharacters(snapshot, blockedCharacterIds);
  const resetCycle = String(snapshot.weeklyResetMs || 0);

  await throwIfSupabaseError(
    supabase.from('meow_profiles').upsert({
      user_id: profile.userId,
      discord_id: profile.discordId,
      display_name: profile.displayName,
      avatar_url: profile.avatarUrl || null,
      consent_accepted: true
    })
  );

  await throwIfSupabaseError(supabase.from('meow_completion_snapshots').delete().eq('user_id', profile.userId));
  await throwIfSupabaseError(supabase.from('meow_encounter_snapshots').delete().eq('user_id', profile.userId));
  await throwIfSupabaseError(supabase.from('meow_raid_reservations').delete().eq('user_id', profile.userId));

  if (syncedSnapshot.characters.length > 0) {
    const reservedCharacterIds = new Set(
      (syncedSnapshot.raidReservations || [])
        .filter((reservation) => reservation.reservedForStatic)
        .map((reservation) => reservation.charId)
    );

    await throwIfSupabaseError(
      supabase.from('meow_characters').upsert(
        syncedSnapshot.characters.map((character) => ({
          user_id: profile.userId,
          char_id: character.charId,
          roster_id: character.rosterId,
          roster_name: character.rosterName,
          char_name: character.charName,
          class_id: character.classId,
          item_level: character.itemLevel,
          combat_power: character.combatPower,
          display_order: character.displayOrder,
          earns_gold: character.earnsGold,
          hide_from_dashboard: character.hideFromDashboard,
          meow_connect_enabled: character.meowConnectEnabled,
          has_static_reservation: reservedCharacterIds.has(character.charId)
        }))
      )
    );

    const syncedCharacterIds = syncedSnapshot.characters.map((character) => character.charId);
    await throwIfSupabaseError(
      supabase
        .from('meow_characters')
        .delete()
        .eq('user_id', profile.userId)
        .not('char_id', 'in', `(${syncedCharacterIds.join(',')})`)
    );
  } else {
    await throwIfSupabaseError(supabase.from('meow_characters').delete().eq('user_id', profile.userId));
  }

  if (syncedSnapshot.completionSnapshots.length > 0) {
    await throwIfSupabaseError(
      supabase.from('meow_completion_snapshots').upsert(
        syncedSnapshot.completionSnapshots.map((completion) => ({
          user_id: profile.userId,
          roster_id: completion.rosterId,
          char_id: completion.charId,
          content_id: completion.contentId,
          difficulty: completion.difficulty || '',
          gate: completion.gate || completion.sessionId || 'raid',
          is_completed: completion.isCompleted,
          source: completion.source || 'manual',
          session_id: completion.sessionId || null,
          reset_cycle: completion.resetCycle || resetCycle,
          completed_at: completion.completedAt ?? null
        }))
      )
    );
  }

  const reservations = syncedSnapshot.raidReservations.filter((reservation) => reservation.reservedForStatic);
  if (reservations.length > 0) {
    await throwIfSupabaseError(
      supabase.from('meow_raid_reservations').upsert(
        reservations.map((reservation) => ({
          user_id: profile.userId,
          roster_id: reservation.rosterId,
          char_id: reservation.charId,
          content_id: reservation.contentId,
          difficulty: reservation.difficulty || '',
          reserved_for_static: reservation.reservedForStatic
        }))
      )
    );
  }

  if ((syncedSnapshot.encounterSnapshots || []).length > 0) {
    const encounterRows = (syncedSnapshot.encounterSnapshots || []).map((encounter) => ({
      user_id: profile.userId,
      local_player: encounter.localPlayer,
      content_id: encounter.contentId,
      raid_name: encounter.raidName,
      difficulty: encounter.difficulty || '',
      gate: encounter.gate || 'raid',
      cleared: encounter.cleared,
      fight_start: encounter.fightStart,
      duration: encounter.duration || 0,
      players_json: encounter.players || [],
      matched_character_ids_json: encounter.matchedCharacterIds || [],
      bible_logs_json: encounter.bibleLogs || [],
      reset_cycle: encounter.resetCycle || resetCycle
    }));

    try {
      await throwIfSupabaseError(supabase.from('meow_encounter_snapshots').upsert(encounterRows));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (!message.includes('bible_logs_json')) throw error;

      writeMeowConnectLog('warn', 'Supabase meow_encounter_snapshots.bible_logs_json is missing; synced encounters without Lost Ark Bible preview links.');
      await throwIfSupabaseError(
        supabase.from('meow_encounter_snapshots').upsert(
          encounterRows.map(({ bible_logs_json, ...row }) => row)
        )
      );
    }
  }

  return { syncedSnapshot, duplicateCharacters: blockedDuplicateCharacters };
}

export async function fetchMeowConnectRemoteSnapshots(resetCycle?: string): Promise<MeowConnectRemoteSnapshot[]> {
  const profile = await getCurrentSupabaseDiscordProfile();
  const [{ data: profiles }, { data: characters }, { data: completions }, { data: reservations }, { data: encounters }] = await Promise.all([
    throwIfSupabaseError(supabase.from('meow_profiles').select('*').neq('user_id', profile.userId)),
    throwIfSupabaseError(supabase.from('meow_characters').select('*').neq('user_id', profile.userId)),
    throwIfSupabaseError(
      supabase
        .from('meow_completion_snapshots')
        .select('*')
        .neq('user_id', profile.userId)
        .eq('reset_cycle', resetCycle || '')
    ),
    throwIfSupabaseError(supabase.from('meow_raid_reservations').select('*').neq('user_id', profile.userId)),
    throwIfSupabaseError(
      supabase
        .from('meow_encounter_snapshots')
        .select('*')
        .neq('user_id', profile.userId)
        .eq('reset_cycle', resetCycle || '')
    )
  ]);

  return applyWhitelistDisplayNames(buildRemoteSnapshots(
    (profiles || []) as MeowProfileRow[],
    (characters || []) as MeowCharacterRow[],
    (completions || []) as MeowCompletionRow[],
    (reservations || []) as MeowReservationRow[],
    (encounters || []) as MeowEncounterRow[]
  ));
}

async function loadLocalEncounterSnapshots(snapshot: MeowConnectLocalSnapshot): Promise<MeowConnectEncounterSnapshot[]> {
  try {
    const encounters = await invoke<EncounterPreview[]>('get_encounters_preview', { limit: 120 });
    const characterIdsByName = new Map(
      snapshot.characters.map((character) => [character.charName.trim().toLowerCase(), character.charId])
    );

    return encounters.flatMap((encounter) => {
      const match = getRaidMatchForEncounter(encounter.current_boss);
      if (!match) return [];
      if (encounter.fight_start > 0 && encounter.fight_start < snapshot.weeklyResetMs) return [];

      const entry: MeowConnectEncounterSnapshot = {
        localPlayer: encounter.local_player,
        contentId: match.contentId,
        raidName: getRaidName(match.contentId, encounter.current_boss),
        difficulty: encounter.difficulty || '',
        gate: match.gate,
        cleared: Boolean(encounter.cleared),
        fightStart: encounter.fight_start,
        duration: encounter.duration || 0,
        clearedAt: getEncounterClearedAt(encounter.fight_start, encounter.duration || 0),
        players: encounter.players || [],
        matchedCharacterIds: [encounter.local_player, ...(encounter.players || [])]
          .map((player) => characterIdsByName.get(player.trim().toLowerCase()) || 0)
          .filter(Boolean),
        bibleLogs: (encounter.bible_logs || [])
          .map((log) => ({
            gate: match.gate,
            upstreamId: log.upstream_id
          }))
          .filter((log) => Boolean(log.upstreamId)),
        resetCycle: String(snapshot.weeklyResetMs || 0)
      };
      return [entry];
    });
  } catch {
    return [];
  }
}

async function findMeowConnectCharacterConflicts(characters: MeowConnectCharacterSnapshot[]): Promise<MeowConnectCharacterConflict[]> {
  const charIds = Array.from(new Set(characters.map((character) => character.charId).filter(Boolean)));
  if (charIds.length === 0) return [];

  const { data } = await throwIfSupabaseError(
    supabase.rpc('meow_find_character_conflicts', {
      character_ids: charIds
    })
  );
  const conflicts = ((data || []) as MeowCharacterConflictRow[]).map((row) => ({
    charId: Number(row.char_id),
    charName: row.char_name || characters.find((character) => character.charId === Number(row.char_id))?.charName || String(row.char_id),
    ownerDisplayName: row.owner_display_name || 'another MeowConnect user'
  }));

  return conflicts.filter((conflict) => Number.isFinite(conflict.charId));
}

function filterSnapshotForBlockedCharacters(
  snapshot: MeowConnectLocalSnapshot,
  blockedCharacterIds: Set<number>
): MeowConnectLocalSnapshot {
  if (blockedCharacterIds.size === 0) return snapshot;

  return {
    ...snapshot,
    characters: snapshot.characters.filter((character) => !blockedCharacterIds.has(character.charId)),
    completionSnapshots: snapshot.completionSnapshots.filter((completion) => !blockedCharacterIds.has(completion.charId)),
    raidReservations: snapshot.raidReservations.filter((reservation) => !blockedCharacterIds.has(reservation.charId)),
    encounterSnapshots: (snapshot.encounterSnapshots || []).map((encounter) => ({
      ...encounter,
      matchedCharacterIds: (encounter.matchedCharacterIds || []).filter((charId) => !blockedCharacterIds.has(charId))
    }))
  };
}

function getStoredBlockedDuplicateCharacters(): MeowConnectCharacterConflict[] {
  if (typeof localStorage === 'undefined') return [];

  try {
    const values = JSON.parse(localStorage.getItem(BLOCKED_DUPLICATE_CHARACTERS_STORAGE_KEY) || '[]') as MeowConnectCharacterConflict[];
    const now = Date.now();
    return Array.isArray(values)
      ? values.filter((value) => Number.isFinite(Number(value.charId))).map((value) => ({
          charId: Number(value.charId),
          charName: String(value.charName || value.charId),
          ownerDisplayName: String(value.ownerDisplayName || 'another MeowConnect user'),
          blockedAt: Number.isFinite(Number(value.blockedAt)) ? Number(value.blockedAt) : now - DUPLICATE_CHARACTER_RECHECK_MS
        }))
      : [];
  } catch {
    return [];
  }
}

function rememberBlockedDuplicateCharacters(conflicts: MeowConnectCharacterConflict[]) {
  if (typeof localStorage === 'undefined' || conflicts.length === 0) return;

  const now = Date.now();
  const byCharId = new Map(getStoredBlockedDuplicateCharacters().map((conflict) => [conflict.charId, conflict]));
  for (const conflict of conflicts) {
    byCharId.set(conflict.charId, { ...conflict, blockedAt: now });
  }
  localStorage.setItem(
    BLOCKED_DUPLICATE_CHARACTERS_STORAGE_KEY,
    JSON.stringify(Array.from(byCharId.values()).sort((a, b) => a.charId - b.charId))
  );
}

function forgetBlockedDuplicateCharacters(conflicts: MeowConnectCharacterConflict[]) {
  if (typeof localStorage === 'undefined' || conflicts.length === 0) return;

  const releasedIds = new Set(conflicts.map((conflict) => conflict.charId));
  const keptConflicts = getStoredBlockedDuplicateCharacters().filter((conflict) => !releasedIds.has(conflict.charId));
  localStorage.setItem(
    BLOCKED_DUPLICATE_CHARACTERS_STORAGE_KEY,
    JSON.stringify(keptConflicts.sort((a, b) => a.charId - b.charId))
  );
}

function shouldRecheckBlockedDuplicate(conflict: MeowConnectCharacterConflict): boolean {
  return Date.now() - Number(conflict.blockedAt || 0) >= DUPLICATE_CHARACTER_RECHECK_MS;
}

function logBlockedDuplicateCharacters(
  level: 'info' | 'warn',
  conflicts: MeowConnectCharacterConflict[],
  prefix: string
) {
  for (const conflict of conflicts) {
    writeMeowConnectLog(
      level,
      `${prefix}: ${conflict.charName} (char_id ${conflict.charId}) already exists under ${conflict.ownerDisplayName}.`
    );
  }
}

function dedupeCharacterConflicts(conflicts: MeowConnectCharacterConflict[]): MeowConnectCharacterConflict[] {
  const byCharId = new Map<number, MeowConnectCharacterConflict>();
  for (const conflict of conflicts) {
    byCharId.set(conflict.charId, conflict);
  }
  return Array.from(byCharId.values()).sort((a, b) => a.charName.localeCompare(b.charName, undefined, { sensitivity: 'base' }));
}

function logReleasedDuplicateCharacters(conflicts: MeowConnectCharacterConflict[]) {
  for (const conflict of conflicts) {
    writeMeowConnectLog(
      'info',
      `Released duplicate character block after recheck: ${conflict.charName} (char_id ${conflict.charId}) no longer exists under another MeowConnect profile.`
    );
  }
}

async function applyWhitelistDisplayNames(snapshots: MeowConnectRemoteSnapshot[]): Promise<MeowConnectRemoteSnapshot[]> {
  return Promise.all(snapshots.map(async (snapshot) => ({
    ...snapshot,
    profile: {
      ...snapshot.profile,
      displayName: await resolveDiscordWhitelistDisplayName(snapshot.profile.discordId, snapshot.profile.displayName)
    }
  })));
}

function getEncounterClearedAt(fightStart: number, duration: number): number | undefined {
  if (!fightStart || !duration) return fightStart || undefined;
  return fightStart + Math.max(0, duration);
}

function getRaidMatchForEncounter(bossName: string): { contentId: string; gate?: string } | null {
  const normalizedBossName = bossName.trim().toLowerCase();
  if (!normalizedBossName) return null;

  for (const [contentId, gateGroups] of Object.entries(encounterMap)) {
    for (const [gateLabel, bossNames] of Object.entries(gateGroups)) {
      const gate = normalizeGateLabel(gateLabel);
      if (bossNames.some((entry) => entry.trim().toLowerCase() === normalizedBossName)) {
        return { contentId, gate };
      }
    }
  }

  const raid = RAIDS.find((entry) =>
    entry.name.trim().toLowerCase() === normalizedBossName ||
    normalizedBossName.includes(entry.name.trim().toLowerCase())
  );
  return raid ? { contentId: raid.id } : null;
}

function getRaidName(contentId: string, fallback: string): string {
  return RAIDS.find((raid) => raid.id === contentId)?.name || fallback;
}

function normalizeGateLabel(value?: string | null): string | undefined {
  const match = String(value || '').match(/gate\s*(\d+)|g\s*(\d+)/i);
  const gateNumber = match?.[1] ?? match?.[2];
  return gateNumber ? `Gate ${gateNumber}` : undefined;
}

function buildRemoteSnapshots(
  profiles: MeowProfileRow[],
  characters: MeowCharacterRow[],
  completions: MeowCompletionRow[],
  reservations: MeowReservationRow[],
  encounters: MeowEncounterRow[]
): MeowConnectRemoteSnapshot[] {
  const snapshotsByUser = new Map<string, MeowConnectRemoteSnapshot>();

  for (const profile of profiles) {
    snapshotsByUser.set(profile.user_id, {
      profile: {
        userId: profile.user_id,
        discordId: profile.discord_id,
        displayName: profile.display_name,
        avatarUrl: profile.avatar_url || undefined
      },
      characters: [],
      completionSnapshots: [],
      raidReservations: [],
      encounterSnapshots: [],
      updatedAt: profile.updated_at || new Date().toISOString()
    });
  }

  for (const character of characters) {
    const snapshot = snapshotsByUser.get(character.user_id);
    if (!snapshot) continue;
    snapshot.characters.push({
      charId: Number(character.char_id),
      charName: character.char_name,
      rosterId: character.roster_id,
      rosterName: character.roster_name,
      classId: character.class_id,
      itemLevel: Number(character.item_level || 0),
      combatPower: Number(character.combat_power || 0),
      displayOrder: Number(character.display_order || 0),
      earnsGold: Boolean(character.earns_gold),
      hideFromDashboard: Boolean(character.hide_from_dashboard),
      meowConnectEnabled: Boolean(character.meow_connect_enabled),
      hasStaticReservation: Boolean(character.has_static_reservation)
    });
  }

  for (const completion of completions) {
    const snapshot = snapshotsByUser.get(completion.user_id);
    if (!snapshot) continue;
    snapshot.completionSnapshots.push({
      rosterId: completion.roster_id,
      charId: Number(completion.char_id),
      contentId: completion.content_id,
      difficulty: completion.difficulty || undefined,
      gate: completion.gate || undefined,
      isCompleted: Boolean(completion.is_completed),
      source: completion.source,
      sessionId: completion.session_id || undefined,
      completedAt: completion.completed_at ?? undefined,
      resetCycle: completion.reset_cycle
    });
    if (completion.updated_at && completion.updated_at > snapshot.updatedAt) {
      snapshot.updatedAt = completion.updated_at;
    }
  }

  for (const reservation of reservations) {
    const snapshot = snapshotsByUser.get(reservation.user_id);
    if (!snapshot) continue;
    snapshot.raidReservations.push({
      rosterId: reservation.roster_id,
      charId: Number(reservation.char_id),
      contentId: reservation.content_id,
      difficulty: reservation.difficulty,
      reservedForStatic: Boolean(reservation.reserved_for_static)
    });
    if (reservation.updated_at && reservation.updated_at > snapshot.updatedAt) {
      snapshot.updatedAt = reservation.updated_at;
    }
  }

  for (const encounter of encounters) {
    const snapshot = snapshotsByUser.get(encounter.user_id);
    if (!snapshot) continue;
    snapshot.encounterSnapshots.push({
      localPlayer: encounter.local_player,
      contentId: encounter.content_id,
      raidName: encounter.raid_name,
      difficulty: encounter.difficulty || '',
      gate: encounter.gate || undefined,
      cleared: Boolean(encounter.cleared),
      fightStart: Number(encounter.fight_start || 0),
      duration: Number(encounter.duration || 0),
      clearedAt: getEncounterClearedAt(Number(encounter.fight_start || 0), Number(encounter.duration || 0)) || parseTimestampMs(encounter.updated_at),
      players: parseJsonArray<string>(encounter.players_json),
      matchedCharacterIds: parseJsonArray<number>(encounter.matched_character_ids_json).map((value) => Number(value || 0)).filter(Boolean),
      bibleLogs: parseJsonArray<{ gate?: string; upstreamId?: string; upstream_id?: string }>(encounter.bible_logs_json)
        .map((log) => ({
          gate: log.gate,
          upstreamId: log.upstreamId || log.upstream_id || ''
        }))
        .filter((log) => Boolean(log.upstreamId)),
      resetCycle: encounter.reset_cycle
    });
    if (encounter.updated_at && encounter.updated_at > snapshot.updatedAt) {
      snapshot.updatedAt = encounter.updated_at;
    }
  }

  return Array.from(snapshotsByUser.values()).filter((snapshot) => snapshot.characters.length > 0);
}

function parseJsonArray<T>(value: unknown): T[] {
  if (Array.isArray(value)) return value as T[];
  if (typeof value !== 'string' || !value) return [];
  try {
    const parsed = JSON.parse(value);
    return Array.isArray(parsed) ? parsed as T[] : [];
  } catch {
    return [];
  }
}

function parseTimestampMs(value?: string | null): number | undefined {
  if (!value) return undefined;
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp) ? timestamp : undefined;
}

function writeMeowConnectLog(level: 'info' | 'warn' | 'error' | 'debug', message: string) {
  if (typeof window === 'undefined') return;
  invoke('write_frontend_log', {
    level,
    message: `MeowConnect: ${message}`
  }).catch((error) => {
    console.warn('Failed to write frontend log:', error);
  });
}

async function throwIfSupabaseError<T>(request: PromiseLike<{ data: T; error: unknown }>): Promise<{ data: T }> {
  const result = await request;
  if (result.error) {
    const error = result.error as { message?: string; code?: string };
    const message = error.message || 'Supabase request failed';
    if (
      error.code === '23505' &&
      (message.includes('idx_meow_groups_unique_tag') || message.toLowerCase().includes('group_tag'))
    ) {
      throw new Error('Group tag is already taken.');
    }
    throw new Error(message);
  }
  return { data: result.data };
}

interface MeowProfileRow {
  user_id: string;
  discord_id: string;
  display_name: string;
  avatar_url?: string | null;
  updated_at?: string;
}

interface MeowCharacterRow {
  user_id: string;
  char_id: number;
  roster_id: string;
  roster_name: string;
  char_name: string;
  class_id: string;
  item_level: number;
  combat_power?: number | null;
  display_order: number;
  earns_gold: boolean;
  hide_from_dashboard?: boolean | null;
  meow_connect_enabled?: boolean | null;
  has_static_reservation?: boolean | null;
}

interface MeowCompletionRow {
  user_id: string;
  roster_id: string;
  char_id: number;
  content_id: string;
  gate?: string | null;
  difficulty?: string | null;
  is_completed: boolean;
  source: string;
  session_id?: string | null;
  completed_at?: number | null;
  reset_cycle: string;
  updated_at?: string;
}

interface MeowReservationRow {
  user_id: string;
  roster_id: string;
  char_id: number;
  content_id: string;
  difficulty: string;
  reserved_for_static: boolean;
  updated_at?: string;
}

interface MeowEncounterRow {
  user_id: string;
  local_player: string;
  content_id: string;
  raid_name: string;
  difficulty: string;
  gate: string;
  cleared: boolean;
  fight_start: number;
  duration?: number | null;
  players_json: string[] | string | null;
  matched_character_ids_json: number[] | string | null;
  bible_logs_json?: Array<{ gate?: string; upstreamId?: string; upstream_id?: string }> | string | null;
  reset_cycle: string;
  updated_at?: string;
}

interface MeowCharacterConflictRow {
  char_id: number;
  char_name?: string | null;
  owner_display_name?: string | null;
}
