import { invoke } from '@tauri-apps/api/core';
import { getCurrentSupabaseDiscordProfile, resolveDiscordWhitelistDisplayName, supabase } from '$lib/services/supabase-auth';
import type {
  MeowConnectGroup,
  MeowConnectGroupMember,
  MeowConnectGroupRaidAssignment,
  MeowConnectProfile
} from './types';

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
      const role: MeowConnectGroup['role'] = group.owner_user_id === profile.userId
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

export async function syncMeowConnectGroupTagsToLocal(groups: MeowConnectGroup[]): Promise<void> {
  const profile = await getCurrentSupabaseDiscordProfile();
  const assignments = groups.flatMap((group) =>
    group.assignments
      .filter((assignment) =>
        assignment.userId === profile.userId &&
        Boolean(group.groupTag.trim())
      )
      .map((assignment) => ({
        charId: assignment.charId,
        contentId: assignment.contentId,
        groupId: group.groupId,
        groupTag: group.groupTag.trim().toUpperCase(),
        groupName: group.groupName
      }))
  );

  await invoke('replace_meow_connect_group_raid_tags', { assignments });
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
        difficulty: '',
        reserved_for_static: assignment.reservedForStatic
      },
      { onConflict: 'group_id,user_id,char_id,content_id,difficulty' }
    )
  );
}

export async function removeMeowConnectRaidGroupAssignment(assignment: Pick<MeowConnectGroupRaidAssignment, 'groupId' | 'charId' | 'contentId'>): Promise<void> {
  const profile = await getCurrentSupabaseDiscordProfile();

  await throwIfSupabaseError(
    supabase
      .from('meow_group_raid_assignments')
      .delete()
      .eq('group_id', assignment.groupId)
      .eq('user_id', profile.userId)
      .eq('char_id', assignment.charId)
      .eq('content_id', assignment.contentId)
  );
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

function isMissingMeowGroupSchemaError(err: unknown): boolean {
  const message = String((err as { message?: string })?.message || err || '').toLowerCase();
  return message.includes('meow_groups') ||
    message.includes('meow_group_members') ||
    message.includes('meow_group_raid_assignments') ||
    message.includes('could not find the table');
}

interface MeowProfileRow {
  user_id: string;
  discord_id: string;
  display_name: string;
  avatar_url?: string | null;
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
  difficulty?: string | null;
  reserved_for_static?: boolean | null;
  created_at?: string;
  updated_at?: string;
}
