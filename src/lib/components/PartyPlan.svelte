<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { characters, rosters, type Character } from '$lib/store';
  import { GAME_CLASSES } from '$lib/data/classes';
  import { RAIDS } from '$lib/data/raids';
  import { encounterMap } from '$lib/data/encounters';
  import type {
    CharacterRaidConfig,
    CompletionStatusEntry,
    DashboardSnapshot,
    DiscordAuthResult,
    EncounterPreview,
    FriendOption,
    PlannedMember,
    RaidLane
  } from './party-plan/types';
  import './party-plan/party-plan.css';
  import {
    buildPartyPlanInviteUrl,
    deleteLocalPartyPlan,
    deletePartyPlanFromSheet,
    extractPartyPlanGroupId,
    extractPartyPlanGroupSecret,
    extractPartyPlanSpreadsheetId,
    listLocalPartyPlans,
    loadLocalPartyPlan,
    loadPartyPlanMemberClearsFromSheet,
    loadPartyPlanFromSheet,
    loadPartyPlanStaticReservationsFromSheet,
    loadPartyPlanStatusFromSheet,
    saveLocalPartyPlan,
    saveMergedPartyPlanToSheet,
    savePartyPlanSnapshotsToSheet,
    savePartyPlanToSheet,
    type PartyPlanAssignment,
    type PartyPlanCharacter,
    type PartyPlanCompletionSnapshot,
    type PartyPlanData,
    type PartyPlanEncounterSnapshot,
    type PartyPlanMemberClear,
    type PartyPlanStaticReservation
  } from '$lib/services/party-plan';

  const PARTY_PLAN_ALLOWED_DISCORD_IDS = new Set([
    '592298453002878996',
    '222326155674386432',
    '330010523863220225'
  ]);

  let friendOptions: FriendOption[] = [];
  $: testFriendRosterIds = new Set(friendOptions.map((friend) => friend.testRosterId).filter(Boolean));

  let groupName = '';
  let groupMode: 'group' | 'static' = 'group';
  let groupId = '';
  let groupSecret = '';
  let currentOwnerDiscordId = '';
  let groupCreated = false;
  let groupMembers: PlannedMember[] = [];
  let partyPlanAuthLoaded = false;
  let currentDiscordId = '';
  let currentDiscordName = '';
  let savedPartyPlans: PartyPlanData[] = [];
  let remoteMemberClears: PartyPlanMemberClear[] = [];
  let remoteStaticReservations: PartyPlanStaticReservation[] = [];
  let joinBlockedMessage = '';
  let friendSearch = '';
  let selectedCharacterIds = new Set<number>();
  let sheetUrl = '';
  let importedSheetUrl = '';
  let dirty = false;
  let saveState: 'idle' | 'saving' | 'saved' = 'idle';
  let remoteSyncState: 'idle' | 'syncing' | 'synced' | 'error' = 'idle';
  let remoteSyncMessage = '';
  let partyNoticeMessage = '';
  let partyNoticeType: 'info' | 'success' | 'warning' | 'error' = 'info';
  let partyNoticeTimer: number | null = null;
  let partyPlanEndpointUrl = '';
  let snapshotWatchTimer: number | null = null;
  let watchedRemoteGroupId = '';
  let snapshotSyncInFlight = false;
  let lastSnapshotFingerprint = '';
  let lastCompletionWatchFingerprint = '';
  let lastRemoteSnapshotFingerprint = '';
  let lastRemotePlanUpdatedAt = '';
  let pendingRemotePlanUpdatedAt = '';
  let lastPlanUpdatedAt = '';
  let activeGroupTab: 'configuration' | 'raid-board' = 'configuration';
  let dropFeedbackMessage = '';
  let activeAssignPayload = '';
  let completionByCharacter: Record<string, CompletionStatusEntry[]> = {};
  let raidConfigsByCharacter: Record<string, CharacterRaidConfig[]> = {};
  let recentEncounters: EncounterPreview[] = [];
  let sharedCharacters: PartyPlanCharacter[] = [];
  let sharedCompletionSnapshots: PartyPlanCompletionSnapshot[] = [];
  let sharedEncounterSnapshots: PartyPlanEncounterSnapshot[] = [];
  let selectedRaidIds = new Set<string>();
  let raidAssignments: Record<string, string[]> = {};
  let collapsedMemberIds = new Set<string>();
  let entryMode: 'create' | 'join' | null = null;
  let staticRunSelections: Record<string, Set<number>> = {};
  let staticRunGroups: Record<string, number[][]> = {};
  let localRosterUploaded = false;
  let remoteUpdatePending = false;

  const defaultMemberColors = ['#ff8c42', '#38bdf8', '#a78bfa', '#34d399', '#f472b6', '#facc15'];

  onMount(async () => {
    await loadPartyPlanEndpointUrl();
    await loadCurrentDiscordAuth();
    if (!PARTY_PLAN_ALLOWED_DISCORD_IDS.has(currentDiscordId)) return;

    loadSavedPartyPlans();
    loadWhitelistMembers();
    loadCompletionSnapshots();
    loadRecentEncounters();
  });

  onDestroy(() => {
    if (snapshotWatchTimer !== null) {
      window.clearInterval(snapshotWatchTimer);
    }
    if (partyNoticeTimer !== null) {
      window.clearTimeout(partyNoticeTimer);
    }
  });

  $: canAccessPartyPlan = PARTY_PLAN_ALLOWED_DISCORD_IDS.has(currentDiscordId);

  $: orderedRosters = [...$rosters].sort((a, b) =>
    (a.roster_display_order ?? 0) - (b.roster_display_order ?? 0)
    || a.roster_name.localeCompare(b.roster_name)
  );

  $: groupCharacterList = buildGroupCharacterList($characters, sharedCharacters);
  $: selectedCharacters = groupCharacterList.filter((character) => selectedCharacterIds.has(character.char_id));

  $: filteredFriends = friendOptions
    .filter((friend) => !groupMembers.some((member) => member.id === friend.id))
    .filter((friend) => {
      const search = friendSearch.trim().toLowerCase();
      if (!search) return false;
      return friend.name.toLowerCase().startsWith(search);
    });

  $: allRaidLanes = buildRaidLanes();
  $: raidLanes = allRaidLanes
    .filter((lane) => selectedRaidIds.has(lane.id))
    .map((lane) => ({
      ...lane,
      assignments: raidAssignments[lane.id] ?? []
    }));
  $: characterGroups = buildCharacterGroups(groupMembers, groupCharacterList, orderedRosters);

  async function createGroup() {
    const name = groupName.trim();
    if (!name) return;
    if (!currentDiscordId) {
      await loadCurrentDiscordAuth();
    }

    groupId = `group-${slugify(name)}-${Date.now()}`;
    groupSecret = generateGroupSecret();
    currentOwnerDiscordId = currentDiscordId || 'self';
    groupCreated = true;
    groupMode = 'group';
    groupMembers = [getLocalMember()];
    selectedCharacterIds = new Set(
      $characters
        .filter((character) => !testFriendRosterIds.has(character.roster_id))
        .map((character) => character.char_id)
    );
    selectedRaidIds = new Set(allRaidLanes.map((lane) => lane.id));
    sheetUrl = buildPartyPlanInviteUrl(
      'https://docs.google.com/spreadsheets/d/meowgang-party-plan-template/edit',
      groupId,
      groupSecret
    );
    dirty = true;
    await loadCompletionSnapshots();
    await saveCurrentPlan();
  }

  async function loadCurrentDiscordAuth() {
    try {
      const auth = await invoke<DiscordAuthResult>('verify_stored_discord_auth');
      if (auth.approved) {
        currentDiscordId = auth.user_id ?? '';
        currentDiscordName = auth.username ?? '';
      }
    } catch (error) {
      console.warn('Failed to load current Discord auth for Party Plan:', error);
    } finally {
      partyPlanAuthLoaded = true;
    }
  }

  async function loadPartyPlanEndpointUrl() {
    try {
      const configuredEndpoint = (await invoke<string | null>('get_party_plan_endpoint_url'))?.trim() ?? '';
      if (configuredEndpoint) {
        partyPlanEndpointUrl = configuredEndpoint;
        localStorage.setItem('meowgang.partyPlan.endpointUrl', configuredEndpoint);
        return;
      }
    } catch (error) {
      console.warn('Failed to load Party Plan endpoint URL:', error);
    }

    partyPlanEndpointUrl = localStorage.getItem('meowgang.partyPlan.endpointUrl')?.trim() ?? '';
  }

  function showPartyNotice(message: string, type: 'info' | 'success' | 'warning' | 'error' = 'info') {
    if (!message) return;
    partyNoticeMessage = message;
    partyNoticeType = type;

    if (partyNoticeTimer !== null) {
      window.clearTimeout(partyNoticeTimer);
    }

    partyNoticeTimer = window.setTimeout(() => {
      partyNoticeMessage = '';
      partyNoticeTimer = null;
    }, type === 'error' ? 5000 : 3000);
  }

  function setRemoteSyncMessage(
    state: 'idle' | 'syncing' | 'synced' | 'error',
    message: string
  ) {
    remoteSyncState = state;
    remoteSyncMessage = message;
    if (message) {
      showPartyNotice(message, state === 'error' ? 'error' : state === 'synced' ? 'success' : 'info');
    }
  }

  function setDropFeedbackMessage(message: string, type: 'info' | 'warning' | 'error' = 'warning') {
    dropFeedbackMessage = message;
    if (message) {
      showPartyNotice(message, type);
    }
  }

  async function loadSavedPartyPlans() {
    try {
      savedPartyPlans = await listLocalPartyPlans();
    } catch (error) {
      console.warn('Failed to load saved Party Plans:', error);
      savedPartyPlans = [];
    }
  }

  function getLocalMember(): PlannedMember {
    return {
      id: currentDiscordId || 'self',
      name: currentDiscordName || 'You',
      type: currentDiscordId && currentDiscordId === currentOwnerDiscordId ? 'owner' : 'friend',
      color: defaultMemberColors[0]
    };
  }

  function normalizeMembersForLocalUser(members: PlannedMember[]): PlannedMember[] {
    const localId = getLocalMemberId();
    let hasLocalMember = false;
    const normalized = members.map((member, index) => {
      if (member.id === localId) {
        hasLocalMember = true;
        return {
          ...member,
          name: currentDiscordName || member.name,
          type: member.id === currentOwnerDiscordId ? 'owner' as const : 'friend' as const,
          color: member.color ?? defaultMemberColors[index % defaultMemberColors.length]
        };
      }

      return {
        ...member,
        type: member.id === currentOwnerDiscordId ? 'owner' as const : 'friend' as const,
        color: member.color ?? defaultMemberColors[index % defaultMemberColors.length]
      };
    });

    return hasLocalMember ? normalized : [...normalized, getLocalMember()];
  }

  function getLocalMemberId(): string {
    return currentDiscordId || 'self';
  }

  function isCurrentUserOwner(plan: PartyPlanData | null = null): boolean {
    const localId = getLocalMemberId();
    const planOwnerId = plan?.ownerDiscordId || plan?.members.find((member) => member.type === 'owner')?.id;
    if (plan) {
      const ownerMember = plan.members.find((member) => member.type === 'owner' || member.id === planOwnerId);
      return Boolean(
        (planOwnerId && planOwnerId === localId) ||
        plan.members.some((member) => member.id === localId && member.type === 'owner') ||
        (currentDiscordName && ownerMember?.name === currentDiscordName)
      );
    }

    const ownerId = currentOwnerDiscordId || groupMembers.find((member) => member.type === 'owner')?.id;
    const ownerMember = groupMembers.find((member) => member.type === 'owner' || member.id === ownerId);
    return Boolean(
      (ownerId && ownerId === localId) ||
      groupMembers.some((member) => member.id === localId && member.type === 'owner') ||
      (currentDiscordName && ownerMember?.name === currentDiscordName)
    );
  }

  function buildCurrentOwnerId(): string {
    return currentOwnerDiscordId || currentDiscordId || groupMembers.find((member) => member.type === 'owner')?.id || 'self';
  }

  function setGroupMode(mode: 'group' | 'static') {
    if (!isCurrentUserOwner()) return;
    groupMode = mode;
    dirty = true;
  }

  function addFriend(friend: FriendOption) {
    if (!isCurrentUserOwner()) return;
    groupMembers = [...groupMembers, {
      id: friend.id,
      name: friend.name,
      type: 'friend',
      testRosterId: friend.testRosterId,
      color: defaultMemberColors[groupMembers.length % defaultMemberColors.length]
    }];

    if (friend.testRosterId) {
      const next = new Set(selectedCharacterIds);
      for (const character of $characters) {
        if (character.roster_id === friend.testRosterId) {
          next.add(character.char_id);
        }
      }
      selectedCharacterIds = next;
    }

    friendSearch = '';
    dirty = true;
  }

  async function loadWhitelistMembers() {
    try {
      friendOptions = await invoke<FriendOption[]>('get_discord_whitelist_members');
    } catch (error) {
      console.warn('Failed to load Discord whitelist members for Party Plan:', error);
      friendOptions = [];
    }
  }

  function removeFriend(memberId: string) {
    if (!isCurrentUserOwner()) return;
    groupMembers = groupMembers.filter((member) => member.id !== memberId || member.id === currentOwnerDiscordId);
    dirty = true;
  }

  function updateMemberColor(memberId: string, color: string) {
    groupMembers = groupMembers.map((member) =>
      member.id === memberId ? { ...member, color } : member
    );
    dirty = true;
  }

  function toggleCharacter(characterId: number) {
    const character = groupCharacterList.find((entry) => entry.char_id === characterId);
    if (character && getCharacterOwner(character)?.id !== getLocalMemberId()) return;
    const next = new Set(selectedCharacterIds);
    if (next.has(characterId)) {
      next.delete(characterId);
    } else {
      next.add(characterId);
    }
    selectedCharacterIds = next;
    dirty = true;
  }

  function toggleRaidTracking(raidId: string) {
    if (!isCurrentUserOwner()) return;
    const next = new Set(selectedRaidIds);
    if (next.has(raidId)) {
      next.delete(raidId);
    } else {
      next.add(raidId);
    }
    selectedRaidIds = next;
    dirty = true;
  }

  function toggleMemberCollapsed(memberId: string) {
    const next = new Set(collapsedMemberIds);
    if (next.has(memberId)) {
      next.delete(memberId);
    } else {
      next.add(memberId);
    }
    collapsedMemberIds = next;
  }

  function getMemberCharacters(member: PlannedMember): Character[] {
    const ownedSharedCharacterIds = new Set(
      sharedCharacters
        .filter((character) => character.discordId === member.id && character.included)
        .map((character) => character.charId)
    );

    if (ownedSharedCharacterIds.size > 0) {
      return selectedCharacters.filter((character) => ownedSharedCharacterIds.has(character.char_id));
    }

    if (member.id === getLocalMemberId()) {
      return selectedCharacters.filter((character) => !testFriendRosterIds.has(character.roster_id));
    }

    if (member.testRosterId) {
      return selectedCharacters.filter((character) => character.roster_id === member.testRosterId);
    }

    return [];
  }

  function buildRaidLanes(): RaidLane[] {
    const raidByName = new Map<string, RaidLane>();

    for (const raid of RAIDS) {
      const raidMinIlvl = Math.min(...raid.gates.map((gate) => gate.minIlvl));
      const raidMaxIlvl = Math.max(...raid.gates.map((gate) => gate.minIlvl));
      const existing = raidByName.get(raid.name);

      if (existing) {
        existing.minIlvl = Math.min(existing.minIlvl, raidMinIlvl);
        existing.maxIlvl = Math.max(existing.maxIlvl, raidMaxIlvl);
        existing.raidIds = Array.from(new Set([...existing.raidIds, raid.id]));
        existing.difficulties = mergeRaidDifficulties(existing.difficulties, {
          difficulty: raid.difficulty,
          minIlvl: raidMinIlvl
        });
      } else {
        raidByName.set(raid.name, {
          id: raid.id,
          name: raid.name,
          minIlvl: raidMinIlvl,
          maxIlvl: raidMaxIlvl,
          raidIds: [raid.id],
          difficulties: [{ difficulty: raid.difficulty, minIlvl: raidMinIlvl }],
          assignments: []
        });
      }
    }

    return Array.from(raidByName.values()).sort((a, b) => b.maxIlvl - a.maxIlvl);
  }

  function mergeRaidDifficulties(
    existing: Array<{ difficulty: string; minIlvl: number }>,
    next: { difficulty: string; minIlvl: number }
  ) {
    const byDifficulty = new Map(existing.map((entry) => [entry.difficulty, entry]));
    const current = byDifficulty.get(next.difficulty);
    byDifficulty.set(next.difficulty, {
      difficulty: next.difficulty,
      minIlvl: current ? Math.min(current.minIlvl, next.minIlvl) : next.minIlvl
    });

    return Array.from(byDifficulty.values()).sort((a, b) => a.minIlvl - b.minIlvl);
  }

  async function loadCompletionSnapshots() {
    const nextCompletionByCharacter: Record<string, CompletionStatusEntry[]> = {};
    const nextRaidConfigsByCharacter: Record<string, CharacterRaidConfig[]> = {};

    await Promise.all($rosters.map(async (roster) => {
      try {
        const snapshot = await invoke<DashboardSnapshot>('get_dashboard_snapshot', {
          rosterId: roster.id
        });
        Object.assign(nextCompletionByCharacter, snapshot.completion_by_character || {});
        Object.assign(nextRaidConfigsByCharacter, snapshot.raid_configs_by_character || {});
      } catch (error) {
        console.warn(`Failed to load party plan completion state for roster ${roster.id}:`, error);
      }
    }));

    completionByCharacter = nextCompletionByCharacter;
    raidConfigsByCharacter = nextRaidConfigsByCharacter;
  }

  async function loadRecentEncounters() {
    try {
      recentEncounters = await invoke<EncounterPreview[]>('get_encounters_preview', {
        limit: 100
      });
    } catch (error) {
      console.warn('Failed to load party plan encounter player preview:', error);
      recentEncounters = [];
    }
  }

  function partyPlanCharacterToCharacter(character: PartyPlanCharacter): Character {
    return {
      char_id: character.charId,
      char_name: character.charName,
      roster_id: character.rosterId,
      roster_name: character.rosterName,
      class_id: character.classId,
      item_level: character.itemLevel,
      combat_power: character.combatPower,
      display_order: character.displayOrder,
      earns_gold: false,
      hide_from_dashboard: false,
      icon_id: character.iconId
    };
  }

  function buildGroupCharacterList(localCharacters: Character[], remoteCharacters: PartyPlanCharacter[]): Character[] {
    const localMemberId = getLocalMemberId();
    const localKeys = new Set(localCharacters.map((character) => `${localMemberId}:${character.char_id}`));
    return [
      ...localCharacters,
      ...remoteCharacters
        .filter((character) => !localKeys.has(`${character.discordId}:${character.charId}`))
        .map(partyPlanCharacterToCharacter)
    ];
  }

  function getRosterName(rosterId: string, rosterList = orderedRosters): string {
    return rosterList.find((roster) => roster.id === rosterId)?.roster_name ?? rosterId;
  }

  function groupCharactersByRoster(characterList: Character[], rosterList = orderedRosters) {
    const rosterOrder = new Map(rosterList.map((roster, index) => [roster.id, index]));
    const grouped = new Map<string, Character[]>();

    for (const character of characterList) {
      if (!grouped.has(character.roster_id)) {
        grouped.set(character.roster_id, []);
      }
      grouped.get(character.roster_id)?.push(character);
    }

    return Array.from(grouped.entries())
      .sort(([a], [b]) =>
        (rosterOrder.get(a) ?? Number.MAX_SAFE_INTEGER) - (rosterOrder.get(b) ?? Number.MAX_SAFE_INTEGER)
        || getRosterName(a, rosterList).localeCompare(getRosterName(b, rosterList))
      )
      .map(([rosterId, rosterCharacters]) => ({
        rosterId,
        rosterName: getRosterName(rosterId, rosterList),
        characters: rosterCharacters.sort((a, b) => a.display_order - b.display_order)
      }));
  }

  function buildCharacterGroups(members: PlannedMember[], characterList: Character[], rosterList = orderedRosters) {
    const groups: Array<{
      key: string;
      member: PlannedMember;
      rosterName: string;
      characters: Character[];
    }> = [];

    const localMember = members.find((member) => member.id === getLocalMemberId());
    if (localMember) {
      const localCharacters = characterList.filter((character) => getCharacterOwner(character)?.id === localMember.id);
      for (const rosterGroup of groupCharactersByRoster(localCharacters, rosterList)) {
        groups.push({
          key: `${localMember.id}-${rosterGroup.rosterId}`,
          member: localMember,
          rosterName: rosterGroup.rosterName,
          characters: rosterGroup.characters
        });
      }
    }

    for (const member of members.filter((entry) => entry.id !== getLocalMemberId())) {
      const ownedCharacters = characterList
        .filter((character) => getCharacterOwner(character)?.id === member.id)
        .sort((a, b) => a.display_order - b.display_order);

      if (ownedCharacters.length === 0 && !member.testRosterId) {
        groups.push({
          key: member.id,
          member,
          rosterName: 'Shared roster',
          characters: []
        });
        continue;
      }

      for (const rosterGroup of groupCharactersByRoster(
        ownedCharacters.length > 0
          ? ownedCharacters
          : characterList.filter((character) => character.roster_id === member.testRosterId),
        rosterList
      )) {
        groups.push({
          key: `${member.id}-${rosterGroup.rosterId}`,
          member,
          rosterName: rosterGroup.rosterName,
          characters: rosterGroup.characters
        });
      }
    }

    return groups;
  }

  function getClassIcon(character: Character): string {
    const classInfo = GAME_CLASSES[character.class_id];
    return classInfo?.iconId ?? character.icon_id ?? '0';
  }

  function formatNumber(value: number): string {
    return value.toLocaleString('de-DE', {
      minimumFractionDigits: value % 1 === 0 ? 0 : 2,
      maximumFractionDigits: 2
    });
  }

  function formatDateTime(value: string): string {
    if (!value) return 'Never';
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    return date.toLocaleString(undefined, {
      dateStyle: 'short',
      timeStyle: 'short'
    });
  }

  function slugify(value: string): string {
    return value
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '') || 'group';
  }

  function generateGroupSecret(): string {
    const bytes = new Uint8Array(24);
    crypto.getRandomValues(bytes);
    return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
  }

  function getAssignPayload(kind: 'character' | 'member', id: string | number) {
    return JSON.stringify({ kind, id: String(id) });
  }

  function selectAssignPayload(kind: 'character' | 'member', id: string | number) {
    if (!isCurrentUserOwner()) return;
    activeAssignPayload = getAssignPayload(kind, id);
    setDropFeedbackMessage('Runner selected. Choose a raid lane.', 'info');
  }

  function handleLaneAssign(lane: RaidLane) {
    if (!activeAssignPayload) return;
    assignPayloadToLane(activeAssignPayload, lane);
  }

  function assignPayloadToLane(payload: string, lane: RaidLane) {
    if (!isCurrentUserOwner()) return;
    if (!payload) {
      setDropFeedbackMessage('No runner selected. Click a runner, then assign it to a raid lane.');
      return;
    }

    try {
      const parsed = JSON.parse(payload) as { kind: 'character' | 'member'; id: string };
      const assignmentIds = getDropAssignmentIds(parsed, lane);
      if (assignmentIds.length === 0) {
        const reservedCharacter = parsed.kind === 'character'
          ? selectedCharacters.find((entry) => String(entry.char_id) === parsed.id)
          : null;
        const reservation = reservedCharacter ? getStaticReservationForLane(reservedCharacter, lane) : null;
        setDropFeedbackMessage(
          reservation
            ? `${reservedCharacter?.char_name ?? 'Runner'} is reserved for static ${reservation.groupName} ${reservation.raidName} run.`
            : `${lane.name}: no iLvl eligible selected runners for this lane.`
        );
        window.setTimeout(() => {
          dropFeedbackMessage = '';
        }, 2200);
        return;
      }

      raidAssignments = {
        ...raidAssignments,
        [lane.id]: Array.from(new Set([...(raidAssignments[lane.id] ?? []), ...assignmentIds]))
      };
      dropFeedbackMessage = '';
      activeAssignPayload = '';
      dirty = true;
    } catch (error) {
      console.warn('Invalid party plan assignment payload:', error);
      setDropFeedbackMessage('Invalid runner payload. Try selecting the runner again.');
    }
  }

  function getDropAssignmentIds(parsed: { kind: 'character' | 'member'; id: string }, lane: RaidLane): string[] {
    if (parsed.kind === 'character') {
      const character = selectedCharacters.find((entry) => String(entry.char_id) === parsed.id);
      if (!character || !canAssignCharacterToRaid(character, lane)) return [];
      return [`character:${character.char_id}`];
    }

    const member = groupMembers.find((entry) => entry.id === parsed.id);
    if (!member) return [];

    return getMemberCharacters(member)
      .filter((character) => canAssignCharacterToRaid(character, lane))
      .map((character) => `character:${character.char_id}`);
  }

  function removeAssignment(lane: RaidLane, assignmentId: string) {
    if (!isCurrentUserOwner()) return;
    raidAssignments = {
      ...raidAssignments,
      [lane.id]: (raidAssignments[lane.id] ?? []).filter((current) => current !== assignmentId)
    };
    const [, targetId] = assignmentId.split(':');
    if (targetId) {
      const removedCharId = Number(targetId);
      staticRunGroups = {
        ...staticRunGroups,
        [lane.id]: (staticRunGroups[lane.id] ?? [])
          .map((run) => run.filter((charId) => charId !== removedCharId))
          .filter((run) => run.length > 1)
      };
      const currentSelection = new Set(staticRunSelections[lane.id] ?? []);
      currentSelection.delete(removedCharId);
      staticRunSelections = { ...staticRunSelections, [lane.id]: currentSelection };
    }
    dirty = true;
  }

  function getCurrentAssignments(): PartyPlanAssignment[] {
    return allRaidLanes.flatMap((lane) => {
      const laneAssignments = raidAssignments[lane.id] ?? [];
      const runnerAssignments: PartyPlanAssignment[] = [];
      const assignableCharacterIds = new Set<number>();

      for (const assignmentId of laneAssignments) {
        const [assignmentType, targetId] = assignmentId.split(':') as ['member' | 'character', string];
        if (assignmentType === 'character') {
          const character = selectedCharacters.find((entry) => String(entry.char_id) === targetId);
          if (!character || getStaticReservationForLane(character, lane)) continue;
          assignableCharacterIds.add(character.char_id);
        }

        runnerAssignments.push({
          raidId: lane.id,
          assignmentType,
          targetId,
          slotOrder: runnerAssignments.length
        });
      }

      const staticAssignments = (staticRunGroups[lane.id] ?? [])
        .map((run) => run.filter((charId) => assignableCharacterIds.has(charId)))
        .filter((run) => run.length > 1)
        .map((run, staticIndex) => ({
          raidId: lane.id,
          assignmentType: 'static' as const,
          targetId: run.join(','),
          slotOrder: 10_000 + staticIndex
        }));

      return [...runnerAssignments, ...staticAssignments];
    });
  }

  function getWeeklyResetCycle(timestamp = Date.now()): string {
    const reset = new Date(timestamp);
    reset.setUTCHours(10, 0, 0, 0);

    const day = reset.getUTCDay();
    const daysSinceWednesday = (day + 4) % 7;
    reset.setUTCDate(reset.getUTCDate() - daysSinceWednesday);

    if (timestamp < reset.getTime()) {
      reset.setUTCDate(reset.getUTCDate() - 7);
    }

    return reset.toISOString();
  }

  function normalizeCompletionTimestamp(value?: number): number {
    if (!value || !Number.isFinite(value)) return 0;
    return value < 10_000_000_000 ? value * 1000 : value;
  }

  function buildCompletionSnapshots(updatedAt: string): PartyPlanCompletionSnapshot[] {
    const resetCycle = getWeeklyResetCycle();
    const weeklyResetMs = Date.parse(resetCycle);
    const plannedRaidIds = new Set(allRaidLanes.flatMap((lane) => lane.raidIds));

    return characterGroups.flatMap((group) =>
      group.characters.flatMap((character) => {
        if (!selectedCharacterIds.has(character.char_id)) return [];

        return (completionByCharacter[String(character.char_id)] || [])
          .filter((entry) => plannedRaidIds.has(entry.content_id))
          .filter((entry) => {
            const completedAt = normalizeCompletionTimestamp(entry.timestamp);
            return completedAt >= weeklyResetMs;
          })
          .map((entry) => ({
            discordId: group.member.id,
            rosterId: character.roster_id,
            charId: character.char_id,
            charName: character.char_name,
            contentId: entry.content_id,
            difficulty: entry.details ?? undefined,
            isCompleted: Number(entry.is_completed) === 1,
            sessionId: entry.session_id ?? undefined,
            completedAt: normalizeCompletionTimestamp(entry.timestamp),
            resetCycle,
            updatedAt
          }));
      })
    );
  }

  function normalizeEncounterTimestamp(value: number): number {
    if (!Number.isFinite(value)) return 0;
    return value < 10_000_000_000 ? value * 1000 : value;
  }

  function buildEncounterSnapshots(updatedAt: string): PartyPlanEncounterSnapshot[] {
    const resetCycle = getWeeklyResetCycle();
    const weeklyResetMs = Date.parse(resetCycle);
    const selectedCharactersByName = new Map(
      selectedCharacters.map((character) => [character.char_name.trim().toLowerCase(), character])
    );

    return recentEncounters.flatMap((encounter) => {
      const encounterRaid = getRaidMatchForEncounter(encounter.current_boss);
      const contentId = encounterRaid?.contentId ?? null;
      if (!contentId || !selectedRaidIds.has(contentId)) return [];

      const encounterTime = normalizeEncounterTimestamp(encounter.fight_start);
      if (encounterTime > 0 && encounterTime < weeklyResetMs) return [];

      const matchedCharacterIds = encounter.players
        .map((playerName) => selectedCharactersByName.get(playerName.trim().toLowerCase())?.char_id)
        .filter((charId): charId is number => typeof charId === 'number');

      if (matchedCharacterIds.length === 0) return [];

      const localCharacter = selectedCharactersByName.get(encounter.local_player.trim().toLowerCase());
      const localOwner = localCharacter ? getCharacterOwner(localCharacter) : groupMembers.find((member) => member.id === getLocalMemberId());

      return [{
        discordId: localOwner?.id ?? 'self',
        localPlayer: encounter.local_player,
        contentId,
        raidName: allRaidLanes.find((lane) => lane.id === contentId)?.name ?? encounter.current_boss,
        difficulty: encounter.difficulty,
        gate: encounterRaid?.gate,
        cleared: encounter.cleared,
        fightStart: encounter.fight_start,
        players: encounter.players,
        matchedCharacterIds,
        resetCycle,
        updatedAt
      }];
    });
  }

  function mergeSnapshotsByLocalOwners<T extends { discordId: string }>(existingSnapshots: T[], localSnapshots: T[]): T[] {
    const localOwners = new Set(localSnapshots.map((snapshot) => snapshot.discordId));
    return [
      ...existingSnapshots.filter((snapshot) => !localOwners.has(snapshot.discordId)),
      ...localSnapshots
    ];
  }

  function buildLocalSnapshotPlan(updatedAt = new Date().toISOString()): PartyPlanData {
    return {
      ...buildCurrentPlan(),
      completionSnapshots: buildCompletionSnapshots(updatedAt),
      encounterSnapshots: buildEncounterSnapshots(updatedAt),
      updatedAt
    };
  }

  function getSnapshotFingerprint(plan: PartyPlanData): string {
    return JSON.stringify({
      completionSnapshots: plan.completionSnapshots
        .map((snapshot) => ({
          discordId: snapshot.discordId,
          charId: snapshot.charId,
          contentId: snapshot.contentId,
          isCompleted: snapshot.isCompleted,
          difficulty: snapshot.difficulty ?? '',
          sessionId: snapshot.sessionId ?? '',
          completedAt: snapshot.completedAt,
          resetCycle: snapshot.resetCycle
        }))
        .sort((a, b) => `${a.discordId}:${a.charId}:${a.contentId}:${a.completedAt}`.localeCompare(`${b.discordId}:${b.charId}:${b.contentId}:${b.completedAt}`)),
      encounterSnapshots: plan.encounterSnapshots
        .map((snapshot) => ({
          discordId: snapshot.discordId,
          localPlayer: snapshot.localPlayer,
          contentId: snapshot.contentId,
          difficulty: snapshot.difficulty,
          gate: snapshot.gate ?? '',
          fightStart: snapshot.fightStart,
          players: [...snapshot.players].sort(),
          matchedCharacterIds: [...snapshot.matchedCharacterIds].sort((a, b) => a - b),
          resetCycle: snapshot.resetCycle
        }))
        .sort((a, b) => `${a.discordId}:${a.contentId}:${a.fightStart}`.localeCompare(`${b.discordId}:${b.contentId}:${b.fightStart}`))
    });
  }

  function getCompletionWatchFingerprint(): string {
    return JSON.stringify(
      buildCompletionSnapshots('watch')
        .map((snapshot) => ({
          discordId: snapshot.discordId,
          charId: snapshot.charId,
          contentId: snapshot.contentId,
          isCompleted: snapshot.isCompleted,
          difficulty: snapshot.difficulty ?? '',
          sessionId: snapshot.sessionId ?? '',
          completedAt: snapshot.completedAt,
          resetCycle: snapshot.resetCycle
        }))
        .sort((a, b) => `${a.discordId}:${a.charId}:${a.contentId}:${a.completedAt}`.localeCompare(`${b.discordId}:${b.charId}:${b.contentId}:${b.completedAt}`))
    );
  }

  function getSharedSnapshotFingerprint(
    completionSnapshots: PartyPlanCompletionSnapshot[],
    encounterSnapshots: PartyPlanEncounterSnapshot[]
  ): string {
    return JSON.stringify({
      completionSnapshots: completionSnapshots
        .map((snapshot) => ({
          discordId: snapshot.discordId,
          charId: snapshot.charId,
          contentId: snapshot.contentId,
          isCompleted: snapshot.isCompleted,
          difficulty: snapshot.difficulty ?? '',
          sessionId: snapshot.sessionId ?? '',
          completedAt: snapshot.completedAt,
          resetCycle: snapshot.resetCycle,
          updatedAt: snapshot.updatedAt
        }))
        .sort((a, b) => `${a.discordId}:${a.charId}:${a.contentId}:${a.completedAt}`.localeCompare(`${b.discordId}:${b.charId}:${b.contentId}:${b.completedAt}`)),
      encounterSnapshots: encounterSnapshots
        .map((snapshot) => ({
          discordId: snapshot.discordId,
          localPlayer: snapshot.localPlayer,
          contentId: snapshot.contentId,
          difficulty: snapshot.difficulty,
          gate: snapshot.gate ?? '',
          fightStart: snapshot.fightStart,
          players: [...snapshot.players].sort(),
          matchedCharacterIds: [...snapshot.matchedCharacterIds].sort((a, b) => a - b),
          resetCycle: snapshot.resetCycle,
          updatedAt: snapshot.updatedAt
        }))
        .sort((a, b) => `${a.discordId}:${a.contentId}:${a.fightStart}`.localeCompare(`${b.discordId}:${b.contentId}:${b.fightStart}`))
    });
  }

  function applyRemoteSnapshots(plan: PartyPlanData): boolean {
    const resetCycle = getWeeklyResetCycle();
    const currentCompletionSnapshots = (plan.completionSnapshots ?? [])
      .filter((snapshot) => snapshot.resetCycle === resetCycle);
    const currentEncounterSnapshots = (plan.encounterSnapshots ?? [])
      .filter((snapshot) => snapshot.resetCycle === resetCycle);
    const nextFingerprint = getSharedSnapshotFingerprint(
      currentCompletionSnapshots,
      currentEncounterSnapshots
    );

    if (!nextFingerprint || nextFingerprint === lastRemoteSnapshotFingerprint) return false;

    sharedCompletionSnapshots = currentCompletionSnapshots;
    sharedEncounterSnapshots = currentEncounterSnapshots;
    lastRemoteSnapshotFingerprint = nextFingerprint;
    return true;
  }

  function getCurrentResetCompletionSnapshots(snapshots: PartyPlanCompletionSnapshot[]): PartyPlanCompletionSnapshot[] {
    const resetCycle = getWeeklyResetCycle();
    return snapshots.filter((snapshot) => snapshot.resetCycle === resetCycle);
  }

  function getCurrentResetEncounterSnapshots(snapshots: PartyPlanEncounterSnapshot[]): PartyPlanEncounterSnapshot[] {
    const resetCycle = getWeeklyResetCycle();
    return snapshots.filter((snapshot) => snapshot.resetCycle === resetCycle);
  }

  function applyAssignments(assignments: PartyPlanAssignment[]) {
    const assignmentsByRaid = new Map<string, string[]>();
    const staticRunsByRaid: Record<string, number[][]> = {};
    for (const assignment of assignments) {
      if (assignment.assignmentType === 'static') {
        const charIds = assignment.targetId
          .split(',')
          .map((value) => Number(value))
          .filter((value) => Number.isFinite(value));
        if (charIds.length > 1) {
          staticRunsByRaid[assignment.raidId] = [...(staticRunsByRaid[assignment.raidId] ?? []), charIds];
        }
        continue;
      }

      const current = assignmentsByRaid.get(assignment.raidId) ?? [];
      current[assignment.slotOrder] = `${assignment.assignmentType}:${assignment.targetId}`;
      assignmentsByRaid.set(assignment.raidId, current);
    }

    raidAssignments = Object.fromEntries(
      Array.from(assignmentsByRaid.entries()).map(([raidId, laneAssignments]) => [
        raidId,
        laneAssignments.filter(Boolean)
      ])
    );
    staticRunGroups = staticRunsByRaid;
    staticRunSelections = {};
  }

  function getAssignedCharacters(lane: RaidLane): Character[] {
    return lane.assignments
      .map((assignmentId) => {
        const [kind, id] = assignmentId.split(':');
        if (kind !== 'character') return null;
        const character = selectedCharacters.find((entry) => String(entry.char_id) === id) ?? null;
        if (!character || getStaticReservationForLane(character, lane)) return null;
        return character;
      })
      .filter((character): character is Character => Boolean(character));
  }

  function getOrderedAssignedCharacters(lane: RaidLane): Character[] {
    const assignedCharacters = getAssignedCharacters(lane);
    const charactersById = new Map(assignedCharacters.map((character) => [character.char_id, character]));
    const usedCharacterIds = new Set<number>();
    const orderedCharacters: Character[] = [];

    for (const staticRun of staticRunGroups[lane.id] ?? []) {
      const connectedCharacters = staticRun
        .map((charId) => charactersById.get(charId))
        .filter((character): character is Character => Boolean(character));

      if (connectedCharacters.length < 2) continue;

      for (const character of connectedCharacters) {
        if (usedCharacterIds.has(character.char_id)) continue;
        orderedCharacters.push(character);
        usedCharacterIds.add(character.char_id);
      }
    }

    return [
      ...orderedCharacters,
      ...assignedCharacters.filter((character) => !usedCharacterIds.has(character.char_id))
    ];
  }

  function getRaidCapacity(lane: RaidLane): number {
    const normalizedName = lane.name.trim().toLowerCase();
    if (normalizedName.includes('serca')) return 4;
    if (normalizedName.includes('behemoth')) return 16;
    return 8;
  }

  function getOpenGroupRunCount(lane: RaidLane): number {
    const availableCharactersByOwner = new Map<string, number>();

    for (const character of getAssignedCharacters(lane)) {
      if (getCharacterRaidState(character, lane) !== 'available') continue;

      const ownerId = getCharacterOwner(character)?.id;
      if (!ownerId) continue;

      availableCharactersByOwner.set(ownerId, (availableCharactersByOwner.get(ownerId) ?? 0) + 1);
    }

    const ownerCounts = Array.from(availableCharactersByOwner.values()).filter((count) => count > 0);
    if (ownerCounts.length === 0) return 0;

    const raidCapacity = getRaidCapacity(lane);
    if (ownerCounts.length > raidCapacity) {
      const remainingCounts = [...ownerCounts];
      let openRuns = 0;

      while (remainingCounts.filter((count) => count > 0).length > 1) {
        remainingCounts.sort((a, b) => b - a);
        const usedOwnerCount = Math.min(raidCapacity, remainingCounts.filter((count) => count > 0).length);
        for (let index = 0; index < usedOwnerCount; index += 1) {
          remainingCounts[index] -= 1;
        }
        openRuns += 1;
      }

      return openRuns;
    }

    return Math.min(...ownerCounts);
  }

  function canEnterRaid(character: Character, lane: RaidLane): boolean {
    return character.item_level >= lane.minIlvl;
  }

  function canAssignCharacterToRaid(character: Character, lane: RaidLane): boolean {
    return selectedCharacterIds.has(character.char_id) && canEnterRaid(character, lane) && !getStaticReservationForLane(character, lane);
  }

  function getStaticReservationForLane(character: Character, lane: RaidLane): PartyPlanStaticReservation | null {
    const ownerId = getCharacterOwner(character)?.id;
    if (!ownerId) return null;

    const remoteReservation = remoteStaticReservations.find((reservation) =>
      reservation.groupId !== groupId &&
      reservation.discordId === ownerId &&
      reservation.charId === character.char_id &&
      lane.raidIds.includes(reservation.raidId)
    );
    if (remoteReservation) return remoteReservation;

    for (const plan of savedPartyPlans) {
      if (plan.groupId === groupId || plan.groupMode !== 'static') continue;
      if (!plan.members.some((member) => member.id === ownerId)) continue;
      const assignment = plan.assignments.find((entry) =>
        entry.assignmentType === 'character' &&
        entry.targetId === String(character.char_id) &&
        lane.raidIds.includes(entry.raidId)
      );
      if (!assignment) continue;

      return {
        groupId: plan.groupId,
        groupName: plan.groupName,
        discordId: ownerId,
        charId: character.char_id,
        raidId: assignment.raidId,
        raidName: plan.plannedRaids.find((raid) => raid.raidId === assignment.raidId)?.raidName ?? assignment.raidId
      };
    }

    return null;
  }

  function getStaticReservationForSelectedRaids(character: Character): PartyPlanStaticReservation | null {
    return raidLanes
      .map((lane) => getStaticReservationForLane(character, lane))
      .find((reservation): reservation is PartyPlanStaticReservation => Boolean(reservation)) ?? null;
  }

  function getAssignableRaidCountForCharacter(character: Character): number {
    return raidLanes.filter((lane) => canAssignCharacterToRaid(character, lane)).length;
  }

  function canCharacterEnterSelectedRaid(character: Character): boolean {
    return selectedCharacterIds.has(character.char_id) && raidLanes.some((lane) => canEnterRaid(character, lane));
  }

  function getStaticReservationTitle(character: Character): string {
    const reservation = getStaticReservationForSelectedRaids(character);
    return reservation ? `Reserved for static ${reservation.groupName} ${reservation.raidName} run` : '';
  }

  function normalizeDifficulty(value?: string | null): string {
    return String(value || '').trim().toLowerCase();
  }

  function normalizeGate(value?: string | null): string | null {
    const match = String(value || '').match(/gate\s*(\d+)|g\s*(\d+)/i);
    const gateNumber = match?.[1] ?? match?.[2];
    return gateNumber ? `Gate ${gateNumber}` : null;
  }

  function getSessionGate(sessionId?: string | null): string | null {
    return normalizeGate(sessionId);
  }

  function getRaidMatchForEncounter(bossName: string): { contentId: string; gate?: string } | null {
    const normalizedBossName = bossName.trim().toLowerCase();
    if (!normalizedBossName) return null;

    for (const [contentId, gateGroups] of Object.entries(encounterMap)) {
      for (const [gateLabel, bossNames] of Object.entries(gateGroups)) {
        const gate = normalizeGate(gateLabel) ?? undefined;
        if (bossNames.some((entry) => entry.trim().toLowerCase() === normalizedBossName)) {
          return { contentId, gate };
        }
      }
    }

    const matchedLane = allRaidLanes.find((lane) =>
      lane.name.trim().toLowerCase() === normalizedBossName ||
      normalizedBossName.includes(lane.name.trim().toLowerCase())
    );

    return matchedLane ? { contentId: matchedLane.id } : null;
  }

  function getExpectedRaidGates(contentId: string, difficulty?: string | null): string[] {
    const normalizedDifficulty = normalizeDifficulty(difficulty);
    const raid = RAIDS.find((entry) =>
      entry.id === contentId && (!normalizedDifficulty || normalizeDifficulty(entry.difficulty) === normalizedDifficulty)
    ) ?? RAIDS.find((entry) => entry.id === contentId);

    return raid?.gates.map((gate) => gate.gate) ?? [];
  }

  function addRaidGateProgress(
    progress: Map<string, Set<string>>,
    contentId: string,
    difficulty: string | undefined | null,
    gate: string | null
  ) {
    if (!gate) return;
    const key = `${contentId}:${normalizeDifficulty(difficulty)}`;
    const current = progress.get(key) ?? new Set<string>();
    current.add(gate);
    progress.set(key, current);
  }

  function getCharacterRaidProgress(character: Character, lane: RaidLane): {
    state: 'available' | 'pending' | 'completed';
    difficulty: string | null;
    clearedGates: string[];
    expectedGates: string[];
  } {
    const owner = getCharacterOwner(character);
    const progress = new Map<string, Set<string>>();
    const resetCycle = getWeeklyResetCycle();
    const weeklyResetMs = Date.parse(resetCycle);
    const characterName = character.char_name.trim().toLowerCase();

    for (const entry of completionByCharacter[String(character.char_id)] || []) {
      if (!lane.raidIds.includes(entry.content_id) || Number(entry.is_completed) !== 1) continue;
      const completedAt = normalizeCompletionTimestamp(entry.timestamp);
      if (completedAt > 0 && completedAt < weeklyResetMs) continue;
      addRaidGateProgress(progress, entry.content_id, entry.details, getSessionGate(entry.session_id));
    }

    for (const snapshot of sharedCompletionSnapshots) {
      if (
        snapshot.charId !== character.char_id ||
        (owner && snapshot.discordId !== owner.id) ||
        !lane.raidIds.includes(snapshot.contentId) ||
        !snapshot.isCompleted ||
        snapshot.resetCycle !== resetCycle
      ) continue;
      addRaidGateProgress(progress, snapshot.contentId, snapshot.difficulty, getSessionGate(snapshot.sessionId));
    }

    for (const encounter of recentEncounters) {
      const match = getRaidMatchForEncounter(encounter.current_boss);
      if (!match || !lane.raidIds.includes(match.contentId) || !encounter.cleared) continue;
      if (!wasCharacterInPlayers(character, encounter.players)) continue;
      const encounterTime = normalizeEncounterTimestamp(encounter.fight_start);
      if (encounterTime > 0 && encounterTime < weeklyResetMs) continue;
      addRaidGateProgress(progress, match.contentId, encounter.difficulty, match.gate ?? null);
    }

    for (const snapshot of sharedEncounterSnapshots) {
      if (
        !lane.raidIds.includes(snapshot.contentId) ||
        !snapshot.cleared ||
        snapshot.resetCycle !== resetCycle ||
        !(
          snapshot.matchedCharacterIds.includes(character.char_id) ||
          (characterName && snapshot.players.some((playerName) => playerName.trim().toLowerCase() === characterName))
        )
      ) continue;
      addRaidGateProgress(progress, snapshot.contentId, snapshot.difficulty, snapshot.gate ?? null);
    }

    let pendingProgress: { difficulty: string | null; clearedGates: string[]; expectedGates: string[] } | null = null;
    for (const [key, clearedGateSet] of progress.entries()) {
      const [contentId, difficulty] = key.split(':');
      const expectedGates = getExpectedRaidGates(contentId, difficulty);
      const clearedGates = Array.from(clearedGateSet).sort();
      if (expectedGates.length > 0 && expectedGates.every((gate) => clearedGateSet.has(gate))) {
        return { state: 'completed', difficulty: difficulty || null, clearedGates, expectedGates };
      }
      if (clearedGates.length > 0) {
        pendingProgress = { difficulty: difficulty || null, clearedGates, expectedGates };
      }
    }

    return pendingProgress
      ? { state: 'pending', ...pendingProgress }
      : { state: 'available', difficulty: null, clearedGates: [], expectedGates: [] };
  }

  function hasRaidCompleted(character: Character, lane: RaidLane): boolean {
    return getCharacterRaidProgress(character, lane).state === 'completed';
  }

  function getLocalClearDifficulty(character: Character, lane: RaidLane): string | null {
    return getCharacterRaidProgress(character, lane).difficulty;
  }

  function getCrossGroupClearNote(character: Character, lane: RaidLane): string | null {
    const ownerId = getCharacterOwner(character)?.id;
    if (!ownerId) return null;

    const resetCycle = getWeeklyResetCycle();
    const remoteNote = getCrossGroupClearNoteFromRemote(character, lane, ownerId, resetCycle);
    if (remoteNote) return remoteNote;

    for (const plan of savedPartyPlans) {
      if (plan.groupId === groupId) continue;
      if (!plan.members.some((member) => member.id === ownerId)) continue;

      const progress = new Map<string, Set<string>>();
      for (const snapshot of plan.completionSnapshots ?? []) {
        if (
          snapshot.discordId !== ownerId ||
          snapshot.charId !== character.char_id ||
          !snapshot.isCompleted ||
          snapshot.resetCycle !== resetCycle ||
          !lane.raidIds.includes(snapshot.contentId)
        ) continue;

        addRaidGateProgress(progress, snapshot.contentId, snapshot.difficulty, getSessionGate(snapshot.sessionId));
      }

      for (const snapshot of plan.encounterSnapshots ?? []) {
        if (
          snapshot.discordId !== ownerId ||
          snapshot.resetCycle !== resetCycle ||
          !snapshot.cleared ||
          !lane.raidIds.includes(snapshot.contentId) ||
          !(
            snapshot.matchedCharacterIds.includes(character.char_id) ||
            wasCharacterInPlayers(character, snapshot.players)
          )
        ) continue;

        addRaidGateProgress(progress, snapshot.contentId, snapshot.difficulty, snapshot.gate ?? null);
      }

      for (const [key, clearedGateSet] of progress.entries()) {
        const [contentId, difficulty] = key.split(':');
        const expectedGates = getExpectedRaidGates(contentId, difficulty);
        if (expectedGates.length > 0 && expectedGates.every((gate) => clearedGateSet.has(gate))) {
          const difficultyLabel = difficulty ? `${difficulty} ` : '';
          const groupLabel = plan.groupMode === 'static' ? `Static ${plan.groupName}` : `members from ${plan.groupName}`;
          return `Cleared ${difficultyLabel}with ${groupLabel}`;
        }
      }
    }

    return null;
  }

  function getCrossGroupClearNoteFromRemote(character: Character, lane: RaidLane, ownerId: string, resetCycle: string): string | null {
    const clearsByGroup = new Map<string, {
      groupName: string;
      groupMode: 'group' | 'static';
      progress: Map<string, Set<string>>;
    }>();

    for (const clear of remoteMemberClears) {
      if (
        clear.groupId === groupId ||
        clear.discordId !== ownerId ||
        clear.charId !== character.char_id ||
        clear.resetCycle !== resetCycle ||
        !lane.raidIds.includes(clear.contentId)
      ) continue;

      const entry = clearsByGroup.get(clear.groupId) ?? {
        groupName: clear.groupName,
        groupMode: clear.groupMode,
        progress: new Map<string, Set<string>>()
      };
      addRaidGateProgress(entry.progress, clear.contentId, clear.difficulty, clear.gate ?? getSessionGate(clear.sessionId));
      clearsByGroup.set(clear.groupId, entry);
    }

    for (const groupClear of clearsByGroup.values()) {
      for (const [key, clearedGateSet] of groupClear.progress.entries()) {
        const [contentId, difficulty] = key.split(':');
        const expectedGates = getExpectedRaidGates(contentId, difficulty);
        if (expectedGates.length > 0 && expectedGates.every((gate) => clearedGateSet.has(gate))) {
          const difficultyLabel = difficulty ? `${difficulty} ` : '';
          const groupLabel = groupClear.groupMode === 'static' ? `Static ${groupClear.groupName}` : `members from ${groupClear.groupName}`;
          return `Cleared ${difficultyLabel}with ${groupLabel}`;
        }
      }
    }

    return null;
  }

  function wasSeenInEncounterPlayers(character: Character, lane: RaidLane): boolean {
    const characterName = character.char_name.trim().toLowerCase();
    if (!characterName) return false;

    const wasSeenLocally = recentEncounters.some((encounter) =>
      lane.raidIds.includes(getRaidIdForEncounter(encounter.current_boss) ?? '') &&
      encounter.players.some((playerName) => playerName.trim().toLowerCase() === characterName)
    );

    const wasSeenShared = sharedEncounterSnapshots.some((snapshot) =>
      lane.raidIds.includes(snapshot.contentId) &&
      snapshot.cleared &&
      (
        snapshot.matchedCharacterIds.includes(character.char_id) ||
        snapshot.players.some((playerName) => playerName.trim().toLowerCase() === characterName)
      )
    );

    return wasSeenLocally || wasSeenShared;
  }

  function isCurrentUserStrictGroupOwner(plan: PartyPlanData | null = null): boolean {
    const localId = getLocalMemberId();
    const members = plan?.members ?? groupMembers;
    const ownerId = plan?.ownerDiscordId || currentOwnerDiscordId || members.find((member) => member.type === 'owner')?.id;

    return Boolean(
      (ownerId && ownerId === localId) ||
      members.some((member) => member.id === localId && member.type === 'owner')
    );
  }

  function getEncounterMemberIds(players: string[]): Set<string> {
    const playerNames = new Set(players.map((playerName) => playerName.trim().toLowerCase()).filter(Boolean));
    const memberIds = new Set<string>();

    for (const character of selectedCharacters) {
      if (!playerNames.has(character.char_name.trim().toLowerCase())) continue;
      const owner = getCharacterOwner(character);
      if (owner?.id) memberIds.add(owner.id);
    }

    return memberIds;
  }

  function getRequiredGroupMemberIds(): string[] {
    return groupMembers
      .filter((member) => getMemberCharacters(member).length > 0)
      .map((member) => member.id);
  }

  function didAllGroupMembersClearTogether(players: string[]): boolean {
    const requiredMemberIds = getRequiredGroupMemberIds();
    if (requiredMemberIds.length < 2) return false;

    const encounterMemberIds = getEncounterMemberIds(players);
    return requiredMemberIds.every((memberId) => encounterMemberIds.has(memberId));
  }

  function wasCharacterInPlayers(character: Character, players: string[]): boolean {
    const characterName = character.char_name.trim().toLowerCase();
    return Boolean(characterName && players.some((playerName) => playerName.trim().toLowerCase() === characterName));
  }

  function getTogetherClearDifficulty(character: Character, lane: RaidLane): string | null {
    const encounter = recentEncounters.find((entry) =>
      lane.raidIds.includes(getRaidIdForEncounter(entry.current_boss) ?? '') &&
      wasCharacterInPlayers(character, entry.players) &&
      didAllGroupMembersClearTogether(entry.players)
    );

    if (encounter?.difficulty) return encounter.difficulty;

    const sharedEncounter = sharedEncounterSnapshots.find((snapshot) =>
      lane.raidIds.includes(snapshot.contentId) &&
      snapshot.cleared &&
      (
        snapshot.matchedCharacterIds.includes(character.char_id) ||
        wasCharacterInPlayers(character, snapshot.players)
      ) &&
      didAllGroupMembersClearTogether(snapshot.players)
    );

    return sharedEncounter?.difficulty ?? null;
  }

  function getObservedClearNote(character: Character, lane: RaidLane): string | null {
    const localEncounter = recentEncounters.find((entry) =>
      lane.raidIds.includes(getRaidIdForEncounter(entry.current_boss) ?? '') &&
      wasCharacterInPlayers(character, entry.players)
    );
    const sharedEncounter = sharedEncounterSnapshots.find((snapshot) =>
      lane.raidIds.includes(snapshot.contentId) &&
      snapshot.cleared &&
      (
        snapshot.matchedCharacterIds.includes(character.char_id) ||
        wasCharacterInPlayers(character, snapshot.players)
      )
    );
    const players = localEncounter?.players ?? sharedEncounter?.players ?? [];
    const difficulty = localEncounter?.difficulty ?? sharedEncounter?.difficulty;
    if (!difficulty) return null;

    const ownerId = getCharacterOwner(character)?.id;
    const otherMemberNames = groupMembers
      .filter((member) => member.id !== ownerId && getEncounterMemberIds(players).has(member.id))
      .map((member) => member.name);

    if (otherMemberNames.length > 0) {
      return `Cleared ${difficulty} with ${otherMemberNames.join(', ')}`;
    }

    return `Cleared ${difficulty} public`;
  }

  function getRaidIdForEncounter(bossName: string): string | null {
    return getRaidMatchForEncounter(bossName)?.contentId ?? null;
  }

  function getCharacterRaidState(character: Character, lane: RaidLane): 'available' | 'pending' | 'completed' | 'too-low' | 'excluded' {
    if (!selectedCharacterIds.has(character.char_id)) return 'excluded';
    if (!canEnterRaid(character, lane)) return 'too-low';
    return getCharacterRaidProgress(character, lane).state;
  }

  function getVisibleCharactersForRaid(lane: RaidLane): Character[] {
    return selectedCharacters.filter((character) => {
      const state = getCharacterRaidState(character, lane);
      return state === 'available' || state === 'pending' || state === 'completed';
    });
  }

  function getAvailableMemberCharacters(member: PlannedMember): Character[] {
    return getMemberCharacters(member).filter((character) =>
      raidLanes.some((lane) => canAssignCharacterToRaid(character, lane))
    );
  }

  function getVisibleMemberCharacters(member: PlannedMember): Character[] {
    return getMemberCharacters(member).filter((character) => canCharacterEnterSelectedRaid(character));
  }

  function getCharacterOwner(character: Character): PlannedMember | null {
    const sharedOwner = groupMembers.find((member) =>
      sharedCharacters.some((sharedCharacter) =>
        sharedCharacter.discordId === member.id &&
        sharedCharacter.charId === character.char_id &&
        sharedCharacter.rosterId === character.roster_id
      )
    );

    if (sharedOwner) return sharedOwner;

    const syncedOwner = groupMembers.find((member) =>
      sharedCompletionSnapshots.some((snapshot) => snapshot.charId === character.char_id && snapshot.discordId === member.id)
    );

    if (syncedOwner) return syncedOwner;

    const friendOwner = groupMembers.find((member) =>
      member.type === 'friend' && member.testRosterId === character.roster_id
    );

    if (friendOwner) return friendOwner;
    return groupMembers.find((member) => member.id === getLocalMemberId()) ?? null;
  }

  function getOwnerColor(character: Character): string {
    const owner = getCharacterOwner(character);
    const ownerIndex = Math.max(groupMembers.findIndex((member) => member.id === owner?.id), 0);
    return owner?.color ?? defaultMemberColors[ownerIndex % defaultMemberColors.length];
  }

  function getRaidCapabilityNote(character: Character, lane: RaidLane): string {
    const availableDifficulties = lane.difficulties
      .filter((entry) => character.item_level >= entry.minIlvl)
      .sort((a, b) => a.minIlvl - b.minIlvl);

    const highest = availableDifficulties[availableDifficulties.length - 1];
    if (!highest) return 'Below entry iLvl';
    return `Can run up to ${highest.difficulty} mode`;
  }

  function getRaidBoardCharacterNote(character: Character, lane: RaidLane): string {
    const progress = getCharacterRaidProgress(character, lane);
    if (progress.state === 'pending') {
      const clearedGates = progress.clearedGates.join(', ');
      const expectedCount = progress.expectedGates.length || '?';
      const difficulty = progress.difficulty ? `${progress.difficulty} ` : '';
      return `Pending ${difficulty}${clearedGates} (${progress.clearedGates.length}/${expectedCount} gates)`;
    }

    const crossGroupClearNote = getCrossGroupClearNote(character, lane);
    if (crossGroupClearNote) return crossGroupClearNote;

    const togetherDifficulty = getTogetherClearDifficulty(character, lane);
    if (togetherDifficulty) return `Cleared ${togetherDifficulty} together`;

    const soloDifficulty = getLocalClearDifficulty(character, lane);
    if (soloDifficulty) return `Cleared ${soloDifficulty} public`;

    const observedClearNote = getObservedClearNote(character, lane);
    if (observedClearNote) return observedClearNote;

    return getRaidCapabilityNote(character, lane);
  }

  function getStaticRunTooltip(lane: RaidLane, charIds: number[]): string {
    const characters = charIds
      .map((charId) => selectedCharacters.find((character) => character.char_id === charId))
      .filter((character): character is Character => Boolean(character));

    const sharedDifficulties = lane.difficulties
      .filter((difficulty) => characters.every((character) => character.item_level >= difficulty.minIlvl))
      .sort((a, b) => a.minIlvl - b.minIlvl);

    const highest = sharedDifficulties[sharedDifficulties.length - 1];
    return highest ? `Can run ${highest.difficulty} mode together` : 'No shared eligible difficulty';
  }

  function toggleStaticRunSelection(laneId: string, charId: number) {
    if (!isCurrentUserOwner() || groupMode !== 'static') return;
    const current = new Set(staticRunSelections[laneId] ?? []);
    if (current.has(charId)) {
      current.delete(charId);
    } else {
      current.add(charId);
    }
    staticRunSelections = { ...staticRunSelections, [laneId]: current };
  }

  function getStaticRunConflict(charIds: number[]): string | null {
    const characters = charIds
      .map((charId) => selectedCharacters.find((character) => character.char_id === charId))
      .filter((character): character is Character => Boolean(character));
    const ownerIds = new Set<string>();
    const rosterIds = new Set<string>();

    for (const character of characters) {
      const ownerId = getCharacterOwner(character)?.id ?? 'unknown';
      if (ownerIds.has(ownerId)) return 'Only one character per group member can be connected.';
      if (rosterIds.has(character.roster_id)) return 'Only one character per roster can be connected.';
      ownerIds.add(ownerId);
      rosterIds.add(character.roster_id);
    }

    return null;
  }

  function createStaticRun(lane: RaidLane) {
    if (!isCurrentUserOwner()) return;
    const selected = Array.from(staticRunSelections[lane.id] ?? []);
    if (selected.length < 2) return;

    const conflict = getStaticRunConflict(selected);
    if (conflict) {
      setDropFeedbackMessage(conflict);
      return;
    }

    staticRunGroups = {
      ...staticRunGroups,
      [lane.id]: [...(staticRunGroups[lane.id] ?? []), selected]
    };
    staticRunSelections = { ...staticRunSelections, [lane.id]: new Set() };
    dirty = true;
  }

  function getStaticRunForCharacter(laneId: string, charId: number): number[] | null {
    return (staticRunGroups[laneId] ?? []).find((run) => run.includes(charId)) ?? null;
  }

  function hasConnectionAfter(lane: RaidLane, character: Character, assignedCharacters: Character[]): boolean {
    const run = getStaticRunForCharacter(lane.id, character.char_id);
    if (!run) return false;

    const index = assignedCharacters.findIndex((entry) => entry.char_id === character.char_id);
    const next = assignedCharacters[index + 1];
    return Boolean(next && run.includes(next.char_id));
  }

  function getRaidRangeLabel(lane: RaidLane): string {
    if (lane.minIlvl === lane.maxIlvl) {
      return `iLvl ${formatNumber(lane.minIlvl)}`;
    }
    return `iLvl ${formatNumber(lane.minIlvl)}-${formatNumber(lane.maxIlvl)}`;
  }

  async function copySheetUrl() {
    if (!sheetUrl) return;
    try {
      await navigator.clipboard.writeText(sheetUrl);
      saveState = 'saved';
      window.setTimeout(() => saveState = dirty ? 'idle' : 'saved', 1200);
    } catch (error) {
      console.warn('Failed to copy party plan URL:', error);
    }
  }

  async function importSheetUrl() {
    const value = importedSheetUrl.trim();
    if (!value) return;
    joinBlockedMessage = '';
    if (!currentDiscordId) {
      await loadCurrentDiscordAuth();
    }

    const importedGroupId = extractPartyPlanGroupId(value);
    const importedGroupSecret = extractPartyPlanGroupSecret(value);
    const spreadsheetId = extractPartyPlanSpreadsheetId(value);
    const savedPlan = importedGroupId ? await loadLocalPartyPlan(importedGroupId) : null;
    if (savedPlan) {
      if (!isLocalUserInvited(savedPlan)) {
        setJoinBlockedMessage(savedPlan);
        return;
      }
      applyPlan(savedPlan);
      if (importedGroupSecret && !groupSecret) {
        groupSecret = importedGroupSecret;
      }
      sheetUrl = buildPartyPlanInviteUrl(value, groupId, groupSecret);
      return;
    }

    groupId = importedGroupId ?? `imported-${Date.now()}`;
    groupSecret = importedGroupSecret ?? generateGroupSecret();
    sheetUrl = buildPartyPlanInviteUrl(value, groupId, groupSecret);
    if (spreadsheetId) {
      setRemoteSyncMessage('synced', `Sheet ${spreadsheetId} linked`);
    }
    groupCreated = true;
    groupName = importedGroupId ?? 'Imported group';
    if (groupMembers.length === 0) {
      groupMembers = [getLocalMember()];
    }
    if (selectedCharacterIds.size === 0) {
      selectedCharacterIds = new Set($characters.map((character) => character.char_id));
    }
    if (selectedRaidIds.size === 0) {
      selectedRaidIds = new Set(allRaidLanes.map((lane) => lane.id));
    }
    dirty = false;
    saveState = 'saved';
    if (partyPlanEndpointUrl.trim()) {
      await loadRemotePlan();
    } else {
      await saveCurrentPlan();
    }
  }

  async function openSavedPlan(plan: PartyPlanData) {
    if (!currentDiscordId) {
      await loadCurrentDiscordAuth();
    }
    applyPlan(plan);
    await loadCompletionSnapshots();
    await loadRecentEncounters();
  }

  function resetActiveGroup() {
    groupCreated = false;
    groupName = '';
    groupMode = 'group';
    groupId = '';
    groupSecret = '';
    currentOwnerDiscordId = '';
    groupMembers = [];
    selectedCharacterIds = new Set();
    selectedRaidIds = new Set();
    raidAssignments = {};
    sheetUrl = '';
    importedSheetUrl = '';
    dirty = false;
    saveState = 'idle';
    remoteSyncState = 'idle';
    remoteSyncMessage = '';
    lastPlanUpdatedAt = '';
    lastRemotePlanUpdatedAt = '';
    pendingRemotePlanUpdatedAt = '';
    remoteUpdatePending = false;
    activeGroupTab = 'configuration';
    staticRunSelections = {};
    staticRunGroups = {};
    dropFeedbackMessage = '';
    activeAssignPayload = '';
    remoteMemberClears = [];
    remoteStaticReservations = [];
    joinBlockedMessage = '';
    stopRemoteWatch();
    loadSavedPartyPlans();
  }

  function isLocalUserInvited(plan: PartyPlanData): boolean {
    const localId = getLocalMemberId();
    return plan.members.some((member) => member.id === localId) || plan.ownerDiscordId === localId;
  }

  function setJoinBlockedMessage(plan: PartyPlanData) {
    const owner = plan.members.find((member) => member.id === plan.ownerDiscordId || member.type === 'owner');
    joinBlockedMessage = `You are not on this group's member list yet. Ask ${owner?.name ?? 'the group owner'} to invite you first.`;
    groupCreated = false;
    groupId = '';
    groupSecret = '';
    groupMembers = [];
    selectedCharacterIds = new Set();
    selectedRaidIds = new Set();
    raidAssignments = {};
    sheetUrl = '';
    dirty = false;
    saveState = 'idle';
    remoteSyncState = 'idle';
  }

  function removeMemberFromPlan(plan: PartyPlanData, memberId: string): PartyPlanData {
    return {
      ...plan,
      members: plan.members.filter((member) => member.id !== memberId),
      characters: plan.characters.filter((character) => character.discordId !== memberId),
      assignments: plan.assignments.filter((assignment) =>
        assignment.assignmentType !== 'member' || assignment.targetId !== memberId
      ),
      completionSnapshots: (plan.completionSnapshots ?? []).filter((snapshot) => snapshot.discordId !== memberId),
      encounterSnapshots: (plan.encounterSnapshots ?? []).filter((snapshot) => snapshot.discordId !== memberId),
      updatedAt: new Date().toISOString()
    };
  }

  function getLocallyOwnedPlanMemberIds(): Set<string> {
    const ownerIds = new Set<string>();
    ownerIds.add(getLocalMemberId());
    return ownerIds;
  }

  function buildOwnerScopedRemotePlan(currentPlan: PartyPlanData, localSnapshotPlan: PartyPlanData, ownerIds: Set<string>): PartyPlanData {
    const isLocalOwner = (ownerId: string) => ownerIds.has(ownerId);

    return {
      ...currentPlan,
      characters: localSnapshotPlan.characters.filter((character) => isLocalOwner(character.discordId)),
      completionSnapshots: (localSnapshotPlan.completionSnapshots ?? []).filter((snapshot) => isLocalOwner(snapshot.discordId)),
      encounterSnapshots: (localSnapshotPlan.encounterSnapshots ?? []).filter((snapshot) => isLocalOwner(snapshot.discordId))
    };
  }

  async function deleteCurrentGroup() {
    if (!groupId || !window.confirm(`Delete "${groupName || 'this group'}" for everyone?`)) return;

    const config = getRemoteSyncConfig();
    setRemoteSyncMessage('syncing', 'Deleting group...');

    try {
      if (config) {
        await deletePartyPlanFromSheet(config);
      }
      await deleteLocalPartyPlan(groupId);
      resetActiveGroup();
    } catch (error) {
      setRemoteSyncMessage('error', error instanceof Error ? error.message : String(error));
    }
  }

  async function leaveCurrentGroup() {
    if (!groupId || !window.confirm(`Leave "${groupName || 'this group'}"?`)) return;

    if (isCurrentUserOwner()) {
      await deleteCurrentGroup();
      return;
    }

    const memberId = getLocalMemberId();
    const config = getRemoteSyncConfig();
    setRemoteSyncMessage('syncing', 'Leaving group...');

    try {
      if (config) {
        const remotePlan = await loadPartyPlanFromSheet(config);
        const sourcePlan = remotePlan ?? buildCurrentPlan();
        const updatedPlan = removeMemberFromPlan(sourcePlan, memberId);
        await savePartyPlanToSheet(updatedPlan, config);
      }
      await deleteLocalPartyPlan(groupId);
      resetActiveGroup();
    } catch (error) {
      setRemoteSyncMessage('error', error instanceof Error ? error.message : String(error));
    }
  }

  function getRemoteSyncConfig() {
    const endpointUrl = partyPlanEndpointUrl.trim();
    if (!endpointUrl || !groupId || !groupSecret) return null;

    return {
      endpointUrl,
      groupId,
      groupSecret
    };
  }

  function persistRemoteEndpoint() {
    localStorage.setItem('meowgang.partyPlan.endpointUrl', partyPlanEndpointUrl.trim());
  }

  function startRemoteWatch() {
    if (!groupId || watchedRemoteGroupId === groupId) return;

    stopRemoteWatch();
    watchedRemoteGroupId = groupId;
    snapshotWatchTimer = window.setInterval(checkSnapshotChanges, 60_000);
  }

  function stopRemoteWatch() {
    if (snapshotWatchTimer !== null) {
      window.clearInterval(snapshotWatchTimer);
      snapshotWatchTimer = null;
    }
    watchedRemoteGroupId = '';
    snapshotSyncInFlight = false;
  }

  async function loadRemotePlan() {
    const config = getRemoteSyncConfig();
    if (!config) {
      setRemoteSyncMessage('error', 'Endpoint, group id, or invite secret is missing.');
      return;
    }

    setRemoteSyncMessage('syncing', 'Loading remote plan...');
    persistRemoteEndpoint();

    try {
      const remotePlan = await loadPartyPlanFromSheet(config);
      if (!remotePlan) {
        setRemoteSyncMessage('error', 'No remote group found for this invite.');
        return;
      }
      if (!isLocalUserInvited(remotePlan)) {
        setJoinBlockedMessage(remotePlan);
        return;
      }

      applyPlan(remotePlan);
      await saveCurrentPlan();
      lastSnapshotFingerprint = getSnapshotFingerprint(buildLocalSnapshotPlan());
      applyRemoteSnapshots(remotePlan);
      lastRemotePlanUpdatedAt = remotePlan.updatedAt;
      pendingRemotePlanUpdatedAt = '';
      remoteUpdatePending = false;
      await refreshRemoteMemberClears(config);
      startRemoteWatch();
      setRemoteSyncMessage('synced', 'Remote plan loaded.');
    } catch (error) {
      setRemoteSyncMessage('error', error instanceof Error ? error.message : String(error));
    }
  }

  function getPrimarySaveLabel(): string {
    if (remoteSyncState === 'syncing' || saveState === 'saving') return 'Saving...';
    if (!partyPlanEndpointUrl.trim()) return dirty ? 'Save local' : 'Saved';
    if (dirty && remoteUpdatePending) return 'Save & Merge';
    if (dirty) return localRosterUploaded ? 'Save & Update' : 'Save & Upload';
    if (remoteUpdatePending) return 'Merge updates';
    return localRosterUploaded ? 'Update' : 'Upload';
  }

  function isPrimarySaveDisabled(): boolean {
    if (remoteSyncState === 'syncing' || saveState === 'saving') return true;
    if (!partyPlanEndpointUrl.trim()) return !dirty;
    return false;
  }

  async function saveCombinedPlan() {
    if (!partyPlanEndpointUrl.trim()) {
      if (dirty) await saveChanges();
      return;
    }

    await saveRemotePlan();
  }

  async function saveRemotePlan() {
    const config = getRemoteSyncConfig();
    if (!config) {
      setRemoteSyncMessage('error', 'Endpoint, group id, or invite secret is missing.');
      return;
    }

    setRemoteSyncMessage(
      'syncing',
      dirty
        ? remoteUpdatePending
          ? 'Saving locally, then merging remote changes...'
          : 'Saving locally, then uploading...'
        : remoteUpdatePending
          ? 'Merging remote changes...'
          : 'Saving remote plan...'
    );
    persistRemoteEndpoint();

    try {
      if (dirty) {
        saveState = 'saving';
        await saveCurrentPlan();
        dirty = false;
        saveState = 'saved';
      }

      const currentPlan = buildCurrentPlan();
      const localSnapshotPlan = buildLocalSnapshotPlan(currentPlan.updatedAt);
      const ownerIds = getLocallyOwnedPlanMemberIds();
      const ownerScopedPlan = buildOwnerScopedRemotePlan(currentPlan, localSnapshotPlan, ownerIds);
      const remotePlan = await saveMergedPartyPlanToSheet(ownerScopedPlan, config, Array.from(ownerIds));
      applyPlan(remotePlan);
      await saveCurrentPlan();
      lastSnapshotFingerprint = getSnapshotFingerprint(buildLocalSnapshotPlan());
      applyRemoteSnapshots(remotePlan);
      lastRemotePlanUpdatedAt = remotePlan.updatedAt;
      pendingRemotePlanUpdatedAt = '';
      remoteUpdatePending = false;
      localRosterUploaded = true;
      dirty = false;
      await refreshRemoteMemberClears(config);
      startRemoteWatch();
      setRemoteSyncMessage('synced', 'Remote plan saved.');
    } catch (error) {
      setRemoteSyncMessage('error', error instanceof Error ? error.message : String(error));
    }
  }

  async function refreshRemoteMemberClears(config = getRemoteSyncConfig()) {
    if (!config) return;
    const memberIds = Array.from(new Set(groupMembers.map((member) => member.id).filter(Boolean)));
    const [clearGroups, reservationGroups] = await Promise.all([
      Promise.all(memberIds.map((memberId) => loadPartyPlanMemberClearsFromSheet(config, memberId))),
      Promise.all(memberIds.map((memberId) => loadPartyPlanStaticReservationsFromSheet(config, memberId)))
    ]);
    remoteMemberClears = clearGroups.flat();
    remoteStaticReservations = reservationGroups.flat();
  }

  async function checkSnapshotChanges() {
    if (!groupCreated || snapshotSyncInFlight) return;

    const config = getRemoteSyncConfig();
    if (!config) return;

    snapshotSyncInFlight = true;
    try {
      const remoteStatus = await loadPartyPlanStatusFromSheet(config);
      if (remoteStatus?.updatedAt && remoteStatus.updatedAt !== lastRemotePlanUpdatedAt) {
        const remotePlan = await loadPartyPlanFromSheet(config);
        if (dirty) {
          if (remoteStatus.updatedAt !== pendingRemotePlanUpdatedAt) {
            pendingRemotePlanUpdatedAt = remoteStatus.updatedAt;
            remoteUpdatePending = true;
            setRemoteSyncMessage('idle', 'Remote changes detected. Save & merge when ready.');
          }
        } else if (remotePlan) {
          applyPlan(remotePlan);
          await saveCurrentPlan();
          applyRemoteSnapshots(remotePlan);
          lastRemotePlanUpdatedAt = remotePlan.updatedAt;
          pendingRemotePlanUpdatedAt = '';
          remoteUpdatePending = false;
          await refreshRemoteMemberClears(config);
          setRemoteSyncMessage('synced', 'Group data updated.');
        }
      }

      await loadCompletionSnapshots();

      const nextCompletionFingerprint = getCompletionWatchFingerprint();
      if (!lastCompletionWatchFingerprint) {
        lastCompletionWatchFingerprint = nextCompletionFingerprint;
        return;
      }
      if (!nextCompletionFingerprint || nextCompletionFingerprint === lastCompletionWatchFingerprint) return;

      await loadRecentEncounters();
      const snapshotPlan = buildLocalSnapshotPlan();
      const nextFingerprint = getSnapshotFingerprint(snapshotPlan);
      if (!nextFingerprint || nextFingerprint === lastSnapshotFingerprint) {
        lastCompletionWatchFingerprint = nextCompletionFingerprint;
        return;
      }

      const savedSnapshotPlan = await savePartyPlanSnapshotsToSheet(snapshotPlan, config);
      if (savedSnapshotPlan) {
        applyRemoteSnapshots(savedSnapshotPlan);
        lastRemotePlanUpdatedAt = savedSnapshotPlan.updatedAt;
      }
      lastCompletionWatchFingerprint = nextCompletionFingerprint;
      lastSnapshotFingerprint = nextFingerprint;
      setRemoteSyncMessage('synced', 'Snapshot state synced.');
    } catch (error) {
      console.warn('Party Plan background snapshot sync failed:', error);
      setRemoteSyncMessage('idle', 'Background sync will retry.');
    } finally {
      snapshotSyncInFlight = false;
    }
  }

  async function saveChanges() {
    saveState = 'saving';
    await new Promise((resolve) => window.setTimeout(resolve, 450));
    await saveCurrentPlan();
    dirty = false;
    saveState = 'saved';
  }

  function buildCurrentPlan(): PartyPlanData {
    const now = new Date().toISOString();
    const completionSnapshots = buildCompletionSnapshots(now);
    const encounterSnapshots = buildEncounterSnapshots(now);

    return {
      groupId,
      groupSecret,
      groupName: groupName || 'Imported group',
      groupMode,
      ownerDiscordId: currentOwnerDiscordId || buildCurrentOwnerId(),
      sheetUrl,
      sheetVersion: 1,
      members: groupMembers,
      characters: characterGroups.flatMap((group) =>
        group.characters.map((character) => ({
          charId: character.char_id,
          discordId: group.member.id,
          rosterId: character.roster_id,
          rosterName: group.rosterName,
          charName: character.char_name,
          classId: character.class_id,
          iconId: character.icon_id,
          itemLevel: character.item_level,
          combatPower: character.combat_power,
          included: selectedCharacterIds.has(character.char_id),
          displayOrder: character.display_order
        }))
      ),
      plannedRaids: allRaidLanes.map((lane) => ({
        raidId: lane.id,
        raidName: lane.name,
        minIlvl: lane.minIlvl,
        maxIlvl: lane.maxIlvl,
        enabled: selectedRaidIds.has(lane.id)
      })),
      assignments: getCurrentAssignments(),
      completionSnapshots: mergeSnapshotsByLocalOwners(sharedCompletionSnapshots, completionSnapshots),
      encounterSnapshots: mergeSnapshotsByLocalOwners(sharedEncounterSnapshots, encounterSnapshots),
      createdAt: now,
      updatedAt: now
    };
  }

  async function saveCurrentPlan() {
    if (!groupId) return;
    const savedPlan = await saveLocalPartyPlan(buildCurrentPlan());
    groupId = savedPlan.groupId;
    groupSecret = savedPlan.groupSecret || groupSecret || generateGroupSecret();
    sheetUrl = buildPartyPlanInviteUrl(savedPlan.sheetUrl, groupId, groupSecret);
    lastPlanUpdatedAt = savedPlan.updatedAt;
    sharedCharacters = savedPlan.characters ?? [];
    sharedCompletionSnapshots = getCurrentResetCompletionSnapshots(savedPlan.completionSnapshots ?? []);
    sharedEncounterSnapshots = getCurrentResetEncounterSnapshots(savedPlan.encounterSnapshots ?? []);
    lastCompletionWatchFingerprint = getCompletionWatchFingerprint();
    await loadSavedPartyPlans();
  }

  function applyPlan(plan: PartyPlanData) {
    groupId = plan.groupId;
    groupSecret = plan.groupSecret || generateGroupSecret();
    currentOwnerDiscordId = plan.ownerDiscordId ?? '';
    groupName = plan.groupName;
    groupMode = plan.groupMode ?? 'group';
    lastPlanUpdatedAt = plan.updatedAt;
    sheetUrl = buildPartyPlanInviteUrl(plan.sheetUrl, groupId, groupSecret);
    groupMembers = normalizeMembersForLocalUser(plan.members);
    selectedCharacterIds = new Set(plan.characters.filter((character) => character.included).map((character) => character.charId));
    selectedRaidIds = new Set(plan.plannedRaids.filter((raid) => raid.enabled).map((raid) => raid.raidId));
    sharedCharacters = plan.characters ?? [];
    sharedCompletionSnapshots = getCurrentResetCompletionSnapshots(plan.completionSnapshots ?? []);
    sharedEncounterSnapshots = getCurrentResetEncounterSnapshots(plan.encounterSnapshots ?? []);
    localRosterUploaded = plan.characters.some((character) => character.discordId === getLocalMemberId());
    groupCreated = true;
    dirty = false;
    saveState = 'saved';
    applyAssignments(plan.assignments);
  }
