import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';
import { RAIDS, type Raid } from '$lib/data/raids';
import { encounterMap } from '$lib/data/encounters';
import { getCurrentSupabaseDiscordProfile, resolveDiscordWhitelistDisplayName, supabase } from '$lib/services/supabase-auth';

export interface MeowConnectCharacterSnapshot {
  charId: number;
  charName: string;
  rosterId: string;
  rosterName: string;
  classId: string;
  itemLevel: number;
  combatPower: number;
  displayOrder: number;
  earnsGold: boolean;
  hideFromDashboard: boolean;
  meowConnectEnabled: boolean;
  hasStaticReservation: boolean;
}

export interface MeowConnectRaidReservationSnapshot {
  rosterId: string;
  charId: number;
  contentId: string;
  difficulty: string;
  reservedForStatic: boolean;
}

export interface MeowConnectCompletionSnapshot {
  rosterId: string;
  charId: number;
  contentId: string;
  gate?: string;
  difficulty?: string;
  isCompleted: boolean;
  source: string;
  sessionId?: string;
  completedAt?: number;
  resetCycle?: string;
}

export interface MeowConnectLocalSnapshot {
  generatedAt: number;
  weeklyResetMs: number;
  characters: MeowConnectCharacterSnapshot[];
  completionSnapshots: MeowConnectCompletionSnapshot[];
  raidReservations: MeowConnectRaidReservationSnapshot[];
  encounterSnapshots: MeowConnectEncounterSnapshot[];
}

export interface MeowConnectProfile {
  userId: string;
  discordId: string;
  displayName: string;
  avatarUrl?: string;
}

export interface MeowConnectRemoteSnapshot {
  profile: MeowConnectProfile;
  characters: MeowConnectCharacterSnapshot[];
  completionSnapshots: MeowConnectCompletionSnapshot[];
  raidReservations: MeowConnectRaidReservationSnapshot[];
  encounterSnapshots: MeowConnectEncounterSnapshot[];
  updatedAt: string;
}

export interface MeowConnectFriendConnection {
  userId: string;
  friendUserId: string;
  status: 'pending' | 'accepted' | 'blocked';
  direction: 'incoming' | 'outgoing';
  sharesStatic: boolean;
  profile: MeowConnectProfile;
  updatedAt: string;
}

export interface MeowConnectGroupMember {
  groupId: string;
  userId: string;
  status: 'invited' | 'accepted' | 'declined' | 'removed';
  profile?: MeowConnectProfile;
  invitedByUserId?: string;
  updatedAt: string;
}

export interface MeowConnectGroup {
  groupId: string;
  ownerUserId: string;
  groupName: string;
  groupTag: string;
  role: 'owner' | 'member' | 'invited';
  members: MeowConnectGroupMember[];
  assignments: MeowConnectGroupRaidAssignment[];
  createdAt: string;
  updatedAt: string;
}

export interface MeowConnectGroupRaidAssignment {
  assignmentId?: string;
  groupId: string;
  userId: string;
  rosterId: string;
  charId: number;
  contentId: string;
  difficulty: string;
  reservedForStatic: boolean;
  updatedAt?: string;
}

export interface MeowConnectAvailabilityRow {
  ownerId: string;
  ownerUserId?: string;
  ownerName: string;
  ownerAvatarUrl?: string;
  favoriteKey: string;
  favorite: boolean;
  character: MeowConnectCharacterSnapshot;
  raid: Raid;
  clearedGates: number;
  totalGates: number;
  openGates: number;
  status: 'open' | 'cleared' | 'too_low';
  clearedDifficulty?: string;
  reservedForStatic: boolean;
  staticReservationDetailsVisible: boolean;
  sources: string[];
}

export interface MeowConnectEncounterSnapshot {
  localPlayer: string;
  contentId: string;
  raidName: string;
  difficulty: string;
  gate?: string;
  cleared: boolean;
  fightStart: number;
  duration: number;
  clearedAt?: number;
  players: string[];
  matchedCharacterIds: number[];
  resetCycle?: string;
}

export interface MeowConnectLogEntry extends MeowConnectEncounterSnapshot {
  ownerId: string;
  ownerName: string;
  ownerAvatarUrl?: string;
  participants: MeowConnectLogParticipant[];
  source: 'Manual' | 'LOA Logs' | string;
}

export interface MeowConnectLogParticipant {
  ownerId: string;
  ownerName: string;
  ownerAvatarUrl?: string;
  localPlayer: string;
}

export interface MeowConnectSupabaseConfig {
  url: string;
  anonKey: string;
  accessToken?: string;
}

