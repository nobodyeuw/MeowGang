import {
  RAID_MANAGEMENT_BOOTSTRAP_ADMIN_DISCORD_IDS,
  getRaidSignupRaid,
  type RaidManagementAccessMember,
  type RaidManagementRequest,
  type RaidManagementRunType,
  type RaidSignupCustomRaid,
  type RaidSignupSheetRaid,
  type RaidSignupSheet
} from '$lib/data/raid-management';
import { supabase } from '$lib/services/supabase-auth';

const ACCESS_STORAGE_KEY = 'raidManagement.accessMembers';
const SHEETS_STORAGE_KEY = 'raidManagement.signupSheets';
const TICKET_TABLE = 'raid_management_tickets';
const LEGACY_TICKET_TABLE = 'meowtator_project_requests';

function readJson<T>(key: string, fallback: T): T {
  if (typeof localStorage === 'undefined') return fallback;
  try {
    return JSON.parse(localStorage.getItem(key) || '') as T;
  } catch {
    return fallback;
  }
}

function writeJson<T>(key: string, value: T) {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(key, JSON.stringify(value));
}

function isMissingTicketTableError(error: { message?: string; code?: string } | null): boolean {
  const message = String(error?.message || '');
  return error?.code === '42P01' || /raid_management_tickets/i.test(message);
}

export function getRaidManagementAccessMembers(): RaidManagementAccessMember[] {
  return readJson<RaidManagementAccessMember[]>(ACCESS_STORAGE_KEY, []);
}

export function hasRaidManagementAccess(discordId?: string | null): boolean {
  const normalizedId = String(discordId || '').trim();
  if (!normalizedId) return false;
  if (RAID_MANAGEMENT_BOOTSTRAP_ADMIN_DISCORD_IDS.includes(normalizedId)) return true;
  return getRaidManagementAccessMembers().some((member) => member.discordId === normalizedId);
}

export async function loadRaidManagementAccessMembers(): Promise<RaidManagementAccessMember[]> {
  const { data, error } = await supabase
    .from('raid_management_access')
    .select('discord_id, display_name, granted_at')
    .order('display_name', { ascending: true });

  if (error && /display_name|granted_at/i.test(error.message)) {
    const fallback = await supabase
      .from('raid_management_access')
      .select('discord_id')
      .order('discord_id', { ascending: true });

    if (fallback.error) {
      throw new Error(fallback.error.message);
    }

    return ((fallback.data || []) as Array<{ discord_id: string }>).map((row) => ({
      discordId: row.discord_id,
      displayName: row.discord_id,
      grantedAt: 0
    }));
  }

  if (error) {
    throw new Error(error.message);
  }

  return ((data || []) as Array<{ discord_id: string; display_name?: string | null; granted_at?: string | null }>).map((row) => ({
    discordId: row.discord_id,
    displayName: row.display_name || row.discord_id,
    grantedAt: row.granted_at ? new Date(row.granted_at).getTime() : 0
  }));
}

export async function hasRaidManagementAccessRemote(discordId?: string | null): Promise<boolean> {
  const normalizedId = String(discordId || '').trim();
  if (!normalizedId) return false;
  if (hasRaidManagementAccess(normalizedId)) return true;

  const { data, error } = await supabase
    .from('raid_management_access')
    .select('discord_id')
    .eq('discord_id', normalizedId)
    .maybeSingle();

  if (error) {
    throw new Error(error.message);
  }

  return Boolean(data);
}

export async function grantRaidManagementAccessMember(
  member: Omit<RaidManagementAccessMember, 'grantedAt'>,
  grantedByDiscordId = ''
): Promise<void> {
  const normalizedMember = {
    discord_id: member.discordId.trim(),
    display_name: member.displayName.trim() || member.discordId.trim(),
    granted_by_discord_id: grantedByDiscordId.trim() || null
  };
  if (!normalizedMember.discord_id) return;

  const { error } = await supabase
    .from('raid_management_access')
    .upsert(normalizedMember, { onConflict: 'discord_id' });

  if (error && /display_name|granted_by_discord_id/i.test(error.message)) {
    const fallback = await supabase
      .from('raid_management_access')
      .upsert({ discord_id: normalizedMember.discord_id }, { onConflict: 'discord_id' });

    if (fallback.error) {
      throw new Error(fallback.error.message);
    }
    return;
  }

  if (error) {
    throw new Error(error.message);
  }
}