</script>

<div class="party-plan">
  {#if !partyPlanAuthLoaded}
    <section class="entry-panel restricted-panel">
      <p class="eyebrow">Party Plan</p>
      <h2>Checking access...</h2>
    </section>
  {:else if !canAccessPartyPlan}
    <section class="entry-panel restricted-panel">
      <p class="eyebrow">Restricted area</p>
      <h2>Party Plan is currently limited for testing.</h2>
      <p class="subtitle">This tab is only available for selected testers right now.</p>
    </section>
  {:else if !groupCreated}
    <section class="party-hero centered-entry">
      <div>
        <h2>Party Plan</h2>
        <p class="subtitle">Create a new group or join an existing shared plan.</p>
      </div>
    </section>
    <section class="entry-panel">
      <div class="entry-actions">
        <button
          type="button"
          class:active={entryMode === 'create'}
          on:click={() => entryMode = 'create'}
        >
          Create a new group
        </button>
        <button
          type="button"
          class="secondary-button"
          class:active={entryMode === 'join'}
          on:click={() => entryMode = 'join'}
        >
          Join existing group
        </button>
      </div>

      {#if savedPartyPlans.length > 0}
        <div class="saved-groups">
          <div class="panel-title-row compact-title">
            <div>
              <h3>Saved groups</h3>
              <p>{savedPartyPlans.length} remembered</p>
            </div>
          </div>
          <div class="saved-group-list">
            {#each savedPartyPlans as plan}
              <button type="button" class="saved-group-card" on:click={() => openSavedPlan(plan)}>
                <span>
                  <strong>{plan.groupName}</strong>
                  <em>{plan.members.length} member{plan.members.length === 1 ? '' : 's'}</em>
                </span>
                <small>{isCurrentUserOwner(plan) ? 'Owner' : 'Member'}</small>
              </button>
            {/each}
          </div>
        </div>
      {/if}

      {#if entryMode === 'create'}
        <div class="entry-form">
          <label for="group-name">Group name</label>
          <div class="inline-input">
            <input id="group-name" bind:value={groupName} placeholder="Group name" on:keydown={(event) => event.key === 'Enter' && createGroup()} />
            <button type="button" on:click={createGroup} disabled={!groupName.trim()}>Create</button>
          </div>
          <p class="form-warning">Group names cannot be renamed after creation.</p>
        </div>
      {:else if entryMode === 'join'}
        <div class="entry-form">
          <label for="join-sheet-url">Group URL</label>
          <div class="inline-input">
            <input id="join-sheet-url" bind:value={importedSheetUrl} placeholder="Paste Google Sheet URL" on:keydown={(event) => event.key === 'Enter' && importSheetUrl()} />
            <button type="button" on:click={importSheetUrl} disabled={!importedSheetUrl.trim()}>Join</button>
          </div>
          {#if joinBlockedMessage}
            <p class="form-warning">{joinBlockedMessage}</p>
          {/if}
        </div>
      {/if}
    </section>
  {:else}
    <section class="group-toolbar">
      <div class="active-group-heading">
        <button
          type="button"
          class="icon-button return-groups-button"
          on:click={resetActiveGroup}
          aria-label="Back to groups"
          title="Back to groups"
        >
          <span class="return-arrow-icon" aria-hidden="true"></span>
        </button>
        <div>
          <p class="eyebrow">Active group</p>
          <h3>{groupName || 'Imported group'}</h3>
          <p class="last-updated-label">Last updated: {formatDateTime(lastRemotePlanUpdatedAt || lastPlanUpdatedAt)}</p>
        </div>
      </div>

      <div class="group-mini-tabs toolbar-tabs" aria-label="Party plan sections">
        <button
          type="button"
          class:active={activeGroupTab === 'configuration'}
          on:click={() => activeGroupTab = 'configuration'}
        >
          Group configuration
        </button>
        <button
          type="button"
          class:active={activeGroupTab === 'raid-board'}
          on:click={() => activeGroupTab = 'raid-board'}
        >
          Raid board
        </button>
      </div>

      <div class="toolbar-actions">
        {#if isCurrentUserStrictGroupOwner()}
          <div class="sheet-chip">
            <input
              aria-label="Invite link"
              readonly
              value={sheetUrl ? 'Invite link ready' : 'No invite link yet'}
              title={sheetUrl ? 'Invite link ready' : 'No invite link yet'}
            />
            <button type="button" on:click={copySheetUrl} disabled={!sheetUrl}>Copy</button>
          </div>
        {/if}
        <div class="remote-sync-panel">
          <button type="button" class="secondary-button" on:click={loadRemotePlan} disabled={remoteSyncState === 'syncing' || !partyPlanEndpointUrl.trim()}>
            Load
          </button>
          <button
            type="button"
            class:dirty={dirty || remoteUpdatePending}
            class="secondary-button"
            on:click={saveCombinedPlan}
            disabled={isPrimarySaveDisabled()}
          >
            {getPrimarySaveLabel()}
          </button>
        </div>
        {#if isCurrentUserStrictGroupOwner()}
          <button type="button" class="danger-button delete-button" on:click={deleteCurrentGroup}>Delete</button>
        {:else}
          <button type="button" class="secondary-button" on:click={leaveCurrentGroup}>Leave</button>
        {/if}
      </div>
    </section>

    {#if activeGroupTab === 'configuration'}
      <div class="planner-layout configuration-layout">
        <section class="member-column">
          <div class="panel-title-row">
            <div>
              <h3>Group</h3>
              <p>{groupMembers.length} member{groupMembers.length === 1 ? '' : 's'} planned</p>
            </div>
          </div>

          <div class="member-list">
            {#each groupMembers as member}
              <div
                class="member-card"
                class:removable={member.id !== currentOwnerDiscordId}
              >
                <div>
                  <strong>{member.name}</strong>
                  <span>{member.id === currentOwnerDiscordId ? 'Owner' : member.id === getLocalMemberId() ? 'Your roster' : 'Member roster'}</span>
                </div>
                <label class="member-color-picker" aria-label={`${member.name} color`}>
                  <input
                    type="color"
                    value={member.color ?? defaultMemberColors[0]}
                    on:input={(event) => updateMemberColor(member.id, event.currentTarget.value)}
                  />
                </label>
                {#if isCurrentUserOwner() && member.id !== currentOwnerDiscordId}
                  <button type="button" class="member-remove-button" on:click={() => removeFriend(member.id)} aria-label="Remove friend">x</button>
                {/if}
              </div>
            {/each}
          </div>

          <div class="mode-switch" aria-label="Group behavior">
            <button
              type="button"
              class:active={groupMode === 'group'}
              on:click={() => setGroupMode('group')}
              disabled={!isCurrentUserOwner()}
            >
              Non-Static
            </button>
            <button
              type="button"
              class:active={groupMode === 'static'}
              on:click={() => setGroupMode('static')}
              disabled={!isCurrentUserOwner()}
            >
              Static
            </button>
          </div>

          {#if isCurrentUserOwner()}
            <div class="add-friend">
              <label for="friend-search">Add friend</label>
              <input id="friend-search" bind:value={friendSearch} placeholder="Search whitelist name" />
              {#if filteredFriends.length > 0}
                <div class="friend-results">
                  {#each filteredFriends as friend}
                    <button type="button" on:click={() => addFriend(friend)}>{friend.name}</button>
                  {/each}
                </div>
              {:else}
                <p class="muted">{friendSearch.trim() ? 'No whitelist member starts with that name.' : 'Type a whitelist name to search.'}</p>
              {/if}
            </div>
          {/if}

          <div class="raid-selection">
            <div class="panel-title-row compact-title">
              <div>
                <h3>Planned raids</h3>
                <p>{selectedRaidIds.size} selected</p>
              </div>
            </div>

            <div class="raid-selection-list">
              {#each allRaidLanes as lane}
                <label class="raid-selection-row">
                  <input
                    type="checkbox"
                    checked={selectedRaidIds.has(lane.id)}
                    disabled={!isCurrentUserOwner()}
                    on:change={() => toggleRaidTracking(lane.id)}
                  />
                  <span>{lane.name}</span>
                  <em>{getRaidRangeLabel(lane).replace('iLvl ', '')}</em>
                </label>
              {/each}
            </div>
          </div>
        </section>

        <section class="roster-column">
          <div class="panel-title-row">
            <div>
              <h3>Group characters</h3>
              <p>{selectedCharacters.length} selected for this plan</p>
            </div>
          </div>

          <div class="roster-groups compact-roster-groups">
            {#each characterGroups as rosterGroup (rosterGroup.key)}
              <div class="roster-group">
                <h4>{rosterGroup.member.name} / {rosterGroup.rosterName}</h4>
                <div class="character-list">
                  {#each rosterGroup.characters as character}
                    <label
                      class="character-row"
                      class:disabled={!selectedCharacterIds.has(character.char_id)}
                    >
                      <input
                        type="checkbox"
                        checked={selectedCharacterIds.has(character.char_id)}
                        disabled={getCharacterOwner(character)?.id !== getLocalMemberId()}
                        on:change={() => toggleCharacter(character.char_id)}
                      />
                      <img src={`/images/classes/${getClassIcon(character)}.png`} alt="" />
                      <span class="character-summary">
                        <span class="character-name">{character.char_name}</span>
                        <span class="character-stats">
                          {formatNumber(character.item_level)}
                          <em>{formatNumber(character.combat_power)}</em>
                        </span>
                      </span>
                    </label>
                  {/each}
                  {#if rosterGroup.characters.length === 0}
                    <p class="muted friend-empty-state">No local test roster linked yet.</p>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </section>
      </div>
    {:else}
      <div class="planner-layout raid-board-layout">
        <section class="runner-column">
          <div class="panel-title-row">
            <div>
              <h3>Runners</h3>
              <p>Select characters or friends, then assign them into raid lanes</p>
            </div>
          </div>
          <div class="member-runner-groups">
            {#each groupMembers as member}
              <div class="member-runner-group">
                <button
                  type="button"
                  class="member-collapse-button"
                  on:click={() => toggleMemberCollapsed(member.id)}
                >
                  <span>{collapsedMemberIds.has(member.id) ? '+' : '-'}</span>
                  <strong>{member.name}</strong>
                  <em>{getAvailableMemberCharacters(member).length} available</em>
                </button>

                <button
                  type="button"
                  class="member-card member-assign-label"
                  class:assign-selected={activeAssignPayload === getAssignPayload('member', member.id)}
                  on:click={() => selectAssignPayload('member', member.id)}
                  disabled={!isCurrentUserOwner()}
                >
                  <div>
                    <strong>{member.name}</strong>
                    <span>{member.id === currentOwnerDiscordId ? 'Owner label' : 'Member label'}</span>
                  </div>
                </button>

                {#if !collapsedMemberIds.has(member.id)}
                  {#if getVisibleMemberCharacters(member).length > 0}
                    <div class="runner-list">
                      {#each getVisibleMemberCharacters(member) as character}
                        <button
                          type="button"
                          class="runner-chip"
                          class:assign-selected={activeAssignPayload === getAssignPayload('character', character.char_id)}
                          class:reserved={Boolean(getStaticReservationForSelectedRaids(character))}
                          title={getStaticReservationTitle(character)}
                          on:click={() => selectAssignPayload('character', character.char_id)}
                          disabled={!isCurrentUserOwner() || getAssignableRaidCountForCharacter(character) === 0}
                        >
                          <img src={`/images/classes/${getClassIcon(character)}.png`} alt="" />
                          <span>{character.char_name}</span>
                          <em>{formatNumber(character.item_level)}</em>
                        </button>
                      {/each}
                    </div>
                  {:else}
                    <p class="muted friend-empty-state">Friend characters load after that member opens the shared group.</p>
                  {/if}
                {/if}
              </div>
            {/each}
          </div>
        </section>

        <section class="raid-column">
          <div class="panel-title-row">
            <div>
              <h3>Raid board</h3>
              <div class="raid-board-hints" aria-label="Raid board controls">
                {#if groupMode === 'static'}
                  <span>
                    <span class="mouse-icon left-click" aria-hidden="true"></span>
                    Select static run
                  </span>
                {/if}
                <span>
                  <span class="mouse-icon right-click" aria-hidden="true"></span>
                  Remove runner
                </span>
              </div>
            </div>
          </div>

          <div class="raid-lanes">
            {#if raidLanes.length === 0}
              <div class="empty-raid-board">Select at least one planned raid in Group configuration.</div>
            {/if}
            {#each raidLanes as lane}
              <div
                class="raid-lane"
                class:assign-armed={Boolean(activeAssignPayload)}
                role="region"
                aria-label={`${lane.name} party assignments`}
              >
                <div class="raid-lane-header">
                  <div>
                    <strong>{lane.name}</strong>
                    <span>{getRaidRangeLabel(lane)}</span>
                  </div>
                  <span
                    class="assignment-count"
                    title={`${getOpenGroupRunCount(lane)} open group runs (${getRaidCapacity(lane)}-player raid)`}
                  >
                    {getOpenGroupRunCount(lane)}
                  </span>
                </div>

                {#if activeAssignPayload}
                  <button
                    type="button"
                    class="lane-assign-button"
                    on:click={() => handleLaneAssign(lane)}
                  >
                    Assign selected here
                  </button>
                {/if}

                <div class="eligible-preview assigned-preview">
                  {#each getOrderedAssignedCharacters(lane) as character}
                    <button
                      type="button"
                      class="available-runner"
                      class:cleared={getCharacterRaidState(character, lane) === 'completed'}
                      class:pending={getCharacterRaidState(character, lane) === 'pending'}
                      class:static-selected={groupMode === 'static' && staticRunSelections[lane.id]?.has(character.char_id)}
                      class:static-connected={groupMode === 'static' && Boolean(getStaticRunForCharacter(lane.id, character.char_id))}
                      class:connection-after={groupMode === 'static' && hasConnectionAfter(lane, character, getOrderedAssignedCharacters(lane))}
                      style={`--owner-color: ${getOwnerColor(character)}`}
                      title={`${getCharacterOwner(character)?.name ?? 'Unknown'} - ${getRaidBoardCharacterNote(character, lane)}`}
                      on:click={() => toggleStaticRunSelection(lane.id, character.char_id)}
                      on:contextmenu={(event) => {
                        event.preventDefault();
                        removeAssignment(lane, `character:${character.char_id}`);
                      }}
                      disabled={!isCurrentUserOwner()}
                    >
                      <span class="owner-dot"></span>
                      <img src={`/images/classes/${getClassIcon(character)}.png`} alt="" />
                      <span class="runner-name">{character.char_name}</span>
                      <span class="runner-owner">{getCharacterOwner(character)?.name ?? 'Unknown'}</span>
                      <em>
                        iLvl {formatNumber(character.item_level)}
                        <strong>CP {formatNumber(character.combat_power)}</strong>
                      </em>
                    </button>
                  {:else}
                    <span class="drop-hint">No planned runners assigned yet</span>
                  {/each}
                </div>

                {#if groupMode === 'static' && getAssignedCharacters(lane).length > 1}
                  <div class="static-run-tools">
                    <button
                      type="button"
                      class="secondary-button"
                      on:click={() => createStaticRun(lane)}
                      disabled={!isCurrentUserOwner() || (staticRunSelections[lane.id]?.size ?? 0) < 2}
                    >
                      Connect selected
                    </button>
                    <span>{staticRunSelections[lane.id]?.size ?? 0} selected</span>
                  </div>
                {/if}

                {#if lane.assignments.length === 0}
                  <div class="assignment-list">
                    <span class="drop-hint">Assign planned runners here</span>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </section>
      </div>
    {/if}
  {/if}

  {#if partyNoticeMessage}
    <div class:error={partyNoticeType === 'error'} class:warning={partyNoticeType === 'warning'} class:success={partyNoticeType === 'success'} class="party-toast">
      <span>{partyNoticeMessage}</span>
    </div>
  {/if}
</div>