const FAVORITES_STORAGE_KEY = 'meowConnect.favoriteCharacterKeys';
const CONSENT_STORAGE_KEY = 'meowConnect.consentAccepted';
const FEATURE_ENABLED_STORAGE_KEY = 'meowConnect.featureEnabled';
const REALTIME_ENABLED_STORAGE_KEY = 'meowConnect.realtimeEnabled';
const LAST_UPLOAD_STORAGE_KEY = 'meowConnect.lastUploadAt';
const LAST_UPLOAD_SIGNATURE_STORAGE_KEY = 'meowConnect.lastUploadSignature';
const UNSYNCED_CHANGES_STORAGE_KEY = 'meowConnect.unsyncedChanges';
const BLOCKED_DUPLICATE_CHARACTERS_STORAGE_KEY = 'meowConnect.blockedDuplicateCharacters';
const DEFAULT_AUTO_UPLOAD_COOLDOWN_MS = 15 * 60 * 1000;
const DUPLICATE_CHARACTER_RECHECK_MS = 24 * 60 * 60 * 1000;
let unsyncedReconcileTimer: ReturnType<typeof setTimeout> | null = null;

export interface MeowConnectCharacterConflict {
  charId: number;
  charName: string;
  ownerDisplayName: string;
  blockedAt?: number;
}

export interface MeowConnectUploadResult {
  snapshot: MeowConnectLocalSnapshot;
  uploaded: boolean;
  uploadedCharacterCount?: number;
  duplicateCharacters?: MeowConnectCharacterConflict[];
  skippedReason?: 'unchanged';
}

export type MeowConnectConnectionState = 'inactive' | 'connecting' | 'active' | 'sleeping' | 'offline' | 'login_required';

export interface MeowConnectConnectionStatus {
  state: MeowConnectConnectionState;
  message: string;
  updatedAt: number;
}

export const meowConnectStatus = writable<MeowConnectConnectionStatus>({
  state: hasStoredMeowConnectFeatureEnabled() && hasStoredMeowConnectConsent() ? 'connecting' : 'inactive',
  message: hasStoredMeowConnectFeatureEnabled() && hasStoredMeowConnectConsent()
    ? 'Checking MeowConnect connection.'
    : 'MeowConnect is inactive.',
  updatedAt: Date.now()
});

export const meowConnectHasUnsyncedChanges = writable<boolean>(hasStoredUnsyncedChanges());

interface EncounterPreview {
  current_boss: string;
  local_player: string;
  difficulty: string;
  fight_start: number;
  duration?: number;
  cleared: boolean;
  players: string[];
}

export async function loadMeowConnectLocalSnapshot(): Promise<MeowConnectLocalSnapshot> {
  const snapshot = await invoke<MeowConnectLocalSnapshot>('get_meow_connect_local_snapshot');
  const encounterSnapshots = await loadLocalEncounterSnapshots(snapshot);
  return { ...snapshot, encounterSnapshots };
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
  await throwIfSupabaseError(supabase.from('meow_characters').delete().eq('user_id', profile.userId));

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
    await throwIfSupabaseError(
      supabase.from('meow_encounter_snapshots').upsert(
        (syncedSnapshot.encounterSnapshots || []).map((encounter) => ({
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
          reset_cycle: encounter.resetCycle || resetCycle
        }))
      )
    );
  }

  return { syncedSnapshot, duplicateCharacters: blockedDuplicateCharacters };
}

export async function uploadMeowConnectSnapshotIfNeeded(options: {
  force?: boolean;
  cooldownMs?: number;
} = {}): Promise<MeowConnectUploadResult> {
  const snapshot = await loadMeowConnectLocalSnapshot();
  const signature = buildSnapshotSignature(snapshot);
  const lastSignature = localStorage.getItem(LAST_UPLOAD_SIGNATURE_STORAGE_KEY) || '';
  const lastUploadAt = getStoredTimestamp(LAST_UPLOAD_STORAGE_KEY);
  const cooldownMs = options.cooldownMs ?? DEFAULT_AUTO_UPLOAD_COOLDOWN_MS;
  const unchanged = signature === lastSignature;
  const cooldownActive = Date.now() - lastUploadAt < cooldownMs;

  if (!options.force && unchanged && cooldownActive) {
    if (hasStoredUnsyncedChanges()) {
      clearMeowConnectUnsyncedChanges();
    }
    return { snapshot, uploaded: false, skippedReason: 'unchanged' };
  }

  const syncResult = await syncMeowConnectSnapshot(snapshot);
  localStorage.setItem(LAST_UPLOAD_SIGNATURE_STORAGE_KEY, signature);
  setStoredTimestamp(LAST_UPLOAD_STORAGE_KEY, Date.now());
  clearMeowConnectUnsyncedChanges();

  return {
    snapshot,
    uploaded: true,
    uploadedCharacterCount: syncResult.syncedSnapshot.characters.length,
    duplicateCharacters: syncResult.duplicateCharacters
  };
}

export function getMeowConnectLastUploadAt(): number {
  return getStoredTimestamp(LAST_UPLOAD_STORAGE_KEY);
}

