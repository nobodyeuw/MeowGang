import { invoke } from '@tauri-apps/api/core';

export interface PartyPlanMember {
  id: string;
  name: string;
  type: 'owner' | 'friend';
  testRosterId?: string;
  color?: string;
}

export interface PartyPlanCharacter {
  charId: number;
  discordId: string;
  rosterId: string;
  rosterName: string;
  charName: string;
  classId: string;
  iconId?: string;
  itemLevel: number;
  combatPower: number;
  included: boolean;
  displayOrder: number;
}

export interface PartyPlanRaid {
  raidId: string;
  raidName: string;
  minIlvl: number;
  maxIlvl: number;
  enabled: boolean;
}

export interface PartyPlanAssignment {
  raidId: string;
  assignmentType: 'member' | 'character' | 'static';
  targetId: string;
  slotOrder: number;
}

export interface PartyPlanCompletionSnapshot {
  discordId: string;
  rosterId: string;
  charId: number;
  charName: string;
  contentId: string;
  difficulty?: string;
  isCompleted: boolean;
  sessionId?: string;
  completedAt: number;
  resetCycle: string;
  updatedAt: string;
}

export interface PartyPlanEncounterSnapshot {
  discordId: string;
  localPlayer: string;
  contentId: string;
  raidName: string;
  difficulty: string;
  gate?: string;
  cleared: boolean;
  fightStart: number;
  players: string[];
  matchedCharacterIds: number[];
  resetCycle: string;
  updatedAt: string;
}

export interface PartyPlanData {
  groupId: string;
  groupSecret: string;
  groupName: string;
  groupMode?: 'group' | 'static';
  ownerDiscordId?: string;
  sheetUrl: string;
  sheetVersion: number;
  members: PartyPlanMember[];
  characters: PartyPlanCharacter[];
  plannedRaids: PartyPlanRaid[];
  assignments: PartyPlanAssignment[];
  completionSnapshots: PartyPlanCompletionSnapshot[];
  encounterSnapshots: PartyPlanEncounterSnapshot[];
  createdAt: string;
  updatedAt: string;
}

export interface PartyPlanMemberClear {
  groupId: string;
  groupName: string;
  groupMode: 'group' | 'static';
  discordId: string;
  charId: number;
  contentId: string;
  difficulty?: string;
  sessionId?: string;
  gate?: string;
  resetCycle: string;
}

export interface PartyPlanStaticReservation {
  groupId: string;
  groupName: string;
  discordId: string;
  charId: number;
  raidId: string;
  raidName: string;
}

export const PARTY_PLAN_SHEET_NAMES = [
  'Groups',
  'Members',
  'Characters',
  'PlannedRaids',
  'Assignments',
  'CompletionSnapshots',
  'EncounterSnapshots'
] as const;

export type PartyPlanSheetName = typeof PARTY_PLAN_SHEET_NAMES[number];

export type PartyPlanSheetTables = Record<PartyPlanSheetName, string[][]>;

export const PARTY_PLAN_SHEET_HEADERS: Record<PartyPlanSheetName, string[]> = {
  Groups: [
    'group_id',
    'group_secret',
    'group_name',
    'owner_discord_id',
    'sheet_url',
    'sheet_version',
    'created_at',
    'updated_at',
    'group_mode',
    'group_config_updated_at',
    'roster_updated_at',
    'assignment_updated_at',
    'snapshot_updated_at'
  ],
  Members: [
    'group_id',
    'discord_id',
    'name',
    'type',
    'test_roster_id',
    'color'
  ],
  Characters: [
    'char_id',
    'discord_id',
    'roster_id',
    'roster_name',
    'char_name',
    'class_id',
    'icon_id',
    'item_level',
    'combat_power',
    'included_group_ids',
    'display_order'
  ],
  PlannedRaids: [
    'group_id',
    'raid_id',
    'raid_name',
    'min_ilvl',
    'max_ilvl',
    'enabled'
  ],
  Assignments: [
    'group_id',
    'raid_id',
    'assignment_type',
    'target_id',
    'slot_order'
  ],
  CompletionSnapshots: [
    'group_id',
    'discord_id',
    'roster_id',
    'char_id',
    'char_name',
    'content_id',
    'difficulty',
    'is_completed',
    'session_id',
    'completed_at',
    'reset_cycle',
    'updated_at'
  ],
  EncounterSnapshots: [
    'group_id',
    'discord_id',
    'local_player',
    'content_id',
    'raid_name',
    'difficulty',
    'gate',
    'cleared',
    'fight_start',
    'players_json',
    'matched_character_ids_json',
    'reset_cycle',
    'updated_at'
  ]
};

