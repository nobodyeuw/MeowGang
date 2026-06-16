// Raid Management is the MeowGang-only Discord signup planning model.
// The Tauri app owns planning/configuration; the Discord bot will later publish
// these sheets to Discord and handle button/select-menu signups.

export type RaidManagementRunType = 'learning' | 'reclear' | 'raid-train';
export type RaidSignupRole = 'dps' | 'support' | 'any' | 'fixed';

export interface RaidManagementAccessMember {
  discordId: string;
  displayName: string;
  grantedAt: number;
}

export interface RaidSignupSheetRaid {
  id: string;
  name: string;
  spots: number;
  dpsSpots: number;
  supportSpots: number;
  custom?: boolean;
}

export interface RaidSignupCustomRaid extends RaidSignupSheetRaid {
  custom: true;
}

export interface RaidSignupSheet {
  id: string;
  eventId: string;
  title: string;
  runType: RaidManagementRunType;
  raidIds: string[];
  customRaids: RaidSignupCustomRaid[];
  startsAt: string;
  dpsSpots: number;
  supportSpots: number;
  anySpots: number;
  experiencedRequired: number;
  note: string;
  preRegisteredMembers: RaidSignupPreRegisteredMember[];
  createdAt: number;
  updatedAt: number;
}

export type RaidSignupPreRegisteredStatus = 'learner' | 'experienced' | 'can_help' | 'leader';

export interface RaidSignupPreRegisteredMember {
  discordId: string;
  displayName: string;
  role: RaidSignupRole;
  status: RaidSignupPreRegisteredStatus;
  raidSections?: string[];
}

export interface RaidManagementRequest {
  id: string;
  title: string;
  requester: string;
  discordId: string;
  raidNames: string[];
  category: string;
  status: 'open' | 'accepted' | 'declined' | 'closed';
  decidedBy: string;
  reviewNote: string;
  createdAt: number;
  dateWindow: string;
  timeWindow: string;
  canDoSidereals: boolean;
  details: string;
}

export interface RaidSignupRoleConfig {
  role: RaidSignupRole;
  label: string;
  availableFor: RaidManagementRunType[];
}

export const RAID_SIGNUP_ROLES: RaidSignupRoleConfig[] = [
  { role: 'dps', label: 'DPS', availableFor: ['learning', 'reclear', 'raid-train'] },
  { role: 'support', label: 'SUP', availableFor: ['learning', 'reclear', 'raid-train'] },
  { role: 'any', label: 'ANY', availableFor: ['learning', 'reclear', 'raid-train'] }
];

export const RAID_SIGNUP_RAIDS: RaidSignupSheetRaid[] = [
  { id: 'echidna', name: 'Echidna', spots: 8, dpsSpots: 6, supportSpots: 2 },
  { id: 'behemoth', name: 'Behemoth', spots: 16, dpsSpots: 12, supportSpots: 4 },
  { id: 'aegir', name: 'Aegir', spots: 8, dpsSpots: 6, supportSpots: 2 },
  { id: 'brelshaza', name: 'Brelshaza', spots: 8, dpsSpots: 6, supportSpots: 2 },
  { id: 'mordum', name: 'Mordum', spots: 8, dpsSpots: 6, supportSpots: 2 },
  { id: 'armoche', name: 'Armoche', spots: 8, dpsSpots: 6, supportSpots: 2 },
  { id: 'kazeros', name: 'Kazeros', spots: 8, dpsSpots: 6, supportSpots: 2 },
  { id: 'serca', name: 'Serca', spots: 4, dpsSpots: 3, supportSpots: 1 },
  { id: 'cathedral', name: 'Cathedral', spots: 4, dpsSpots: 3, supportSpots: 1 }
];

// Add your Discord id here if you want the app to ship with one permanent
// bootstrap admin. Until the bot/backend exists, extra access grants are local.
export const RAID_MANAGEMENT_BOOTSTRAP_ADMIN_DISCORD_IDS: string[] = ['330010523863220225'];

export function getRaidSignupRaid(id: string): RaidSignupSheetRaid | undefined {
  return RAID_SIGNUP_RAIDS.find((raid) => raid.id === id);
}

// Raid parties are built around 4-player groups. Each party needs one support,
// so an 8-player raid defaults to 6 DPS + 2 SUP and a 16-player raid to 12 + 4.
export function buildRaidSignupComposition(spots: number) {
  const normalizedSpots = Math.max(1, Math.floor(spots));
  const supportSpots = Math.max(1, Math.ceil(normalizedSpots / 4));
  return {
    spots: normalizedSpots,
    dpsSpots: Math.max(0, normalizedSpots - supportSpots),
    supportSpots
  };
}

export function getRaidSignupSelectedRaids(
  raidIds: string[],
  customRaids: RaidSignupCustomRaid[] = []
): RaidSignupSheetRaid[] {
  return [
    ...raidIds.map((raidId) => getRaidSignupRaid(raidId)).filter(Boolean),
    ...customRaids
  ] as RaidSignupSheetRaid[];
}

export function getRaidSignupTotalSpots(raidIds: string[], customRaids: RaidSignupCustomRaid[] = []): number {
  return getRaidSignupSelectedRaids(raidIds, customRaids)
    .map((raid) => raid.spots)
    .reduce((total, spots) => total + spots, 0);
}

export function getRaidSignupTotalDpsSpots(raidIds: string[], customRaids: RaidSignupCustomRaid[] = []): number {
  return getRaidSignupSelectedRaids(raidIds, customRaids)
    .map((raid) => raid.dpsSpots)
    .reduce((total, spots) => total + spots, 0);
}

export function getRaidSignupTotalSupportSpots(raidIds: string[], customRaids: RaidSignupCustomRaid[] = []): number {
  return getRaidSignupSelectedRaids(raidIds, customRaids)
    .map((raid) => raid.supportSpots)
    .reduce((total, spots) => total + spots, 0);
}