function buildSnapshotSignature(snapshot: MeowConnectLocalSnapshot): string {
  return JSON.stringify({
    reset: snapshot.weeklyResetMs,
    characters: snapshot.characters,
    completions: snapshot.completionSnapshots,
    encounters: snapshot.encounterSnapshots || [],
    reservations: snapshot.raidReservations.filter((reservation) => reservation.reservedForStatic)
  });
}

function getStoredTimestamp(key: string): number {
  const value = Number(localStorage.getItem(key) || 0);
  return Number.isFinite(value) ? value : 0;
}

function setStoredTimestamp(key: string, value: number) {
  localStorage.setItem(key, String(value));
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

export async function loadMeowConnectFriends(): Promise<MeowConnectFriendConnection[]> {
  const profile = await getCurrentSupabaseDiscordProfile();
  const { data: connections } = await throwIfSupabaseError(
    supabase
      .from('meow_friend_connections')
      .select('*')
      .or(`user_id.eq.${profile.userId},friend_user_id.eq.${profile.userId}`)
      .order('updated_at', { ascending: false })
  );

  const rows = ((connections || []) as MeowFriendConnectionRow[]).filter(
    (connection) => connection.user_id === profile.userId || connection.friend_user_id === profile.userId
  );
  const profileIds = Array.from(
    new Set(rows.map((connection) => getOtherUserId(connection, profile.userId)).filter(Boolean))
  );

  if (profileIds.length === 0) {
    return [];
  }

  const { data: profileRows } = await throwIfSupabaseError(
    supabase.from('meow_profiles').select('user_id, discord_id, display_name, avatar_url').in('user_id', profileIds)
  );
  const profilesById = new Map(
    ((profileRows || []) as MeowProfileRow[]).map((entry) => [
      entry.user_id,
      {
        userId: entry.user_id,
        discordId: entry.discord_id,
        displayName: entry.display_name,
        avatarUrl: entry.avatar_url || undefined
      }
    ])
  );

  const friendConnectionsByProfile = new Map<string, MeowConnectFriendConnection>();
  for (const connection of rows) {
    const otherUserId = getOtherUserId(connection, profile.userId);
    const otherProfile = profilesById.get(otherUserId);
    if (!otherProfile) continue;

    const existing = friendConnectionsByProfile.get(otherUserId);
    const direction = connection.user_id === profile.userId ? 'outgoing' : 'incoming';
    const updatedAt = connection.updated_at || connection.created_at || '';
    const nextConnection: MeowConnectFriendConnection = {
      userId: connection.user_id,
      friendUserId: connection.friend_user_id,
      status: connection.status,
      direction,
      sharesStatic: Boolean(connection.user_id === profile.userId && connection.shares_static),
      profile: otherProfile,
      updatedAt
    };

    if (!existing) {
      friendConnectionsByProfile.set(otherUserId, nextConnection);
      continue;
    }

    friendConnectionsByProfile.set(otherUserId, {
      ...existing,
      userId: existing.direction === 'incoming' ? existing.userId : nextConnection.userId,
      friendUserId: existing.direction === 'incoming' ? existing.friendUserId : nextConnection.friendUserId,
      status: existing.status === 'accepted' || nextConnection.status === 'accepted' ? 'accepted' : existing.status,
      direction: existing.status === 'pending' && existing.direction === 'incoming' ? 'incoming' : nextConnection.direction,
      sharesStatic: existing.sharesStatic || nextConnection.sharesStatic,
      updatedAt: updatedAt > existing.updatedAt ? updatedAt : existing.updatedAt
    });
  }

  return applyWhitelistDisplayNamesToFriends(
    Array.from(friendConnectionsByProfile.values()).sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
  );
}

export async function setMeowConnectStaticFriend(connection: MeowConnectFriendConnection, enabled: boolean): Promise<void> {
  await throwIfSupabaseError(
    supabase.rpc('meow_set_static_friend', {
      target_user_id: connection.profile.userId,
      enabled
    })
  );
}

export async function loadMeowConnectGroups(): Promise<MeowConnectGroup[]> {
  const profile = await getCurrentSupabaseDiscordProfile();

  try {
    const { data: groups } = await throwIfSupabaseError(
      supabase
        .from('meow_groups')
        .select('*')
        .order('updated_at', { ascending: false })
    );
    const groupRows = (groups || []) as MeowGroupRow[];
    const groupIds = groupRows.map((group) => group.group_id);

    if (groupIds.length === 0) {
      return [];
    }

    const { data: members } = await throwIfSupabaseError(
      supabase
        .from('meow_group_members')
        .select('*')
        .in('group_id', groupIds)
        .order('updated_at', { ascending: false })
    );
    const { data: assignments } = await throwIfSupabaseError(
      supabase
        .from('meow_group_raid_assignments')
        .select('*')
        .in('group_id', groupIds)
    );
    const memberRows = (members || []) as MeowGroupMemberRow[];
    const assignmentRows = (assignments || []) as MeowGroupAssignmentRow[];
    const profileIds = Array.from(new Set(memberRows.map((member) => member.user_id).filter(Boolean)));
    const profilesById = new Map<string, MeowConnectProfile>();

    if (profileIds.length > 0) {
      const { data: profileRows } = await throwIfSupabaseError(
        supabase.from('meow_profiles').select('user_id, discord_id, display_name, avatar_url').in('user_id', profileIds)
      );

      for (const row of (profileRows || []) as MeowProfileRow[]) {
        profilesById.set(row.user_id, {
          userId: row.user_id,
          discordId: row.discord_id,
          displayName: await resolveDiscordWhitelistDisplayName(row.discord_id, row.display_name),
          avatarUrl: row.avatar_url || undefined
        });
      }
    }

    const membersByGroup = new Map<string, MeowConnectGroupMember[]>();
    for (const member of memberRows) {
      const entries = membersByGroup.get(member.group_id) || [];
      entries.push({
        groupId: member.group_id,
        userId: member.user_id,
        status: member.status,
        invitedByUserId: member.invited_by_user_id || undefined,
        profile: profilesById.get(member.user_id),
        updatedAt: member.updated_at || member.created_at || ''
      });
      membersByGroup.set(member.group_id, entries);
    }

    const assignmentsByGroup = new Map<string, MeowConnectGroupRaidAssignment[]>();
    for (const assignment of assignmentRows) {
      const entries = assignmentsByGroup.get(assignment.group_id) || [];
      entries.push({
        assignmentId: assignment.assignment_id,
        groupId: assignment.group_id,
        userId: assignment.user_id,
        rosterId: assignment.roster_id,
        charId: Number(assignment.char_id),
        contentId: assignment.content_id,
        difficulty: assignment.difficulty || '',
        reservedForStatic: Boolean(assignment.reserved_for_static),
        updatedAt: assignment.updated_at || assignment.created_at || ''
      });
      assignmentsByGroup.set(assignment.group_id, entries);
    }

    return groupRows.map((group) => {
      const groupMembers = (membersByGroup.get(group.group_id) || [])
        .sort((a, b) => {
          if (a.userId === group.owner_user_id) return -1;
          if (b.userId === group.owner_user_id) return 1;
          if (a.status !== b.status) return a.status === 'accepted' ? -1 : 1;
          return (a.profile?.displayName || '').localeCompare(b.profile?.displayName || '', undefined, { sensitivity: 'base' });
        });
      const currentMembership = groupMembers.find((member) => member.userId === profile.userId);
      const role = group.owner_user_id === profile.userId
        ? 'owner'
        : currentMembership?.status === 'invited'
          ? 'invited'
          : 'member';

      return {
        groupId: group.group_id,
        ownerUserId: group.owner_user_id,
        groupName: group.group_name,
        groupTag: group.group_tag || '',
        role,
        members: groupMembers,
        assignments: assignmentsByGroup.get(group.group_id) || [],
        createdAt: group.created_at || '',
        updatedAt: group.updated_at || group.created_at || ''
      };
    }).sort((a, b) => {
      if (a.role !== b.role) return a.role === 'owner' ? -1 : b.role === 'owner' ? 1 : a.role === 'invited' ? -1 : 1;
      return a.groupName.localeCompare(b.groupName, undefined, { sensitivity: 'base' });
    });
  } catch (err) {
    if (isMissingMeowGroupSchemaError(err)) {
      return [];
    }
    throw err;
  }
}

export async function createMeowConnectGroup(groupName: string, groupTag = ''): Promise<void> {
  const cleanName = groupName.trim();
  const cleanTag = groupTag.trim().toUpperCase();
  if (cleanName.length < 2 || cleanName.length > 24) {
    throw new Error('Group name must be 2 to 24 characters.');
  }
  if (cleanTag.length > 5) {
    throw new Error('Group tag must be 5 characters or less.');
  }

  await throwIfSupabaseError(
    supabase.rpc('meow_create_group', {
      group_name: cleanName,
      group_tag: cleanTag
    })
  );
}

export async function renameMeowConnectGroup(groupId: string, groupName: string): Promise<void> {
  const cleanName = groupName.trim();
  if (cleanName.length < 2 || cleanName.length > 24) {
    throw new Error('Group name must be 2 to 24 characters.');
  }

  await throwIfSupabaseError(
    supabase
      .from('meow_groups')
      .update({ group_name: cleanName })
      .eq('group_id', groupId)
  );
}

export async function deleteMeowConnectGroup(groupId: string): Promise<void> {
  await throwIfSupabaseError(
    supabase
      .from('meow_groups')
      .delete()
      .eq('group_id', groupId)
  );
}

export async function leaveMeowConnectGroup(groupId: string): Promise<void> {
  const profile = await getCurrentSupabaseDiscordProfile();

  await throwIfSupabaseError(
    supabase
      .from('meow_group_members')
      .delete()
      .eq('group_id', groupId)
      .eq('user_id', profile.userId)
  );
}

export async function inviteMeowConnectGroupMember(groupId: string, discordId: string): Promise<void> {
  const cleanDiscordId = discordId.trim();
  if (!cleanDiscordId) {
    throw new Error('Enter a Discord ID first.');
  }

  await throwIfSupabaseError(
    supabase.rpc('meow_invite_group_member', {
      target_group_id: groupId,
      target_discord_id: cleanDiscordId
    })
  );
}

export async function searchMeowConnectProfiles(query: string): Promise<MeowConnectProfile[]> {
  try {
    const { data } = await throwIfSupabaseError(
      supabase.rpc('meow_search_profiles', {
        search_text: query.trim()
      })
    );

    const rows = (data || []) as MeowProfileRow[];
    return Promise.all(rows.map(async (row) => ({
      userId: row.user_id,
      discordId: row.discord_id,
      displayName: await resolveDiscordWhitelistDisplayName(row.discord_id, row.display_name),
      avatarUrl: row.avatar_url || undefined
    })));
  } catch (err) {
    if (isMissingMeowProfileSearchError(err)) {
      return [];
    }
    throw err;
  }
}

export async function acceptMeowConnectGroupInvite(groupId: string): Promise<void> {
  await throwIfSupabaseError(
    supabase.rpc('meow_accept_group_invite', {
      target_group_id: groupId
    })
  );
}

export async function assignMeowConnectRaidToGroup(assignment: Omit<MeowConnectGroupRaidAssignment, 'assignmentId' | 'userId' | 'updatedAt'>): Promise<void> {
  const profile = await getCurrentSupabaseDiscordProfile();

  await throwIfSupabaseError(
    supabase.from('meow_group_raid_assignments').upsert(
      {
        group_id: assignment.groupId,
        user_id: profile.userId,
        roster_id: assignment.rosterId,
        char_id: assignment.charId,
        content_id: assignment.contentId,
        difficulty: assignment.difficulty || '',
        reserved_for_static: assignment.reservedForStatic
      },
      { onConflict: 'group_id,user_id,char_id,content_id,difficulty' }
    )
  );
}

export async function removeMeowConnectRaidGroupAssignment(assignment: Pick<MeowConnectGroupRaidAssignment, 'groupId' | 'charId' | 'contentId' | 'difficulty'>): Promise<void> {
  const profile = await getCurrentSupabaseDiscordProfile();

  await throwIfSupabaseError(
    supabase
      .from('meow_group_raid_assignments')
      .delete()
      .eq('group_id', assignment.groupId)
      .eq('user_id', profile.userId)
      .eq('char_id', assignment.charId)
      .eq('content_id', assignment.contentId)
      .eq('difficulty', assignment.difficulty || '')
  );
}

export async function sendMeowConnectFriendRequest(discordId: string): Promise<MeowConnectProfile> {
  const profile = await getCurrentSupabaseDiscordProfile();
  const normalizedDiscordId = discordId.trim();
  if (!normalizedDiscordId) {
    throw new Error('Enter a Discord ID first.');
  }
  if (normalizedDiscordId === profile.discordId) {
    throw new Error('You cannot add yourself.');
  }

  const { data } = await throwIfSupabaseError(
    supabase.rpc('meow_find_profile_by_discord_id', {
      target_discord_id: normalizedDiscordId
    })
  );
  const target = ((data || []) as MeowProfileRow[])[0];
  if (!target) {
    throw new Error('No MeowConnect profile found for that Discord ID. They need to log in and accept MeowConnect first.');
  }

  await throwIfSupabaseError(
    supabase.from('meow_friend_connections').upsert(
      {
        user_id: profile.userId,
        friend_user_id: target.user_id,
        status: 'pending'
      },
      { onConflict: 'user_id,friend_user_id' }
    )
  );

  const displayName = await resolveDiscordWhitelistDisplayName(target.discord_id, target.display_name);

  return {
    userId: target.user_id,
    discordId: target.discord_id,
    displayName,
    avatarUrl: target.avatar_url || undefined
  };
}

export async function acceptMeowConnectFriendRequest(requesterUserId: string): Promise<void> {
  await throwIfSupabaseError(
    supabase.rpc('meow_accept_friend_request', {
      requester_user_id: requesterUserId
    })
  );
}

export async function removeMeowConnectFriend(connection: MeowConnectFriendConnection): Promise<void> {
  const profile = await getCurrentSupabaseDiscordProfile();
  await throwIfSupabaseError(
    supabase
      .from('meow_friend_connections')
      .delete()
      .eq('user_id', profile.userId)
      .eq('friend_user_id', connection.profile.userId)
  );
  await throwIfSupabaseError(
    supabase
      .from('meow_friend_connections')
      .delete()
      .eq('user_id', connection.profile.userId)
      .eq('friend_user_id', profile.userId)
  );
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

async function applyWhitelistDisplayNamesToFriends(
  connections: MeowConnectFriendConnection[]
): Promise<MeowConnectFriendConnection[]> {
  return Promise.all(connections.map(async (connection) => ({
    ...connection,
    profile: {
      ...connection.profile,
      displayName: await resolveDiscordWhitelistDisplayName(connection.profile.discordId, connection.profile.displayName)
    }
  })));
}

export function subscribeMeowConnectChanges(onChange: () => void): () => void {
  if (!isMeowConnectRealtimeEnabled()) {
    return () => {};
  }

  const channel = supabase
    .channel('meow-connect-availability')
    .on('postgres_changes', { event: '*', schema: 'public', table: 'meow_profiles' }, onChange)
    .on('postgres_changes', { event: '*', schema: 'public', table: 'meow_friend_connections' }, onChange)
    .on('postgres_changes', { event: '*', schema: 'public', table: 'meow_characters' }, onChange)
    .on('postgres_changes', { event: '*', schema: 'public', table: 'meow_completion_snapshots' }, onChange)
    .on('postgres_changes', { event: '*', schema: 'public', table: 'meow_encounter_snapshots' }, onChange)
    .on('postgres_changes', { event: '*', schema: 'public', table: 'meow_raid_reservations' }, onChange)
    .subscribe((status, error) => {
      if (!hasMeowConnectConsent()) return;
      if (status === 'SUBSCRIBED') {
        updateMeowConnectStatus('active', 'MeowConnect realtime is connected.');
      } else if (status === 'CHANNEL_ERROR' || status === 'TIMED_OUT') {
        markMeowConnectFailure(error || new Error(`MeowConnect realtime ${status.toLowerCase().replace('_', ' ')}.`));
      } else if (status === 'CLOSED') {
        markMeowConnectSleeping('MeowConnect realtime listener is sleeping. Uploads still run on demand.');
      }
    });

  return () => {
    void supabase.removeChannel(channel);
  };
}

export function hasMeowConnectConsent(): boolean {
  return isMeowConnectFeatureEnabled() && hasStoredMeowConnectConsent();
}

export function setMeowConnectConsent(accepted: boolean) {
  if (accepted) {
    localStorage.setItem(CONSENT_STORAGE_KEY, '1');
    updateMeowConnectStatus('connecting', 'Checking MeowConnect connection.');
  } else {
    localStorage.removeItem(CONSENT_STORAGE_KEY);
    updateMeowConnectStatus('inactive', 'MeowConnect is inactive.');
  }
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('meow-connect-consent-changed', { detail: accepted }));
  }
}

