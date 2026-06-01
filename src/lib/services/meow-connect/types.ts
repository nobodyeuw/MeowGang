import type { Raid } from '$lib/data/raids';

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

export interface MeowConnectPendingRequests {
  friendRequests: MeowConnectFriendConnection[];
  groupInvites: MeowConnectGroup[];
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