function boolCell(value: boolean): string {
  return value ? '1' : '0';
}

function parseBoolCell(value: string | undefined): boolean {
  const normalized = (value ?? '').trim().toLowerCase();
  return normalized === '1' || normalized === 'true' || normalized === 'yes';
}

function numberCell(value: number): string {
  return Number.isFinite(value) ? String(value) : '0';
}

function parseNumberCell(value: string | undefined): number {
  const parsed = Number(value ?? 0);
  return Number.isFinite(parsed) ? parsed : 0;
}

function optionalCell(value: string | undefined): string {
  return value ?? '';
}

function parseJsonCell<T>(value: string | undefined, fallback: T): T {
  if (!value?.trim()) return fallback;

  try {
    return JSON.parse(value) as T;
  } catch {
    return fallback;
  }
}

function parseGroupIdsCell(value: string | undefined): Set<string> {
  const normalized = (value ?? '').trim();
  if (!normalized || normalized.toLowerCase() === 'null') return new Set();
  return new Set(normalized.split(',').map((entry) => entry.trim()).filter(Boolean));
}

function rowsWithoutHeader(rows: string[][] | undefined): string[][] {
  return rows?.slice(1).filter((row) => row.some((cell) => cell.trim() !== '')) ?? [];
}

function rowToObject(headers: string[], row: string[]): Record<string, string> {
  return Object.fromEntries(headers.map((header, index) => [header, row[index] ?? '']));
}

export function createEmptyPartyPlanSheetTables(): PartyPlanSheetTables {
  return Object.fromEntries(
    PARTY_PLAN_SHEET_NAMES.map((name) => [name, [PARTY_PLAN_SHEET_HEADERS[name]]])
  ) as PartyPlanSheetTables;
}

export function partyPlanToSheetTables(plan: PartyPlanData): PartyPlanSheetTables {
  const tables = createEmptyPartyPlanSheetTables();

  tables.Groups.push([
    plan.groupId,
    plan.groupSecret,
    plan.groupName,
    optionalCell(plan.ownerDiscordId),
    plan.sheetUrl,
    numberCell(plan.sheetVersion),
    plan.createdAt,
    plan.updatedAt,
    plan.groupMode ?? 'group',
    plan.updatedAt,
    plan.updatedAt,
    plan.updatedAt,
    plan.updatedAt
  ]);

  tables.Members.push(...plan.members.map((member) => [
    plan.groupId,
    member.id,
    member.name,
    member.type,
    optionalCell(member.testRosterId),
    optionalCell(member.color)
  ]));

  tables.Characters.push(...plan.characters.map((character) => [
    numberCell(character.charId),
    character.discordId,
    character.rosterId,
    character.rosterName,
    character.charName,
    character.classId,
    optionalCell(character.iconId),
    numberCell(character.itemLevel),
    numberCell(character.combatPower),
    character.included ? plan.groupId : 'NULL',
    numberCell(character.displayOrder)
  ]));

  tables.PlannedRaids.push(...plan.plannedRaids.map((raid) => [
    plan.groupId,
    raid.raidId,
    raid.raidName,
    numberCell(raid.minIlvl),
    numberCell(raid.maxIlvl),
    boolCell(raid.enabled)
  ]));

  tables.Assignments.push(...plan.assignments.map((assignment) => [
    plan.groupId,
    assignment.raidId,
    assignment.assignmentType,
    assignment.targetId,
    numberCell(assignment.slotOrder)
  ]));

  tables.CompletionSnapshots.push(...plan.completionSnapshots.map((snapshot) => [
    plan.groupId,
    snapshot.discordId,
    snapshot.rosterId,
    numberCell(snapshot.charId),
    snapshot.charName,
    snapshot.contentId,
    optionalCell(snapshot.difficulty),
    boolCell(snapshot.isCompleted),
    optionalCell(snapshot.sessionId),
    numberCell(snapshot.completedAt),
    snapshot.resetCycle,
    snapshot.updatedAt
  ]));

  tables.EncounterSnapshots.push(...plan.encounterSnapshots.map((snapshot) => [
    plan.groupId,
    snapshot.discordId,
    snapshot.localPlayer,
    snapshot.contentId,
    snapshot.raidName,
    snapshot.difficulty,
    optionalCell(snapshot.gate),
    boolCell(snapshot.cleared),
    numberCell(snapshot.fightStart),
    JSON.stringify(snapshot.players),
    JSON.stringify(snapshot.matchedCharacterIds),
    snapshot.resetCycle,
    snapshot.updatedAt
  ]));

  return tables;
}