function hasStoredMeowConnectConsent(): boolean {
  if (typeof localStorage === 'undefined') return false;
  return localStorage.getItem(CONSENT_STORAGE_KEY) === '1';
}

function hasStoredMeowConnectFeatureEnabled(): boolean {
  if (typeof localStorage === 'undefined') return true;
  return localStorage.getItem(FEATURE_ENABLED_STORAGE_KEY) !== '0';
}

export function isMeowConnectFeatureEnabled(): boolean {
  return hasStoredMeowConnectFeatureEnabled();
}

export function setMeowConnectFeatureEnabled(enabled: boolean) {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(FEATURE_ENABLED_STORAGE_KEY, enabled ? '1' : '0');
  if (!enabled) {
    updateMeowConnectStatus('inactive', 'MeowConnect is disabled.');
  } else if (hasStoredMeowConnectConsent()) {
    updateMeowConnectStatus('connecting', 'Checking MeowConnect connection.');
  } else {
    updateMeowConnectStatus('inactive', 'MeowConnect is inactive.');
  }
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('meow-connect-feature-changed', { detail: enabled }));
  }
}

export function isMeowConnectRealtimeEnabled(): boolean {
  if (typeof localStorage === 'undefined') return true;
  return localStorage.getItem(REALTIME_ENABLED_STORAGE_KEY) !== '0';
}

