<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { characters, rosters, type Character } from '$lib/store';
  import { GAME_CLASSES } from '$lib/data/classes';
  import { RAIDS } from '$lib/data/raids';
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
    loadPartyPlanFromSheet,
    saveLocalPartyPlan,
    savePartyPlanSnapshotsToSheet,
    savePartyPlanToSheet,
    type PartyPlanAssignment,
    type PartyPlanCompletionSnapshot,
    type PartyPlanData,
    type PartyPlanEncounterSnapshot,
    type PartyPlanRaidConfigSnapshot
  } from '$lib/services/party-plan';

  const PARTY_PLAN_ALLOWED_DISCORD_IDS = new Set([
    '592298453002878996',
    '222326155674386432',
    '330010523863220225'
  ]);

  let friendOptions: FriendOption[] = [];
  $: testFriendRosterIds = new Set(friendOptions.map((friend) => friend.testRosterId).filter(Boolean));

  let groupName = '';
  let groupId = '';
  let groupSecret = '';
  let currentOwnerDiscordId = '';
  let groupCreated = false;
  let groupMembers: PlannedMember[] = [];
  let partyPlanAuthLoaded = false;
  let currentDiscordId = '';
  let currentDiscordName = '';
  let savedPartyPlans: PartyPlanData[] = [];
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
  let snapshotSyncInFlight = false;
  let lastSnapshotFingerprint = '';
  let lastCompletionWatchFingerprint = '';
  let lastRemoteSnapshotFingerprint = '';
  let activeGroupTab: 'configuration' | 'raid-board' = 'configuration';
  let dropFeedbackMessage = '';
  let activeAssignPayload = '';
  let completionByCharacter: Record<string, CompletionStatusEntry[]> = {};
  let raidConfigsByCharacter: Record<string, CharacterRaidConfig[]> = {};
  let recentEncounters: EncounterPreview[] = [];
  let sharedRaidConfigSnapshots: PartyPlanRaidConfigSnapshot[] = [];
  let sharedCompletionSnapshots: PartyPlanCompletionSnapshot[] = [];
  let sharedEncounterSnapshots: PartyPlanEncounterSnapshot[] = [];
  let selectedRaidIds = new Set<string>();
  let raidAssignments: Record<string, string[]> = {};
  let collapsedMemberIds = new Set<string>();
  let entryMode: 'create' | 'join' | null = null;
  let staticRunSelections: Record<string, Set<number>> = {};
  let staticRunGroups: Record<string, number[][]> = {};

  const defaultMemberColors = ['#ff8c42', '#38bdf8', '#a78bfa', '#34d399', '#f472b6', '#facc15'];

  onMount(async () => {
    await loadPartyPlanEndpointUrl();
    await loadCurrentDiscordAuth();
    if (!PARTY_PLAN_ALLOWED_DISCORD_IDS.has(currentDiscordId)) return;

    loadSavedPartyPlans();
    loadWhitelistMembers();
    loadCompletionSnapshots();
    loadRecentEncounters();
    snapshotWatchTimer = window.setInterval(checkSnapshotChanges, 60_000);
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

  $: selectedCharacters = $characters.filter((character) => selectedCharacterIds.has(character.char_id));

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
  $: characterGroups = buildCharacterGroups(groupMembers, $characters, orderedRosters);

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
    const storedEndpoint = localStorage.getItem('meowgang.partyPlan.endpointUrl')?.trim();
    if (storedEndpoint) {
      partyPlanEndpointUrl = storedEndpoint;
      return;
    }

    try {
      partyPlanEndpointUrl = (await invoke<string | null>('get_party_plan_endpoint_url')) ?? '';
    } catch (error) {
      console.warn('Failed to load Party Plan endpoint URL:', error);
      partyPlanEndpointUrl = '';
    }
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
      type: 'self',
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
          type: 'self' as const,
          color: member.color ?? defaultMemberColors[index % defaultMemberColors.length]
        };
      }

      return {
        ...member,
        type: 'friend' as const,
        color: member.color ?? defaultMemberColors[index % defaultMemberColors.length]
      };
    });

    return hasLocalMember ? normalized : [...normalized, getLocalMember()];
  }

  function getLocalMemberId(): string {
    return currentDiscordId || groupMembers.find((member) => member.type === 'self')?.id || 'self';
  }

  function isCurrentUserOwner(plan: PartyPlanData | null = null): boolean {
    const ownerId = plan?.ownerDiscordId ?? currentOwnerDiscordId;
    return Boolean(ownerId && ownerId === getLocalMemberId());
  }

  function buildCurrentOwnerId(): string {
    return groupMembers.find((member) => member.type === 'self')?.id || currentDiscordId || 'self';
  }

  function addFriend(friend: FriendOption) {
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
    groupMembers = groupMembers.filter((member) => member.id !== memberId || member.type === 'self');
    dirty = true;
  }

  function updateMemberColor(memberId: string, color: string) {
    groupMembers = groupMembers.map((member) =>
      member.id === memberId ? { ...member, color } : member
    );
    dirty = true;
  }

  function toggleCharacter(characterId: number) {
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
    if (member.type === 'friend' && member.testRosterId) {
      return selectedCharacters.filter((character) => character.roster_id === member.testRosterId);
    }

    if (member.type === 'self') {
      return selectedCharacters.filter((character) => !testFriendRosterIds.has(character.roster_id));
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

    const selfMember = members.find((member) => member.type === 'self');
    if (selfMember) {
      const selfCharacters = characterList.filter((character) => !testFriendRosterIds.has(character.roster_id));
      for (const rosterGroup of groupCharactersByRoster(selfCharacters, rosterList)) {
        groups.push({
          key: `${selfMember.id}-${rosterGroup.rosterId}`,
          member: selfMember,
          rosterName: rosterGroup.rosterName,
          characters: rosterGroup.characters
        });
      }
    }

    for (const member of members.filter((entry) => entry.type === 'friend')) {
      if (!member.testRosterId) {
        groups.push({
          key: member.id,
          member,
          rosterName: 'Shared roster',
          characters: []
        });
        continue;
      }

      groups.push({
        key: `${member.id}-${member.testRosterId}`,
        member,
        rosterName: getRosterName(member.testRosterId, rosterList),
        characters: characterList
          .filter((character) => character.roster_id === member.testRosterId)
          .sort((a, b) => a.display_order - b.display_order)
      });
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
    activeAssignPayload = getAssignPayload(kind, id);
    setDropFeedbackMessage('Runner selected. Choose a raid lane.', 'info');
  }

  function handleLaneAssign(lane: RaidLane) {
    if (!activeAssignPayload) return;
    assignPayloadToLane(activeAssignPayload, lane);
  }

  function assignPayloadToLane(payload: string, lane: RaidLane) {
    if (!payload) {
      setDropFeedbackMessage('No runner selected. Click a runner, then assign it to a raid lane.');
      return;
    }

    try {
      const parsed = JSON.parse(payload) as { kind: 'character' | 'member'; id: string };
      const assignmentIds = getDropAssignmentIds(parsed, lane);
      if (assignmentIds.length === 0) {
        setDropFeedbackMessage(`${lane.name}: no iLvl eligible selected runners for this lane.`);
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
      const runnerAssignments = laneAssignments.map((assignmentId, slotOrder) => {
        const [assignmentType, targetId] = assignmentId.split(':') as ['member' | 'character', string];
        return {
          raidId: lane.id,
          assignmentType,
          targetId,
          slotOrder
        };
      });

      const assignedCharacterIds = new Set(
        laneAssignments
          .map((assignmentId) => assignmentId.split(':'))
          .filter(([assignmentType]) => assignmentType === 'character')
          .map(([, targetId]) => Number(targetId))
          .filter((charId) => Number.isFinite(charId))
      );

      const staticAssignments = (staticRunGroups[lane.id] ?? [])
        .map((run) => run.filter((charId) => assignedCharacterIds.has(charId)))
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

  function buildRaidConfigSnapshots(updatedAt: string): PartyPlanRaidConfigSnapshot[] {
    const plannedRaidIds = new Set(allRaidLanes.flatMap((lane) => lane.raidIds));

    return characterGroups.flatMap((group) =>
      group.characters.flatMap((character) => {
        if (!selectedCharacterIds.has(character.char_id)) return [];

        return (raidConfigsByCharacter[String(character.char_id)] || [])
          .filter((config) => plannedRaidIds.has(config.content_id))
          .map((config) => ({
            discordId: group.member.id,
            rosterId: character.roster_id,
            charId: character.char_id,
            charName: character.char_name,
            contentId: config.content_id,
            gate: config.gate ?? '',
            difficulty: config.difficulty,
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
      const contentId = getRaidIdForEncounter(encounter.current_boss);
      if (!contentId || !selectedRaidIds.has(contentId)) return [];

      const encounterTime = normalizeEncounterTimestamp(encounter.fight_start);
      if (encounterTime > 0 && encounterTime < weeklyResetMs) return [];

      const matchedCharacterIds = encounter.players
        .map((playerName) => selectedCharactersByName.get(playerName.trim().toLowerCase())?.char_id)
        .filter((charId): charId is number => typeof charId === 'number');

      if (matchedCharacterIds.length === 0) return [];

      const localCharacter = selectedCharactersByName.get(encounter.local_player.trim().toLowerCase());
      const localOwner = localCharacter ? getCharacterOwner(localCharacter) : groupMembers.find((member) => member.type === 'self');

      return [{
        discordId: localOwner?.id ?? 'self',
        localPlayer: encounter.local_player,
        contentId,
        raidName: allRaidLanes.find((lane) => lane.id === contentId)?.name ?? encounter.current_boss,
        difficulty: encounter.difficulty,
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
      raidConfigSnapshots: buildRaidConfigSnapshots(updatedAt),
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
      raidConfigSnapshots: (plan.raidConfigSnapshots ?? [])
        .map((snapshot) => ({
          discordId: snapshot.discordId,
          charId: snapshot.charId,
          contentId: snapshot.contentId,
          gate: snapshot.gate,
          difficulty: snapshot.difficulty
        }))
        .sort((a, b) => `${a.discordId}:${a.charId}:${a.contentId}:${a.gate}`.localeCompare(`${b.discordId}:${b.charId}:${b.contentId}:${b.gate}`)),
      encounterSnapshots: plan.encounterSnapshots
        .map((snapshot) => ({
          discordId: snapshot.discordId,
          localPlayer: snapshot.localPlayer,
          contentId: snapshot.contentId,
          difficulty: snapshot.difficulty,
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
    raidConfigSnapshots: PartyPlanRaidConfigSnapshot[],
    completionSnapshots: PartyPlanCompletionSnapshot[],
    encounterSnapshots: PartyPlanEncounterSnapshot[]
  ): string {
    return JSON.stringify({
      raidConfigSnapshots: raidConfigSnapshots
        .map((snapshot) => ({
          discordId: snapshot.discordId,
          charId: snapshot.charId,
          contentId: snapshot.contentId,
          gate: snapshot.gate,
          difficulty: snapshot.difficulty,
          updatedAt: snapshot.updatedAt
        }))
        .sort((a, b) => `${a.discordId}:${a.charId}:${a.contentId}:${a.gate}`.localeCompare(`${b.discordId}:${b.charId}:${b.contentId}:${b.gate}`)),
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
    const nextFingerprint = getSharedSnapshotFingerprint(
      plan.raidConfigSnapshots ?? [],
      plan.completionSnapshots ?? [],
      plan.encounterSnapshots ?? []
    );

    if (!nextFingerprint || nextFingerprint === lastRemoteSnapshotFingerprint) return false;

    sharedRaidConfigSnapshots = plan.raidConfigSnapshots ?? [];
    sharedCompletionSnapshots = plan.completionSnapshots ?? [];
    sharedEncounterSnapshots = plan.encounterSnapshots ?? [];
    lastRemoteSnapshotFingerprint = nextFingerprint;
    return true;
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
        return selectedCharacters.find((character) => String(character.char_id) === id) ?? null;
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

  function canEnterRaid(character: Character, lane: RaidLane): boolean {
    return character.item_level >= lane.minIlvl;
  }

  function canAssignCharacterToRaid(character: Character, lane: RaidLane): boolean {
    return selectedCharacterIds.has(character.char_id) && canEnterRaid(character, lane);
  }

  function getSharedCompletionSnapshot(character: Character, lane: RaidLane): PartyPlanCompletionSnapshot | undefined {
    const owner = getCharacterOwner(character);
    return sharedCompletionSnapshots.find((snapshot) =>
      snapshot.charId === character.char_id &&
      (!owner || snapshot.discordId === owner.id) &&
      lane.raidIds.includes(snapshot.contentId) &&
      snapshot.isCompleted
    );
  }

  function hasRaidCompleted(character: Character, lane: RaidLane): boolean {
    const completionStatus = completionByCharacter[String(character.char_id)] || [];
    const hasLocalCompletion = completionStatus.some((entry) =>
      lane.raidIds.includes(entry.content_id) && Number(entry.is_completed) === 1
    );
    return hasLocalCompletion || Boolean(getSharedCompletionSnapshot(character, lane));
  }

  function getLocalClearDifficulty(character: Character, lane: RaidLane): string | null {
    const completionStatus = completionByCharacter[String(character.char_id)] || [];
    const entry = completionStatus.find((status) =>
      lane.raidIds.includes(status.content_id) && Number(status.is_completed) === 1
    );

    return (entry as CompletionStatusEntry & { details?: string | null } | undefined)?.details
      ?? getSharedCompletionSnapshot(character, lane)?.difficulty
      ?? null;
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

  function getTogetherClearDifficulty(character: Character, lane: RaidLane): string | null {
    const characterName = character.char_name.trim().toLowerCase();
    if (!characterName) return null;

    const encounter = recentEncounters.find((entry) =>
      lane.raidIds.includes(getRaidIdForEncounter(entry.current_boss) ?? '') &&
      entry.players.some((playerName) => playerName.trim().toLowerCase() === characterName)
    );

    if (encounter?.difficulty) return encounter.difficulty;

    const sharedEncounter = sharedEncounterSnapshots.find((snapshot) =>
      lane.raidIds.includes(snapshot.contentId) &&
      snapshot.cleared &&
      (
        snapshot.matchedCharacterIds.includes(character.char_id) ||
        snapshot.players.some((playerName) => playerName.trim().toLowerCase() === characterName)
      )
    );

    return sharedEncounter?.difficulty ?? null;
  }

  function getRaidIdForEncounter(bossName: string): string | null {
    const normalizedBossName = bossName.trim().toLowerCase();
    const matchedLane = allRaidLanes.find((lane) =>
      lane.name.trim().toLowerCase() === normalizedBossName ||
      normalizedBossName.includes(lane.name.trim().toLowerCase())
    );

    return matchedLane?.id ?? null;
  }

  function getCharacterRaidState(character: Character, lane: RaidLane): 'available' | 'completed' | 'too-low' | 'excluded' {
    if (!selectedCharacterIds.has(character.char_id)) return 'excluded';
    if (!canEnterRaid(character, lane)) return 'too-low';
    if (hasRaidCompleted(character, lane) || wasSeenInEncounterPlayers(character, lane)) return 'completed';
    return 'available';
  }

  function getVisibleCharactersForRaid(lane: RaidLane): Character[] {
    return selectedCharacters.filter((character) => {
      const state = getCharacterRaidState(character, lane);
      return state === 'available' || state === 'completed';
    });
  }

  function getAvailableMemberCharacters(member: PlannedMember): Character[] {
    return getMemberCharacters(member).filter((character) =>
      raidLanes.some((lane) => canAssignCharacterToRaid(character, lane))
    );
  }

  function getCharacterOwner(character: Character): PlannedMember | null {
    const syncedOwner = groupMembers.find((member) =>
      sharedCompletionSnapshots.some((snapshot) => snapshot.charId === character.char_id && snapshot.discordId === member.id)
    );

    if (syncedOwner) return syncedOwner;

    const friendOwner = groupMembers.find((member) =>
      member.type === 'friend' && member.testRosterId === character.roster_id
    );

    if (friendOwner) return friendOwner;
    return groupMembers.find((member) => member.type === 'self') ?? null;
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

  function getPlannedRaidDifficulty(character: Character, lane: RaidLane): string | null {
    const owner = getCharacterOwner(character);
    const localDifficulty = getLocalPlannedRaidDifficulty(character, lane);
    if (owner?.type === 'self' && localDifficulty) return localDifficulty;

    const sharedConfigs = sharedRaidConfigSnapshots.filter((snapshot) =>
      snapshot.charId === character.char_id &&
      (!owner || snapshot.discordId === owner.id) &&
      lane.raidIds.includes(snapshot.contentId)
    );

    const sharedDifficulty = getAggregatedPlannedDifficulty(sharedConfigs.map((snapshot) => snapshot.difficulty));
    if (sharedDifficulty) return sharedDifficulty;

    return localDifficulty;
  }

  function getLocalPlannedRaidDifficulty(character: Character, lane: RaidLane): string | null {
    const localConfigs = (raidConfigsByCharacter[String(character.char_id)] || []).filter((entry) =>
      lane.raidIds.includes(entry.content_id)
    );

    return getAggregatedPlannedDifficulty(localConfigs.map((config) => config.difficulty));
  }

  function getAggregatedPlannedDifficulty(difficulties: string[]): string | null {
    const normalizedDifficulties = difficulties
      .map((difficulty) => difficulty?.trim())
      .filter((difficulty): difficulty is string => Boolean(difficulty));

    if (normalizedDifficulties.length === 0) return null;
    if (normalizedDifficulties.some((difficulty) => difficulty.toLowerCase() === 'mixed')) return 'mixed';

    const uniqueDifficulties = Array.from(new Set(normalizedDifficulties));
    return uniqueDifficulties.length === 1 ? uniqueDifficulties[0] : 'mixed';
  }

  function getRaidBoardCharacterNote(character: Character, lane: RaidLane): string {
    const togetherDifficulty = getTogetherClearDifficulty(character, lane);
    if (togetherDifficulty) return `Cleared ${togetherDifficulty} together`;

    const soloDifficulty = getLocalClearDifficulty(character, lane);
    if (soloDifficulty) return `Cleared ${soloDifficulty} public`;

    const plannedDifficulty = getPlannedRaidDifficulty(character, lane);
    if (plannedDifficulty) {
      return plannedDifficulty.toLowerCase() === 'mixed'
        ? 'Planned to run mixed gates'
        : `Planned to run ${plannedDifficulty} mode`;
    }

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
    if (!currentDiscordId) {
      await loadCurrentDiscordAuth();
    }

    const importedGroupId = extractPartyPlanGroupId(value);
    const importedGroupSecret = extractPartyPlanGroupSecret(value);
    const spreadsheetId = extractPartyPlanSpreadsheetId(value);
    const savedPlan = importedGroupId ? await loadLocalPartyPlan(importedGroupId) : null;
    if (savedPlan) {
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
    activeGroupTab = 'configuration';
    staticRunSelections = {};
    staticRunGroups = {};
    dropFeedbackMessage = '';
    activeAssignPayload = '';
    loadSavedPartyPlans();
  }

  function removeMemberFromPlan(plan: PartyPlanData, memberId: string): PartyPlanData {
    return {
      ...plan,
      members: plan.members.filter((member) => member.id !== memberId),
      characters: plan.characters.filter((character) => character.discordId !== memberId),
      assignments: plan.assignments.filter((assignment) =>
        assignment.assignmentType !== 'member' || assignment.targetId !== memberId
      ),
      raidConfigSnapshots: (plan.raidConfigSnapshots ?? []).filter((snapshot) => snapshot.discordId !== memberId),
      completionSnapshots: (plan.completionSnapshots ?? []).filter((snapshot) => snapshot.discordId !== memberId),
      encounterSnapshots: (plan.encounterSnapshots ?? []).filter((snapshot) => snapshot.discordId !== memberId),
      updatedAt: new Date().toISOString()
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

      applyPlan(remotePlan);
      await saveCurrentPlan();
      lastSnapshotFingerprint = getSnapshotFingerprint(buildLocalSnapshotPlan());
      applyRemoteSnapshots(remotePlan);
      setRemoteSyncMessage('synced', 'Remote plan loaded.');
    } catch (error) {
      setRemoteSyncMessage('error', error instanceof Error ? error.message : String(error));
    }
  }

  async function saveRemotePlan() {
    const config = getRemoteSyncConfig();
    if (!config) {
      setRemoteSyncMessage('error', 'Endpoint, group id, or invite secret is missing.');
      return;
    }

    setRemoteSyncMessage('syncing', 'Saving remote plan...');
    persistRemoteEndpoint();

    try {
      const remotePlan = await savePartyPlanToSheet(buildCurrentPlan(), config);
      applyPlan(remotePlan);
      await saveCurrentPlan();
      lastSnapshotFingerprint = getSnapshotFingerprint(buildLocalSnapshotPlan());
      applyRemoteSnapshots(remotePlan);
      dirty = false;
      setRemoteSyncMessage('synced', 'Remote plan saved.');
    } catch (error) {
      setRemoteSyncMessage('error', error instanceof Error ? error.message : String(error));
    }
  }

  async function checkSnapshotChanges() {
    if (!groupCreated || snapshotSyncInFlight) return;

    const config = getRemoteSyncConfig();
    if (!config) return;

    snapshotSyncInFlight = true;
    try {
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

      const remotePlan = await savePartyPlanSnapshotsToSheet(snapshotPlan, config);
      applyRemoteSnapshots(remotePlan);
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
    const raidConfigSnapshots = buildRaidConfigSnapshots(now);
    const completionSnapshots = buildCompletionSnapshots(now);
    const encounterSnapshots = buildEncounterSnapshots(now);

    return {
      groupId,
      groupSecret,
      groupName: groupName || 'Imported group',
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
      raidConfigSnapshots: mergeSnapshotsByLocalOwners(sharedRaidConfigSnapshots, raidConfigSnapshots),
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
    sharedRaidConfigSnapshots = savedPlan.raidConfigSnapshots ?? [];
    sharedCompletionSnapshots = savedPlan.completionSnapshots ?? [];
    sharedEncounterSnapshots = savedPlan.encounterSnapshots ?? [];
    lastCompletionWatchFingerprint = getCompletionWatchFingerprint();
    await loadSavedPartyPlans();
  }

  function applyPlan(plan: PartyPlanData) {
    groupId = plan.groupId;
    groupSecret = plan.groupSecret || generateGroupSecret();
    currentOwnerDiscordId = plan.ownerDiscordId ?? '';
    groupName = plan.groupName;
    sheetUrl = buildPartyPlanInviteUrl(plan.sheetUrl, groupId, groupSecret);
    groupMembers = normalizeMembersForLocalUser(plan.members);
    selectedCharacterIds = new Set(plan.characters.filter((character) => character.included).map((character) => character.charId));
    selectedRaidIds = new Set(plan.plannedRaids.filter((raid) => raid.enabled).map((raid) => raid.raidId));
    sharedRaidConfigSnapshots = plan.raidConfigSnapshots ?? [];
    sharedCompletionSnapshots = plan.completionSnapshots ?? [];
    sharedEncounterSnapshots = plan.encounterSnapshots ?? [];
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
        </div>
      {:else if entryMode === 'join'}
        <div class="entry-form">
          <label for="join-sheet-url">Group URL</label>
          <div class="inline-input">
            <input id="join-sheet-url" bind:value={importedSheetUrl} placeholder="Paste Google Sheet URL" on:keydown={(event) => event.key === 'Enter' && importSheetUrl()} />
            <button type="button" on:click={importSheetUrl} disabled={!importedSheetUrl.trim()}>Join</button>
          </div>
        </div>
      {/if}
    </section>
  {:else}
    <section class="group-toolbar">
      <div>
        <p class="eyebrow">Active group</p>
        <h3>{groupName || 'Imported group'}</h3>
      </div>

      <div class="toolbar-actions">
        <div class="sheet-chip">
          <span>{sheetUrl ? 'Invite link ready' : 'No invite link yet'}</span>
          <button type="button" on:click={copySheetUrl} disabled={!sheetUrl}>Copy</button>
        </div>
        <div class="remote-sync-panel">
          <button type="button" class="secondary-button" on:click={loadRemotePlan} disabled={remoteSyncState === 'syncing' || !partyPlanEndpointUrl.trim()}>
            Load
          </button>
          <button type="button" class="secondary-button" on:click={saveRemotePlan} disabled={remoteSyncState === 'syncing' || !partyPlanEndpointUrl.trim()}>
            Sync
          </button>
        </div>
        <button type="button" class:dirty={dirty} on:click={saveChanges} disabled={!dirty || saveState === 'saving'}>
          {#if saveState === 'saving'}
            Saving...
          {:else if dirty}
            Save changes
          {:else}
            Saved
          {/if}
        </button>
        <button type="button" class="secondary-button" on:click={resetActiveGroup}>Groups</button>
        {#if isCurrentUserOwner()}
          <button type="button" class="danger-button" on:click={deleteCurrentGroup}>Delete</button>
        {:else}
          <button type="button" class="danger-button" on:click={leaveCurrentGroup}>Leave</button>
        {/if}
      </div>
    </section>

    <div class="group-mini-tabs" aria-label="Party plan sections">
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
                class:removable={member.type === 'friend'}
              >
                <div>
                  <strong>{member.name}</strong>
                  <span>{member.type === 'self' ? 'Local roster' : 'Friend placeholder'}</span>
                </div>
                <label class="member-color-picker" aria-label={`${member.name} color`}>
                  <input
                    type="color"
                    value={member.color ?? defaultMemberColors[0]}
                    on:input={(event) => updateMemberColor(member.id, event.currentTarget.value)}
                  />
                </label>
                {#if member.type === 'friend'}
                  <button type="button" class="member-remove-button" on:click={() => removeFriend(member.id)} aria-label="Remove friend">x</button>
                {/if}
              </div>
            {/each}
          </div>

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
                >
                  <div>
                    <strong>{member.name}</strong>
                    <span>{member.type === 'self' ? 'Member label' : 'Friend label'}</span>
                  </div>
                </button>

                {#if !collapsedMemberIds.has(member.id)}
                  {#if getAvailableMemberCharacters(member).length > 0}
                    <div class="runner-list">
                      {#each getAvailableMemberCharacters(member) as character}
                        <button
                          type="button"
                          class="runner-chip"
                          class:assign-selected={activeAssignPayload === getAssignPayload('character', character.char_id)}
                          on:click={() => selectAssignPayload('character', character.char_id)}
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
                <span>
                  <span class="mouse-icon left-click" aria-hidden="true"></span>
                  Select static run
                </span>
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
                  <span class="assignment-count">{lane.assignments.length}</span>
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
                      class:static-selected={staticRunSelections[lane.id]?.has(character.char_id)}
                      class:static-connected={Boolean(getStaticRunForCharacter(lane.id, character.char_id))}
                      class:connection-after={hasConnectionAfter(lane, character, getOrderedAssignedCharacters(lane))}
                      style={`--owner-color: ${getOwnerColor(character)}`}
                      title={`${getCharacterOwner(character)?.name ?? 'Unknown'} - ${getRaidBoardCharacterNote(character, lane)}`}
                      on:click={() => toggleStaticRunSelection(lane.id, character.char_id)}
                      on:contextmenu={(event) => {
                        event.preventDefault();
                        removeAssignment(lane, `character:${character.char_id}`);
                      }}
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

                {#if getAssignedCharacters(lane).length > 1}
                  <div class="static-run-tools">
                    <button
                      type="button"
                      class="secondary-button"
                      on:click={() => createStaticRun(lane)}
                      disabled={(staticRunSelections[lane.id]?.size ?? 0) < 2}
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