export function partyPlanFromSheetTables(tables: Partial<PartyPlanSheetTables>): PartyPlanData | null {
  const groupRow = rowsWithoutHeader(tables.Groups)[0];
  if (!groupRow) return null;

  const group = rowToObject(PARTY_PLAN_SHEET_HEADERS.Groups, groupRow);
  const groupId = group.group_id;
  if (!groupId) return null;

  return {
    groupId,
    groupSecret: group.group_secret,
    groupName: group.group_name || 'Imported group',
    groupMode: group.group_mode === 'static' ? 'static' : 'group',
    ownerDiscordId: group.owner_discord_id || undefined,
    sheetUrl: group.sheet_url || '',
    sheetVersion: parseNumberCell(group.sheet_version) || 1,
    members: rowsWithoutHeader(tables.Members)
      .map((row) => rowToObject(PARTY_PLAN_SHEET_HEADERS.Members, row))
      .filter((row) => row.group_id === groupId)
      .map((row) => ({
        id: row.discord_id,
        name: row.name,
        type: row.type === 'friend' ? 'friend' : 'owner',
        testRosterId: row.test_roster_id || undefined,
        color: row.color || undefined
      })),
    characters: rowsWithoutHeader(tables.Characters)
      .map((row) => rowToObject(PARTY_PLAN_SHEET_HEADERS.Characters, row))
      .map((row) => ({
        charId: parseNumberCell(row.char_id),
        discordId: row.discord_id,
        rosterId: row.roster_id,
        rosterName: row.roster_name,
        charName: row.char_name,
        classId: row.class_id,
        iconId: row.icon_id || undefined,
        itemLevel: parseNumberCell(row.item_level),
        combatPower: parseNumberCell(row.combat_power),
        included: parseGroupIdsCell(row.included_group_ids).has(groupId),
        displayOrder: parseNumberCell(row.display_order)
      }))
      .filter((character) => character.included),
    plannedRaids: rowsWithoutHeader(tables.PlannedRaids)
      .map((row) => rowToObject(PARTY_PLAN_SHEET_HEADERS.PlannedRaids, row))
      .filter((row) => row.group_id === groupId)
      .map((row) => ({
        raidId: row.raid_id,
        raidName: row.raid_name,
        minIlvl: parseNumberCell(row.min_ilvl),
        maxIlvl: parseNumberCell(row.max_ilvl),
        enabled: parseBoolCell(row.enabled)
      })),
    assignments: rowsWithoutHeader(tables.Assignments)
      .map((row) => rowToObject(PARTY_PLAN_SHEET_HEADERS.Assignments, row))
      .filter((row) => row.group_id === groupId)
      .map((row) => ({
        raidId: row.raid_id,
        assignmentType: row.assignment_type === 'member'
          ? 'member'
          : row.assignment_type === 'static'
            ? 'static'
            : 'character',
        targetId: row.target_id,
        slotOrder: parseNumberCell(row.slot_order)
      })),
    completionSnapshots: rowsWithoutHeader(tables.CompletionSnapshots)
      .map((row) => rowToObject(PARTY_PLAN_SHEET_HEADERS.CompletionSnapshots, row))
      .filter((row) => row.group_id === groupId)
      .map((row) => ({
        discordId: row.discord_id,
        rosterId: row.roster_id,
        charId: parseNumberCell(row.char_id),
        charName: row.char_name,
        contentId: row.content_id,
        difficulty: row.difficulty || undefined,
        isCompleted: parseBoolCell(row.is_completed),
        sessionId: row.session_id || undefined,
        completedAt: parseNumberCell(row.completed_at),
        resetCycle: row.reset_cycle,
        updatedAt: row.updated_at
      })),
    encounterSnapshots: rowsWithoutHeader(tables.EncounterSnapshots)
      .map((row) => rowToObject(PARTY_PLAN_SHEET_HEADERS.EncounterSnapshots, row))
      .filter((row) => row.group_id === groupId)
      .map((row) => ({
        discordId: row.discord_id,
        localPlayer: row.local_player,
        contentId: row.content_id,
        raidName: row.raid_name,
        difficulty: row.difficulty,
        gate: row.gate || undefined,
        cleared: parseBoolCell(row.cleared),
        fightStart: parseNumberCell(row.fight_start),
        players: parseJsonCell<string[]>(row.players_json, []),
        matchedCharacterIds: parseJsonCell<number[]>(row.matched_character_ids_json, []),
        resetCycle: row.reset_cycle,
        updatedAt: row.updated_at
      })),
    createdAt: group.created_at,
    updatedAt: group.updated_at
  };
}