export async function revokeRaidManagementAccessMember(discordId: string): Promise<void> {
  const { error } = await supabase
    .from('raid_management_access')
    .delete()
    .eq('discord_id', discordId);

  if (error) {
    throw new Error(error.message);
  }
}

export function setRaidManagementAccessMember(member: Omit<RaidManagementAccessMember, 'grantedAt'>) {
  const normalizedMember = {
    discordId: member.discordId.trim(),
    displayName: member.displayName.trim() || member.discordId.trim(),
    grantedAt: Date.now()
  };
  if (!normalizedMember.discordId) return;

  const nextMembers = [
    ...getRaidManagementAccessMembers().filter((existing) => existing.discordId !== normalizedMember.discordId),
    normalizedMember
  ].sort((a, b) => a.displayName.localeCompare(b.displayName, undefined, { sensitivity: 'base' }));

  writeJson(ACCESS_STORAGE_KEY, nextMembers);
}

export function removeRaidManagementAccessMember(discordId: string) {
  writeJson(
    ACCESS_STORAGE_KEY,
    getRaidManagementAccessMembers().filter((member) => member.discordId !== discordId)
  );
}

export function getRaidSignupSheets(): RaidSignupSheet[] {
  return readJson<RaidSignupSheet[]>(SHEETS_STORAGE_KEY, [])
    .sort((a, b) => b.updatedAt - a.updatedAt);
}

export function saveRaidSignupSheet(sheet: RaidSignupSheet) {
  const now = Date.now();
  const nextSheet = {
    ...sheet,
    preRegisteredMembers: sheet.preRegisteredMembers || [],
    updatedAt: now,
    createdAt: sheet.createdAt || now
  };
  writeJson(SHEETS_STORAGE_KEY, [
    nextSheet,
    ...getRaidSignupSheets().filter((existing) => existing.id !== nextSheet.id)
  ]);
}

export function deleteRaidSignupSheet(sheetId: string) {
  writeJson(
    SHEETS_STORAGE_KEY,
    getRaidSignupSheets().filter((sheet) => sheet.id !== sheetId)
  );
}

function parseDiscordTimestampToIso(value: string): string | null {
  const match = value.match(/<t:(\d+):[a-zA-Z]>/);
  if (!match) return null;
  return new Date(Number(match[1]) * 1000).toISOString();
}

interface RaidManagementRequestRow {
  request_id: string;
  requester_discord_id: string;
  requester_display_name: string;
  title: string;
  category: string;
  description: string;
  status: 'open' | 'accepted' | 'declined' | 'closed';
  reviewed_by_discord_id: string | null;
  review_note?: string | null;
  created_at: string;
}

interface RaidSignupSheetRow {
  sheet_id: string;
  event_id: string | null;
  title: string;
  run_type: RaidManagementRunType | string;
  starts_at: string | null;
  dps_spots: number | null;
  support_spots: number | null;
  any_spots: number | null;
  experienced_minimum: number | null;
  created_at: string;
  updated_at: string;
}

interface RaidSignupRaidRow {
  sheet_id: string;
  raid_id: string;
  raid_name: string;
  spots: number;
  dps_spots: number;
  support_spots: number;
  sort_order: number | null;
}

interface RaidSignupEntryRow {
  sheet_id: string;
  discord_id: string;
  display_name: string;
  role: 'dps' | 'support' | 'any';
  status: 'learner' | 'experienced' | 'can_help';
}

function extractRequestField(description: string, label: string): string {
  const line = description
    .split('\n')
    .find((value) => value.toLowerCase().startsWith(`${label.toLowerCase()}:`));
  return line ? line.slice(label.length + 1).trim() : '';
}

function extractFirstRequestField(description: string, labels: string[]): string {
  for (const label of labels) {
    const value = extractRequestField(description, label);
    if (value) return value;
  }
  return '';
}

function toDiscordTimestamp(value: string | null): string {
  if (!value) return '';
  const timestamp = Math.floor(new Date(value).getTime() / 1000);
  return Number.isFinite(timestamp) ? `<t:${timestamp}:F>` : '';
}