export function setMeowConnectRealtimeEnabled(enabled: boolean) {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(REALTIME_ENABLED_STORAGE_KEY, enabled ? '1' : '0');
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('meow-connect-realtime-changed', { detail: enabled }));
  }
}

export function markMeowConnectUnsyncedChanges(reason = 'Local MeowConnect sharing settings changed.') {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(UNSYNCED_CHANGES_STORAGE_KEY, '1');
  }
  meowConnectHasUnsyncedChanges.set(true);
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('meow-connect-unsynced-changed', { detail: { dirty: true, reason } }));
  }
  scheduleMeowConnectUnsyncedReconciliation();
}

export function clearMeowConnectUnsyncedChanges() {
  if (typeof localStorage !== 'undefined') {
    localStorage.removeItem(UNSYNCED_CHANGES_STORAGE_KEY);
  }
  meowConnectHasUnsyncedChanges.set(false);
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('meow-connect-unsynced-changed', { detail: { dirty: false } }));
  }
}

function hasStoredUnsyncedChanges(): boolean {
  if (typeof localStorage === 'undefined') return false;
  return localStorage.getItem(UNSYNCED_CHANGES_STORAGE_KEY) === '1';
}

function scheduleMeowConnectUnsyncedReconciliation() {
  if (typeof window === 'undefined') return;
  if (unsyncedReconcileTimer) clearTimeout(unsyncedReconcileTimer);
  unsyncedReconcileTimer = setTimeout(() => {
    unsyncedReconcileTimer = null;
    void reconcileMeowConnectUnsyncedChanges();
  }, 350);
}