export interface PartyPlanRemoteSyncConfig {
  endpointUrl: string;
  groupId: string;
  groupSecret: string;
}

interface PartyPlanRemoteSyncResponse {
  ok: boolean;
  plan?: PartyPlanData | null;
  status?: PartyPlanRemoteStatus | null;
  memberClears?: PartyPlanMemberClear[];
  staticReservations?: PartyPlanStaticReservation[];
  message: string;
  error?: string;
}

export interface PartyPlanRemoteStatus {
  groupId: string;
  updatedAt: string;
  sheetVersion: number;
  groupConfigUpdatedAt?: string;
  rosterUpdatedAt?: string;
  assignmentUpdatedAt?: string;
  snapshotUpdatedAt?: string;
  relatedMembersUpdatedAt?: string;
}

async function syncPartyPlanRemote(
  action: 'load' | 'status' | 'loadMemberClears' | 'loadStaticReservations' | 'save' | 'saveMerged' | 'saveSnapshots' | 'delete',
  config: PartyPlanRemoteSyncConfig,
  plan?: PartyPlanData,
  mergeOwnerIds: string[] = [],
  memberDiscordId = ''
) {
  return invoke<PartyPlanRemoteSyncResponse>('sync_party_plan_remote', {
    request: {
      endpointUrl: config.endpointUrl,
      action,
      groupId: config.groupId,
      groupSecret: config.groupSecret,
      plan,
      mergeOwnerIds,
      memberDiscordId
    }
  });
}

export async function loadPartyPlanFromSheet(config: PartyPlanRemoteSyncConfig): Promise<PartyPlanData | null> {
  const response = await syncPartyPlanRemote('load', config);
  return response.plan ?? null;
}

export async function loadPartyPlanStatusFromSheet(config: PartyPlanRemoteSyncConfig): Promise<PartyPlanRemoteStatus | null> {
  const response = await syncPartyPlanRemote('status', config);
  return response.status ?? null;
}