function isRaidSignupSheetRaid(value: RaidSignupSheetRaid | undefined): value is RaidSignupSheetRaid {
  return Boolean(value);
}

function stripStructuredRequestDetails(description: string): string {
  return description
    .split('\n')
    .filter((line) => !/^(Raid\(s\)|Date Window|Time Window|Server Date|Server Time):/i.test(line.trim()))
    .join('\n')
    .trim();
}

export async function loadRaidManagementRequests(): Promise<RaidManagementRequest[]> {
  const selectWithReviewNote =
    'request_id, requester_discord_id, requester_display_name, title, category, description, status, reviewed_by_discord_id, review_note, created_at';
  const selectWithoutReviewNote =
    'request_id, requester_discord_id, requester_display_name, title, category, description, status, reviewed_by_discord_id, created_at';

  let data: RaidManagementRequestRow[] | null = null;
  let error: { message: string } | null = null;

  const primary = await supabase
    .from(TICKET_TABLE)
    .select(selectWithReviewNote)
    .in('status', ['accepted', 'declined', 'closed'])
    .order('created_at', { ascending: false })
    .limit(100);

  data = (primary.data || null) as RaidManagementRequestRow[] | null;
  error = primary.error;

  if (error && isMissingTicketTableError(error)) {
    const legacy = await supabase
      .from(LEGACY_TICKET_TABLE)
      .select(selectWithReviewNote)
      .in('status', ['accepted', 'declined', 'closed'])
      .order('created_at', { ascending: false })
      .limit(100);

    data = (legacy.data || null) as RaidManagementRequestRow[] | null;
    error = legacy.error;
  }

  if (error && /review_note/i.test(error.message)) {
    const fallback = await supabase
      .from(isMissingTicketTableError(primary.error) ? LEGACY_TICKET_TABLE : TICKET_TABLE)
      .select(selectWithoutReviewNote)
      .in('status', ['accepted', 'declined', 'closed'])
      .order('created_at', { ascending: false })
      .limit(100);

    data = (fallback.data || null) as RaidManagementRequestRow[] | null;
    error = fallback.error;
  }

  if (error) {
    throw new Error(error.message);
  }

  return ((data || []) as RaidManagementRequestRow[]).map((row) => {
    const raidNames = extractRequestField(row.description, 'Raid(s)')
      .split(',')
      .map((value) => value.trim())
      .filter(Boolean);

    return {
      id: row.request_id,
      title: row.title,
      requester: row.requester_display_name || row.requester_discord_id,
      discordId: row.requester_discord_id,
      raidNames,
      category: row.category,
      status: row.status,
      decidedBy: row.reviewed_by_discord_id || '',
      reviewNote: row.review_note || '',
      createdAt: new Date(row.created_at).getTime(),
      dateWindow: extractFirstRequestField(row.description, ['Server Date', 'Date Window']),
      timeWindow: extractFirstRequestField(row.description, ['Server Time', 'Time Window']),
      details: stripStructuredRequestDetails(row.description)
    };
  });
}

export async function updateRaidManagementRequestStatus(
  requestId: string,
  status: 'accepted' | 'closed',
  reviewerDiscordId = ''
): Promise<void> {
  const { error } = await supabase
    .from(TICKET_TABLE)
    .update({
      status,
      reviewed_by_discord_id: reviewerDiscordId || null
    })
    .eq('request_id', requestId);

  if (error && isMissingTicketTableError(error)) {
    const { error: legacyError } = await supabase
      .from(LEGACY_TICKET_TABLE)
      .update({
        status,
        reviewed_by_discord_id: reviewerDiscordId || null
      })
      .eq('request_id', requestId);

    if (legacyError) {
      throw new Error(legacyError.message);
    }
    return;
  }

  if (error) {
    throw new Error(error.message);
  }
}