async function reconcileMeowConnectUnsyncedChanges() {
  if (typeof localStorage === 'undefined') return;
  const lastSignature = localStorage.getItem(LAST_UPLOAD_SIGNATURE_STORAGE_KEY) || '';
  if (!lastSignature) return;

  try {
    const snapshot = await loadMeowConnectLocalSnapshot();
    if (buildSnapshotSignature(snapshot) === lastSignature) {
      clearMeowConnectUnsyncedChanges();
    }
  } catch (error) {
    console.warn('Failed to reconcile MeowConnect unsynced state:', error);
  }
}

export function updateMeowConnectStatus(state: MeowConnectConnectionState, message: string) {
  meowConnectStatus.set({
    state,
    message,
    updatedAt: Date.now()
  });
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

export function markMeowConnectConnecting(message = 'Checking MeowConnect connection.') {
  if (!hasMeowConnectConsent()) {
    updateMeowConnectStatus('inactive', 'MeowConnect is inactive.');
    return;
  }
  updateMeowConnectStatus('connecting', message);
}

export function markMeowConnectActive(message = 'MeowConnect is active.') {
  if (!hasMeowConnectConsent()) {
    updateMeowConnectStatus('inactive', 'MeowConnect is inactive.');
    return;
  }
  updateMeowConnectStatus('active', message);
}

export function markMeowConnectSleeping(message = 'MeowConnect realtime listener is sleeping. Uploads still run on demand.') {
  if (!hasMeowConnectConsent()) {
    updateMeowConnectStatus('inactive', 'MeowConnect is inactive.');
    return;
  }
  updateMeowConnectStatus('sleeping', message);
  writeMeowConnectLog('info', message);
}

export function markMeowConnectFailure(error: unknown) {
  if (!hasMeowConnectConsent()) {
    updateMeowConnectStatus('inactive', 'MeowConnect is inactive.');
    return;
  }

  const message = formatErrorMessage(error);
  const state: MeowConnectConnectionState = isLoginRequiredError(message) ? 'login_required' : 'offline';
  writeMeowConnectLog(
    state === 'login_required' ? 'warn' : 'error',
    state === 'login_required'
      ? `Login required: ${message}`
      : `Cloud request failed: ${message}`
  );
  updateMeowConnectStatus(
    state,
    state === 'login_required'
      ? 'MeowConnect needs Discord login.'
      : 'MeowConnect cloud is currently unreachable.'
  );
}

function formatErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error || '');
}

