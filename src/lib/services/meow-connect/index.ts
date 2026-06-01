import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';
import { getCurrentSupabaseDiscordProfile, resolveDiscordWhitelistDisplayName, supabase } from '$lib/services/supabase-auth';

// Temporary public MeowConnect API barrel plus app-level orchestration.
// Lower-level Supabase, snapshot, group, log, and preference helpers live in `services/meow-connect/*`.
export { meowConnectSupabaseRequest } from './api';
export {
  buildMeowConnectAvailabilityRows,
  getMeowConnectRaidDifficulties,
  getMeowConnectRaidOptions
} from './availability';
export {
  acceptMeowConnectGroupInvite,
  assignMeowConnectRaidToGroup,
  createMeowConnectGroup,
  deleteMeowConnectGroup,
  inviteMeowConnectGroupMember,
  leaveMeowConnectGroup,
  loadMeowConnectGroups,
  removeMeowConnectRaidGroupAssignment,
  renameMeowConnectGroup,
  syncMeowConnectGroupTagsToLocal
} from './groups';
export { buildMeowConnectLogEntries } from './logs';
import {
  fetchMeowConnectRemoteSnapshots,
  loadMeowConnectLocalSnapshot,
  syncMeowConnectSnapshot
} from './snapshot';
export {
  fetchMeowConnectRemoteSnapshots,
  loadMeowConnectLocalSnapshot,
  syncMeowConnectSnapshot
};
import {
  LAST_UPLOAD_SIGNATURE_STORAGE_KEY,
  LAST_UPLOAD_STORAGE_KEY,
  clearStoredUnsyncedChanges,
  getStoredTimestamp,
  hasStoredMeowConnectConsent,
  hasStoredMeowConnectFeatureEnabled,
  hasStoredUnsyncedChanges,
  isStoredMeowConnectRealtimeEnabled,
  loadFavoritePlayerIds,
  saveFavoritePlayerIds,
  setStoredMeowConnectConsent,
  setStoredMeowConnectFeatureEnabled,
  setStoredMeowConnectRealtimeEnabled,
  setStoredTimestamp,
  setStoredUnsyncedChanges,
  toggleFavoritePlayerId
} from './preferences';
import type {
  MeowConnectConnectionState,
  MeowConnectConnectionStatus,
  MeowConnectFriendConnection,
  MeowConnectGroup,
  MeowConnectLocalSnapshot,
  MeowConnectPendingRequests,
  MeowConnectProfile,
  MeowConnectUploadResult
} from './types';

export type {
  MeowConnectAvailabilityRow,
  MeowConnectCharacterConflict,
  MeowConnectCharacterSnapshot,
  MeowConnectCompletionSnapshot,
  MeowConnectConnectionState,
  MeowConnectConnectionStatus,
  MeowConnectEncounterSnapshot,
  MeowConnectFriendConnection,
  MeowConnectGroup,
  MeowConnectGroupMember,
  MeowConnectGroupRaidAssignment,
  MeowConnectLocalSnapshot,
  MeowConnectLogEntry,
  MeowConnectLogParticipant,
  MeowConnectPendingRequests,
  MeowConnectProfile,
  MeowConnectRaidReservationSnapshot,
  MeowConnectRemoteSnapshot,
  MeowConnectSupabaseConfig,
  MeowConnectUploadResult
} from './types';
export {
  getMeowConnectFavoriteKey,
  loadFavoritePlayerIds,
  saveFavoritePlayerIds,
  toggleFavoritePlayerId
} from './preferences';

const DEFAULT_AUTO_UPLOAD_COOLDOWN_MS = 15 * 60 * 1000;
let unsyncedReconcileTimer: ReturnType<typeof setTimeout> | null = null;

export const meowConnectStatus = writable<MeowConnectConnectionStatus>({
  state: hasStoredMeowConnectFeatureEnabled() && hasStoredMeowConnectConsent() ? 'connecting' : 'inactive',
  message: hasStoredMeowConnectFeatureEnabled() && hasStoredMeowConnectConsent()
    ? 'Checking MeowConnect connection.'
    : 'MeowConnect is inactive.',
  updatedAt: Date.now()
});

export const meowConnectHasUnsyncedChanges = writable<boolean>(hasStoredUnsyncedChanges());

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