export async function loadPartyPlanMemberClearsFromSheet(
  config: PartyPlanRemoteSyncConfig,
  memberDiscordId: string
): Promise<PartyPlanMemberClear[]> {
  const response = await syncPartyPlanRemote('loadMemberClears', config, undefined, [], memberDiscordId);
  return response.memberClears ?? [];
}

export async function loadPartyPlanStaticReservationsFromSheet(
  config: PartyPlanRemoteSyncConfig,
  memberDiscordId: string
): Promise<PartyPlanStaticReservation[]> {
  const response = await syncPartyPlanRemote('loadStaticReservations', config, undefined, [], memberDiscordId);
  return response.staticReservations ?? [];
}

export async function savePartyPlanToSheet(plan: PartyPlanData, config: PartyPlanRemoteSyncConfig): Promise<PartyPlanData> {
  partyPlanToSheetTables(plan);
  const response = await syncPartyPlanRemote('save', config, plan);
  return response.plan ?? plan;
}

export async function saveMergedPartyPlanToSheet(
  plan: PartyPlanData,
  config: PartyPlanRemoteSyncConfig,
  mergeOwnerIds: string[]
): Promise<PartyPlanData> {
  const response = await syncPartyPlanRemote('saveMerged', config, plan, mergeOwnerIds);
  return response.plan ?? plan;
}

export async function savePartyPlanSnapshotsToSheet(plan: PartyPlanData, config: PartyPlanRemoteSyncConfig): Promise<PartyPlanData> {
  const response = await syncPartyPlanRemote('saveSnapshots', config, plan);
  return response.plan ?? plan;
}

export async function deletePartyPlanFromSheet(config: PartyPlanRemoteSyncConfig): Promise<boolean> {
  await syncPartyPlanRemote('delete', config);
  return true;
}

export async function saveLocalPartyPlan(plan: PartyPlanData): Promise<PartyPlanData> {
  return invoke<PartyPlanData>('save_party_plan', { plan });
}

export async function loadLocalPartyPlan(groupId: string): Promise<PartyPlanData | null> {
  return invoke<PartyPlanData | null>('load_party_plan', { groupId });
}

export async function listLocalPartyPlans(): Promise<PartyPlanData[]> {
  return invoke<PartyPlanData[]>('list_party_plans');
}

export async function deleteLocalPartyPlan(groupId: string): Promise<boolean> {
  return invoke<boolean>('delete_party_plan', { groupId });
}

export function extractPartyPlanGroupId(url: string): string | null {
  const value = url.trim();
  if (!value) return null;

  const rangeMatch = value.match(/[#&?]range=([^&]+)/i);
  if (rangeMatch?.[1]) {
    return decodeURIComponent(rangeMatch[1]);
  }

  const groupMatch = value.match(/[#&?]group=([^&]+)/i);
  if (groupMatch?.[1]) {
    return decodeURIComponent(groupMatch[1]);
  }

  return null;
}

export function extractPartyPlanGroupSecret(url: string): string | null {
  const value = url.trim();
  if (!value) return null;

  const secretMatch = value.match(/[#&?]secret=([^&]+)/i);
  if (secretMatch?.[1]) {
    return decodeURIComponent(secretMatch[1]);
  }

  return null;
}

export function extractPartyPlanSpreadsheetId(url: string): string | null {
  const value = url.trim();
  if (!value) return null;

  const spreadsheetMatch = value.match(/\/spreadsheets\/d\/([^/]+)/i);
  if (spreadsheetMatch?.[1]) {
    return decodeURIComponent(spreadsheetMatch[1]);
  }

  return null;
}

export function buildPartyPlanInviteUrl(sheetUrl: string, groupId: string, groupSecret: string): string {
  const baseUrl = sheetUrl.split('#')[0] || sheetUrl;
  const params = new URLSearchParams({
    group: groupId,
    secret: groupSecret
  });

  return `${baseUrl}#${params.toString()}`;
}