function isLoginRequiredError(message: string): boolean {
  const normalized = message.toLowerCase();
  return normalized.includes('requires discord login') ||
    normalized.includes('missing discord identity') ||
    normalized.includes('auth session missing') ||
    normalized.includes('jwt') ||
    normalized.includes('login');
}

export function getMeowConnectFavoriteKey(ownerId: string, charId: number): string {
  return `${ownerId}:${charId}`;
}

export function loadFavoritePlayerIds(): Set<string> {
  try {
    const values = JSON.parse(localStorage.getItem(FAVORITES_STORAGE_KEY) || '[]');
    return new Set(Array.isArray(values) ? values.map((value) => String(value)).filter(Boolean) : []);
  } catch {
    return new Set();
  }
}

export function saveFavoritePlayerIds(ids: Set<string>) {
  localStorage.setItem(FAVORITES_STORAGE_KEY, JSON.stringify(Array.from(ids).sort()));
}

export function toggleFavoritePlayerId(favoriteKey: string): Set<string> {
  const ids = loadFavoritePlayerIds();
  if (ids.has(favoriteKey)) {
    ids.delete(favoriteKey);
  } else {
    ids.add(favoriteKey);
  }
  saveFavoritePlayerIds(ids);
  return ids;
}

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

  const localRows = localSnapshot
    ? buildSnapshotRows(
        {
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
        },
        raids,
        difficulty,
        favoriteIds
      )
    : [];

  return localRows
    .concat(remoteSnapshots.flatMap((snapshot) => buildSnapshotRows(snapshot, raids, difficulty, favoriteIds)))
    .sort((a, b) =>
      Number(b.favorite) - Number(a.favorite) ||
      statusRank(a.status) - statusRank(b.status) ||
      b.character.itemLevel - a.character.itemLevel ||
      a.ownerName.localeCompare(b.ownerName) ||
      a.character.displayOrder - b.character.displayOrder ||
      a.character.charName.localeCompare(b.character.charName)
    );
}

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