export async function loadMeowConnectPendingRequests(): Promise<MeowConnectPendingRequests> {
  const profile = await getCurrentSupabaseDiscordProfile();
  const [{ data: friendRows }, { data: inviteRows }] = await Promise.all([
    throwIfSupabaseError(
      supabase
        .from('meow_friend_connections')
        .select('*')
        .eq('friend_user_id', profile.userId)
        .eq('status', 'pending')
        .order('updated_at', { ascending: false })
    ),
    throwIfSupabaseError(
      supabase
        .from('meow_group_members')
        .select('*')
        .eq('user_id', profile.userId)
        .eq('status', 'invited')
        .order('updated_at', { ascending: false })
    )
  ]);

  const incomingFriendRows = (friendRows || []) as MeowFriendConnectionRow[];
  const pendingInviteRows = (inviteRows || []) as MeowGroupMemberRow[];
  const friendProfileIds = Array.from(new Set(incomingFriendRows.map((connection) => connection.user_id).filter(Boolean)));
  const invitedGroupIds = Array.from(new Set(pendingInviteRows.map((member) => member.group_id).filter(Boolean)));
  const [{ data: friendProfiles }, { data: invitedGroups }] = await Promise.all([
    friendProfileIds.length > 0
      ? throwIfSupabaseError(
          supabase.from('meow_profiles').select('user_id, discord_id, display_name, avatar_url').in('user_id', friendProfileIds)
        )
      : Promise.resolve({ data: [] as MeowProfileRow[] }),
    invitedGroupIds.length > 0
      ? throwIfSupabaseError(
          supabase.from('meow_groups').select('*').in('group_id', invitedGroupIds)
        )
      : Promise.resolve({ data: [] as MeowGroupRow[] })
  ]);

  const profilesById = new Map(
    ((friendProfiles || []) as MeowProfileRow[]).map((entry) => [
      entry.user_id,
      {
        userId: entry.user_id,
        discordId: entry.discord_id,
        displayName: entry.display_name,
        avatarUrl: entry.avatar_url || undefined
      }
    ])
  );
  const groupsById = new Map(((invitedGroups || []) as MeowGroupRow[]).map((group) => [group.group_id, group]));
  const friendRequests = incomingFriendRows
    .map((connection): MeowConnectFriendConnection | null => {
      const otherProfile = profilesById.get(connection.user_id);
      if (!otherProfile) return null;
      return {
        userId: connection.user_id,
        friendUserId: connection.friend_user_id,
        status: connection.status,
        direction: 'incoming',
        sharesStatic: Boolean(connection.shares_static),
        profile: otherProfile,
        updatedAt: connection.updated_at || connection.created_at || ''
      };
    })
    .filter((connection): connection is MeowConnectFriendConnection => Boolean(connection));
  const groupInvites = pendingInviteRows
    .map((member): MeowConnectGroup | null => {
      const group = groupsById.get(member.group_id);
      if (!group) return null;
      return {
        groupId: group.group_id,
        ownerUserId: group.owner_user_id,
        groupName: group.group_name,
        groupTag: group.group_tag || '',
        role: 'invited',
        members: [{
          groupId: member.group_id,
          userId: member.user_id,
          status: member.status,
          invitedByUserId: member.invited_by_user_id || undefined,
          updatedAt: member.updated_at || member.created_at || ''
        }],
        assignments: [],
        createdAt: group.created_at || '',
        updatedAt: group.updated_at || group.created_at || ''
      };
    })
    .filter((group): group is MeowConnectGroup => Boolean(group));

  return {
    friendRequests: await applyWhitelistDisplayNamesToFriends(friendRequests),
    groupInvites
  };
}

export async function setMeowConnectStaticFriend(connection: MeowConnectFriendConnection, enabled: boolean): Promise<void> {
  await throwIfSupabaseError(
    supabase.rpc('meow_set_static_friend', {
      target_user_id: connection.profile.userId,
      enabled
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
    .on('postgres_changes', { event: '*', schema: 'public', table: 'meow_groups' }, onChange)
    .on('postgres_changes', { event: '*', schema: 'public', table: 'meow_group_members' }, onChange)
    .on('postgres_changes', { event: '*', schema: 'public', table: 'meow_group_raid_assignments' }, onChange)
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
  setStoredMeowConnectConsent(accepted);
  updateMeowConnectStatus(
    accepted ? 'connecting' : 'inactive',
    accepted ? 'Checking MeowConnect connection.' : 'MeowConnect is inactive.'
  );
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('meow-connect-consent-changed', { detail: accepted }));
  }
}

export function isMeowConnectFeatureEnabled(): boolean {
  return hasStoredMeowConnectFeatureEnabled();
}

export function setMeowConnectFeatureEnabled(enabled: boolean) {
  setStoredMeowConnectFeatureEnabled(enabled);
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
  return isStoredMeowConnectRealtimeEnabled();
}

export function setMeowConnectRealtimeEnabled(enabled: boolean) {
  setStoredMeowConnectRealtimeEnabled(enabled);
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('meow-connect-realtime-changed', { detail: enabled }));
  }
}

export function markMeowConnectUnsyncedChanges(reason = 'Local MeowConnect sharing settings changed.') {
  setStoredUnsyncedChanges();
  meowConnectHasUnsyncedChanges.set(true);
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('meow-connect-unsynced-changed', { detail: { dirty: true, reason } }));
  }
  scheduleMeowConnectUnsyncedReconciliation();
}

export function clearMeowConnectUnsyncedChanges() {
  clearStoredUnsyncedChanges();
  meowConnectHasUnsyncedChanges.set(false);
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('meow-connect-unsynced-changed', { detail: { dirty: false } }));
  }
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

export function logMeowConnectRequest(message: string, level: 'info' | 'warn' | 'error' | 'debug' = 'debug') {
  writeMeowConnectLog(level, message);
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

function getOtherUserId(connection: MeowFriendConnectionRow, currentUserId: string): string {
  return connection.user_id === currentUserId ? connection.friend_user_id : connection.user_id;
}

function isMissingMeowProfileSearchError(err: unknown): boolean {
  const message = String((err as { message?: string })?.message || err || '').toLowerCase();
  return message.includes('meow_search_profiles') || message.includes('could not find the function');
}