export async function loadRaidSignupSheetsFromSupabase(): Promise<RaidSignupSheet[]> {
  const { data: sheetData, error: sheetError } = await supabase
    .from('raid_signup_sheets')
    .select(
      'sheet_id, event_id, title, run_type, starts_at, dps_spots, support_spots, any_spots, experienced_minimum, created_at, updated_at'
    )
    .order('starts_at', { ascending: true, nullsFirst: false })
    .limit(100);

  if (sheetError) {
    throw new Error(sheetError.message);
  }

  const sheets = (sheetData || []) as RaidSignupSheetRow[];
  if (sheets.length === 0) return [];

  const sheetIds = sheets.map((sheet) => sheet.sheet_id);
  const [
    { data: raidData, error: raidError },
    { data: entryData, error: entryError }
  ] = await Promise.all([
    supabase
      .from('raid_signup_raids')
      .select('sheet_id, raid_id, raid_name, spots, dps_spots, support_spots, sort_order')
      .in('sheet_id', sheetIds)
      .order('sort_order', { ascending: true }),
    supabase
      .from('raid_signup_entries')
      .select('sheet_id, discord_id, display_name, role, status')
      .in('sheet_id', sheetIds)
      .order('created_at', { ascending: true })
  ]);

  if (raidError) {
    throw new Error(raidError.message);
  }
  if (entryError) {
    throw new Error(entryError.message);
  }

  const raidsBySheetId = new Map<string, RaidSignupRaidRow[]>();
  for (const raid of (raidData || []) as RaidSignupRaidRow[]) {
    const existing = raidsBySheetId.get(raid.sheet_id) || [];
    existing.push(raid);
    raidsBySheetId.set(raid.sheet_id, existing);
  }
  const entriesBySheetId = new Map<string, RaidSignupEntryRow[]>();
  for (const entry of (entryData || []) as RaidSignupEntryRow[]) {
    const existing = entriesBySheetId.get(entry.sheet_id) || [];
    existing.push(entry);
    entriesBySheetId.set(entry.sheet_id, existing);
  }

  return sheets.map((sheet) => {
    const raids = raidsBySheetId.get(sheet.sheet_id) || [];
    const fixedRaidIds = raids
      .filter((raid) => getRaidSignupRaid(raid.raid_id))
      .map((raid) => raid.raid_id);
    const customRaids: RaidSignupCustomRaid[] = raids
      .filter((raid) => !getRaidSignupRaid(raid.raid_id))
      .map((raid) => ({
        id: raid.raid_id,
        name: raid.raid_name,
        spots: raid.spots,
        dpsSpots: raid.dps_spots,
        supportSpots: raid.support_spots,
        custom: true
      }));

    return {
      id: sheet.sheet_id,
      eventId: sheet.event_id || sheet.sheet_id,
      title: sheet.title,
      runType: (sheet.run_type === 'raid-night' ? 'raid-night' : sheet.run_type === 'reclear' ? 'reclear' : 'learning'),
      raidIds: fixedRaidIds,
      customRaids,
      startsAt: toDiscordTimestamp(sheet.starts_at),
      dpsSpots: sheet.dps_spots || 0,
      supportSpots: sheet.support_spots || 0,
      anySpots: sheet.any_spots || 0,
      experiencedRequired: sheet.experienced_minimum || 0,
      note: '',
      preRegisteredMembers: (entriesBySheetId.get(sheet.sheet_id) || []).map((entry) => ({
        discordId: entry.discord_id,
        displayName: entry.display_name || entry.discord_id,
        role: entry.role,
        status: entry.status
      })),
      createdAt: new Date(sheet.created_at).getTime(),
      updatedAt: new Date(sheet.updated_at).getTime()
    };
  });
}

export async function updateRaidSignupSheetStart(sheetId: string, startsAtLocal: string): Promise<void> {
  const startsAt = startsAtLocal ? new Date(startsAtLocal).toISOString() : null;
  const { error } = await supabase
    .from('raid_signup_sheets')
    .update({ starts_at: startsAt })
    .eq('sheet_id', sheetId);

  if (error) {
    throw new Error(error.message);
  }
}

export async function removeRaidSignupEntry(sheetId: string, discordId: string): Promise<void> {
  const { error } = await supabase
    .from('raid_signup_entries')
    .delete()
    .eq('sheet_id', sheetId)
    .eq('discord_id', discordId);

  if (error) {
    throw new Error(error.message);
  }
}