export async function meowConnectSupabaseRequest<T>(
  config: MeowConnectSupabaseConfig,
  path: string,
  init: RequestInit = {}
): Promise<T> {
  const url = `${config.url.replace(/\/$/, '')}/rest/v1/${path.replace(/^\//, '')}`;
  const response = await fetch(url, {
    ...init,
    headers: {
      apikey: config.anonKey,
      authorization: `Bearer ${config.accessToken || config.anonKey}`,
      'content-type': 'application/json',
      ...(init.headers || {})
    }
  });

  if (!response.ok) {
    const detail = await response.text();
    throw new Error(`MeowConnect Supabase request failed (${response.status}): ${detail}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json() as Promise<T>;
}

function buildSnapshotRows(
  snapshot: MeowConnectRemoteSnapshot,
  raids: Raid[],
  selectedDifficulty: string,
  favoriteIds: Set<string>
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
        (encountersByCharacter.get(character.character.charId) || []).concat((snapshot.encounterSnapshots || []).filter((encounter) =>
          encounter.cleared &&
          encounter.contentId === raidId &&
          encounter.localPlayer.trim().toLowerCase() === character.character.charName.trim().toLowerCase()
        ))
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
          resetCycle: String(snapshot.weeklyResetMs || 0)
        };
        return [entry];
      });
  } catch {
    return [];
  }
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

function normalizeLogSource(source?: string): 'Manual' | 'LOA Logs' | string {
  const normalized = String(source || '').trim().toLowerCase();
  if (!normalized || normalized === 'manual') return 'Manual';
  if (normalized === 'loalogs' || normalized === 'loa logs') return 'LOA Logs';
  return source || 'Manual';
}

function normalizeLogDifficulty(value?: string): string {
  return String(value || '').trim().toLowerCase();
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

function sameDifficulty(a: string, b: string): boolean {
  return a.trim().toLowerCase() === b.trim().toLowerCase();
}

function statusRank(status: MeowConnectAvailabilityRow['status']): number {
  if (status === 'open') return 0;
  if (status === 'too_low') return 1;
  return 2;
}

async function throwIfSupabaseError<T>(request: PromiseLike<{ data: T; error: unknown }>): Promise<{ data: T }> {
  const result = await request;
  if (result.error) {
    const error = result.error as { message?: string };
    throw new Error(error.message || 'Supabase request failed');
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
  item_level: number | string;
  combat_power: number | string;
  display_order: number;
  earns_gold: boolean;
  hide_from_dashboard: boolean;
  meow_connect_enabled: boolean;
  has_static_reservation?: boolean;
}

interface MeowCharacterConflictRow {
  char_id: number | string;
  char_name?: string | null;
  owner_display_name?: string | null;
}

interface MeowCompletionRow {
  user_id: string;
  roster_id: string;
  char_id: number;
  content_id: string;
  difficulty: string;
  gate: string;
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
  reset_cycle: string;
  updated_at?: string;
}

interface MeowFriendConnectionRow {
  user_id: string;
  friend_user_id: string;
  status: 'pending' | 'accepted' | 'blocked';
  shares_static?: boolean;
  created_at?: string;
  updated_at?: string;
}

interface MeowGroupRow {
  group_id: string;
  owner_user_id: string;
  group_name: string;
  group_tag?: string | null;
  created_at?: string;
  updated_at?: string;
}

interface MeowGroupMemberRow {
  group_id: string;
  user_id: string;
  invited_by_user_id?: string | null;
  status: 'invited' | 'accepted' | 'declined' | 'removed';
  created_at?: string;
  updated_at?: string;
}

interface MeowGroupAssignmentRow {
  assignment_id?: string;
  group_id: string;
  user_id: string;
  roster_id: string;
  char_id: number;
  content_id: string;
  difficulty: string;
  reserved_for_static: boolean;
  created_at?: string;
  updated_at?: string;
}

function getOtherUserId(connection: MeowFriendConnectionRow, currentUserId: string): string {
  return connection.user_id === currentUserId ? connection.friend_user_id : connection.user_id;
}

function isMissingMeowGroupSchemaError(err: unknown): boolean {
  const message = String((err as { message?: string })?.message || err || '').toLowerCase();
  return message.includes('meow_groups') ||
    message.includes('meow_group_members') ||
    message.includes('meow_group_raid_assignments') ||
    message.includes('could not find the table');
}

function isMissingMeowProfileSearchError(err: unknown): boolean {
  const message = String((err as { message?: string })?.message || err || '').toLowerCase();
  return message.includes('meow_search_profiles') || message.includes('could not find the function');
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
      resetCycle: encounter.reset_cycle
    });
    if (encounter.updated_at && encounter.updated_at > snapshot.updatedAt) {
      snapshot.updatedAt = encounter.updated_at;
    }
  }

  return Array.from(snapshotsByUser.values()).filter((snapshot) => snapshot.characters.length > 0);
}

function parseTimestampMs(value?: string | null): number | undefined {
  if (!value) return undefined;
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp) ? timestamp : undefined;
}