export async function publishRaidSignupSheet(sheet: RaidSignupSheet): Promise<string> {
  const { data, error } = await supabase
    .from('raid_signup_sheets')
    .insert({
      event_id: sheet.eventId,
      title: sheet.title,
      run_type: sheet.runType,
      starts_at: parseDiscordTimestampToIso(sheet.startsAt),
      dps_spots: sheet.dpsSpots,
      support_spots: sheet.supportSpots,
      any_spots: sheet.anySpots,
      experienced_minimum: sheet.experiencedRequired,
      status: 'published'
    })
    .select('sheet_id')
    .single();

  if (error) {
    throw new Error(error.message);
  }

  const selectedRaids = [
    ...sheet.raidIds
      .map((raidId) => getRaidSignupRaid(raidId))
      .filter(isRaidSignupSheetRaid)
      .map((raid) => ({
        raid_id: raid.id,
        raid_name: raid.name,
        spots: raid.spots,
        dps_spots: raid.dpsSpots,
        support_spots: raid.supportSpots
      })),
    ...sheet.customRaids.map((raid) => ({
      raid_id: raid.id,
      raid_name: raid.name,
      spots: raid.spots,
      dps_spots: raid.dpsSpots,
      support_spots: raid.supportSpots
    }))
  ];

  if (selectedRaids.length > 0) {
    const { error: raidError } = await supabase
      .from('raid_signup_raids')
      .insert(
        selectedRaids.map((raid, index) => ({
          sheet_id: data.sheet_id,
          ...raid,
          sort_order: index
        }))
      );

    if (raidError) {
      throw new Error(raidError.message);
    }
  }

  if (sheet.preRegisteredMembers.length > 0) {
    const { error: entriesError } = await supabase
      .from('raid_signup_entries')
      .insert(
        sheet.preRegisteredMembers.map((member) => ({
          sheet_id: data.sheet_id,
          discord_id: member.discordId,
          display_name: member.displayName || member.discordId,
          role: member.role,
          status: member.status
        }))
      );

    if (entriesError) {
      throw new Error(entriesError.message);
    }
  }

  return data.sheet_id;
}

export async function updateRaidSignupSheet(sheet: RaidSignupSheet): Promise<void> {
  const { error } = await supabase
    .from('raid_signup_sheets')
    .update({
      event_id: sheet.eventId,
      title: sheet.title,
      run_type: sheet.runType,
      starts_at: parseDiscordTimestampToIso(sheet.startsAt),
      dps_spots: sheet.dpsSpots,
      support_spots: sheet.supportSpots,
      any_spots: sheet.anySpots,
      experienced_minimum: sheet.experiencedRequired,
      status: 'published'
    })
    .eq('sheet_id', sheet.id);

  if (error) {
    throw new Error(error.message);
  }

  const selectedRaids = [
    ...sheet.raidIds
      .map((raidId) => getRaidSignupRaid(raidId))
      .filter(isRaidSignupSheetRaid)
      .map((raid) => ({
        raid_id: raid.id,
        raid_name: raid.name,
        spots: raid.spots,
        dps_spots: raid.dpsSpots,
        support_spots: raid.supportSpots
      })),
    ...sheet.customRaids.map((raid) => ({
      raid_id: raid.id,
      raid_name: raid.name,
      spots: raid.spots,
      dps_spots: raid.dpsSpots,
      support_spots: raid.supportSpots
    }))
  ];

  const { error: deleteRaidsError } = await supabase
    .from('raid_signup_raids')
    .delete()
    .eq('sheet_id', sheet.id);

  if (deleteRaidsError) {
    throw new Error(deleteRaidsError.message);
  }

  if (selectedRaids.length > 0) {
    const { error: raidError } = await supabase
      .from('raid_signup_raids')
      .insert(
        selectedRaids.map((raid, index) => ({
          sheet_id: sheet.id,
          ...raid,
          sort_order: index
        }))
      );

    if (raidError) {
      throw new Error(raidError.message);
    }
  }

  for (const member of sheet.preRegisteredMembers || []) {
    const { error: entryError } = await supabase
      .from('raid_signup_entries')
      .upsert(
        {
          sheet_id: sheet.id,
          discord_id: member.discordId,
          display_name: member.displayName || member.discordId,
          role: member.role,
          status: member.status
        },
        { onConflict: 'sheet_id,discord_id' }
      );

    if (entryError) {
      throw new Error(entryError.message);
    }
  }
}
