<script lang="ts">
  // MeowConnect is the orchestration layer for refresh, filters, popover state, and actions.
  // Individual tabs/panels own presentation only.
  import { createEventDispatcher, onMount } from 'svelte';
  import { appAsset } from '$lib/assets';
  import MeowConnectLogs from '$lib/components/meow-connect/MeowConnectLogs.svelte';
  import MeowConnectProfilePopover from '$lib/components/meow-connect/MeowConnectProfilePopover.svelte';
  import MeowConnectSettings from '$lib/components/meow-connect/MeowConnectSettings.svelte';
  import MeowConnectTogether from '$lib/components/meow-connect/MeowConnectTogether.svelte';
  import type { FriendOption, ProfileRaidGroup, RaidDifficultyFilterItem, RaidTogetherRow } from '$lib/components/meow-connect/types';
  import { RAIDS, type Raid } from '$lib/data/raids';
  import { loadDiscordWhitelistMembers } from '$lib/services/discord-whitelist';
  import {
    acceptMeowConnectFriendRequest,
    acceptMeowConnectGroupInvite,
    assignMeowConnectRaidToGroup,
    buildMeowConnectAvailabilityRows,
    buildMeowConnectLogEntries,
    createMeowConnectGroup,
    deleteMeowConnectGroup,
    getMeowConnectLastUploadAt,
    getMeowConnectRaidDifficulties,
    fetchMeowConnectRemoteSnapshots,
    getMeowConnectRaidOptions,
    hasMeowConnectConsent,
    inviteMeowConnectGroupMember,
    isMeowConnectRealtimeEnabled,
    leaveMeowConnectGroup,
    loadMeowConnectFriends,
    loadMeowConnectGroups,
    loadMeowConnectLocalSnapshot,
    logMeowConnectRequest,
    markMeowConnectActive,
    markMeowConnectConnecting,
    markMeowConnectFailure,
    meowConnectHasUnsyncedChanges,
    removeMeowConnectRaidGroupAssignment,
    removeMeowConnectFriend,
    renameMeowConnectGroup,
    searchMeowConnectProfiles,
    sendMeowConnectFriendRequest,
    setMeowConnectConsent,
    syncMeowConnectGroupTagsToLocal,
    subscribeMeowConnectChanges,
    uploadMeowConnectSnapshotIfNeeded,
    type MeowConnectAvailabilityRow,
    type MeowConnectFriendConnection,
    type MeowConnectGroup,
    type MeowConnectGroupMember,
    type MeowConnectLocalSnapshot,
    type MeowConnectProfile,
    type MeowConnectRemoteSnapshot
  } from '$lib/services/meow-connect';
  import { getCurrentSupabaseDiscordProfile } from '$lib/services/supabase-auth';

  type MeowConnectTab = 'together' | 'logs' | 'settings';

  const RAID_VISIBILITY_STORAGE_KEY = 'meowConnect.visibleRaidIds';
  const RAID_DIFFICULTY_FILTER_STORAGE_KEY = 'meowConnect.raidDifficultyFilters';
  const TOGETHER_FRIEND_SELECTION_STORAGE_KEY = 'meowConnect.togetherFriendSelectionIds';
  const LAST_MANUAL_SYNC_STORAGE_KEY = 'meowConnect.lastManualSyncAt';
  const MANUAL_SYNC_COOLDOWN_MS = 5 * 60 * 1000;
  const raidOptions = getMeowConnectRaidOptions()
    .sort((a, b) => a.gates[0].minIlvl - b.gates[0].minIlvl || a.name.localeCompare(b.name));

  export let activeSection: MeowConnectTab = 'together';
  const dispatch = createEventDispatcher<{ pendingRequestsChanged: number }>();
  const meowConnectIcon = appAsset('meowconnect_tab.png');

  let consentAccepted = false;
  let visibleRaidIds = raidOptions.map((raid) => raid.id);
  let raidDifficultyFilters: Record<string, string> = {};
  let activeProfileGroup: ProfileRaidGroup | null = null;
  let friendPopoverEl: HTMLElement | null = null;
  let currentProfile: MeowConnectProfile | null = null;
  let localSnapshot: MeowConnectLocalSnapshot | null = null;
  let remoteSnapshots: MeowConnectRemoteSnapshot[] = [];
  let friendConnections: MeowConnectFriendConnection[] = [];
  let meowGroups: MeowConnectGroup[] = [];
  let selectedTogetherFriendIds = new Set<string>();
  let togetherFriendSelectionInitialized = false;
  let friendOptions: FriendOption[] = [];
  let friendSearch = '';
  let friendDiscordId = '';
  let groupName = '';
  let groupTag = '';
  let activeGroupRenameId = '';
  let groupRenameInputs: Record<string, string> = {};
  let groupInviteInputs: Record<string, string> = {};
  let groupInviteOptions: MeowConnectProfile[] = [];
  let activeGroupInviteId = '';
  let showFriendPopover = false;
  let isLoading = false;
  let friendActionBusy = false;
  let groupActionBusy = false;
  let groupAssignmentBusyKey = '';
  let currentTime = Date.now();
  let toastMessage = '';
  let toastKind: 'success' | 'error' | 'info' = 'info';
  let unsubscribeRealtime: (() => void) | null = null;
  let realtimeRefreshTimer: ReturnType<typeof setTimeout> | null = null;
  let realtimeChangeBurstCount = 0;
  let groupInviteSearchTimer: ReturnType<typeof setTimeout> | null = null;
  let clockTimer: ReturnType<typeof setInterval> | null = null;
  let toastTimer: ReturnType<typeof setTimeout> | null = null;
  let lastDispatchedPendingRequestCount = -1;

  $: manualSyncRemainingMs = Math.max(0, getStoredTimestamp(LAST_MANUAL_SYNC_STORAGE_KEY) + MANUAL_SYNC_COOLDOWN_MS - currentTime);
  $: manualSyncBlocked = manualSyncRemainingMs > 0;
  $: manualSyncLabel = isLoading
    ? 'Syncing...'
    : manualSyncBlocked
      ? `Sync in ${formatDuration(manualSyncRemainingMs)}`
      : $meowConnectHasUnsyncedChanges
        ? 'Sync changes'
        : 'Sync now';
  $: connectedCharacterCount = localSnapshot?.characters.length || 0;
  $: unsyncedRosterChangeCount = $meowConnectHasUnsyncedChanges ? '1+' : '0';
  $: lastSyncLabel = formatLastSyncTime(getMeowConnectLastUploadAt(), currentTime);
  $: visibleRaids = raidOptions.filter((raid) => visibleRaidIds.includes(raid.id));
  $: raidDifficultyFilterItems = visibleRaids.map((raid): RaidDifficultyFilterItem => ({
    raid,
    difficulties: getRaidDifficultyOptions(raid.id),
    selectedDifficulty: getRaidDifficultyFilter(raid.id, raidDifficultyFilters)
  }));
  $: raidSections = visibleRaids.map((raid) => ({
    raid,
    rows: buildMeowConnectAvailabilityRows(localSnapshot, remoteSnapshots, raid.id, getRaidDifficultyFilter(raid.id, raidDifficultyFilters), new Set(), currentProfile),
    groups: groupRowsByProfile(
      raid.id,
      buildMeowConnectAvailabilityRows(localSnapshot, remoteSnapshots, raid.id, getRaidDifficultyFilter(raid.id, raidDifficultyFilters), new Set(), currentProfile)
    )
  }));
  $: displayedRaidSections = raidSections;
  $: logEntries = buildMeowConnectLogEntries(localSnapshot, remoteSnapshots, visibleRaidIds, currentProfile).slice(0, 80);
  $: acceptedFriendConnections = friendConnections
    .filter((connection) => connection.status === 'accepted')
    .sort(sortFriendConnections);
  $: sortedFriendConnections = [...friendConnections].sort(sortFriendConnections);
  $: if (!togetherFriendSelectionInitialized && acceptedFriendConnections.length > 0) {
    selectedTogetherFriendIds = loadTogetherFriendSelectionIds(acceptedFriendConnections);
    togetherFriendSelectionInitialized = true;
  }
  $: if (togetherFriendSelectionInitialized) {
    const acceptedKeys = new Set(acceptedFriendConnections.map(getFriendConnectionKey));
    const validSelectedIds = new Set(Array.from(selectedTogetherFriendIds).filter((key) => acceptedKeys.has(key)));
    if (validSelectedIds.size !== selectedTogetherFriendIds.size) {
      selectedTogetherFriendIds = validSelectedIds;
      saveTogetherFriendSelectionIds();
    }
  }
  $: selectedTogetherConnections = acceptedFriendConnections.filter((connection) =>
    selectedTogetherFriendIds.has(getFriendConnectionKey(connection))
  );
  $: raidTogetherRows = buildRaidTogetherRows(visibleRaids, selectedTogetherConnections, localSnapshot, remoteSnapshots, currentProfile, raidDifficultyFilters);
  $: activeProfileGroup = activeProfileGroup
    ? raidTogetherRows.flatMap((row) => row.groups).find((group) => group.key === activeProfileGroup?.key) || null
    : null;
  $: connectedFriends = acceptedFriendConnections.length;
  $: ownedGroupCount = meowGroups.filter((group) => group.role === 'owner').length;
  $: pendingGroupInvites = meowGroups.filter((group) => group.role === 'invited');
  $: assignableGroups = meowGroups.filter((group) => group.role !== 'invited');
  $: pendingIncoming = friendConnections.filter(
    (connection) => connection.status === 'pending' && connection.direction === 'incoming'
  );
  $: pendingRequestCount = pendingIncoming.length + pendingGroupInvites.length;
  $: if (pendingRequestCount !== lastDispatchedPendingRequestCount) {
    lastDispatchedPendingRequestCount = pendingRequestCount;
    dispatch('pendingRequestsChanged', pendingRequestCount);
  }
  $: filteredFriendOptions = friendOptions
    .filter((friend) => {
      const query = friendSearch.trim().toLowerCase();
      return query.length === 0 || friend.name.toLowerCase().includes(query) || friend.id.includes(query);
    })
    .slice(0, 7);

  onMount(() => {
    document.addEventListener('mousedown', handleDocumentMouseDown);
    clockTimer = setInterval(() => {
      currentTime = Date.now();
    }, 1000);

    void (async () => {
      consentAccepted = hasMeowConnectConsent();
      visibleRaidIds = loadVisibleRaidIds();
      raidDifficultyFilters = loadRaidDifficultyFilters();
      if (consentAccepted) {
        startRealtimeSubscription();
        await refreshMeowConnect();
      }
    })();

    return () => {
      unsubscribeRealtime?.();
      if (realtimeRefreshTimer) clearTimeout(realtimeRefreshTimer);
      if (groupInviteSearchTimer) clearTimeout(groupInviteSearchTimer);
      if (clockTimer) clearInterval(clockTimer);
      if (toastTimer) clearTimeout(toastTimer);
      document.removeEventListener('mousedown', handleDocumentMouseDown);
    };
  });

  async function acceptConsent() {
    setMeowConnectConsent(true);
    consentAccepted = true;
    startRealtimeSubscription();
    await refreshMeowConnect({ allowUpload: true, manual: true });
  }

  async function refreshMeowConnect(options: { allowUpload?: boolean; manual?: boolean } = {}) {
    const manual = Boolean(options.manual);
    const refreshReason = manual ? 'manual-sync' : options.allowUpload ? 'upload-refresh' : 'initial-load';
    if (manual && manualSyncBlocked) {
      showToast(`Manual sync was recent. Try again in ${formatDuration(manualSyncRemainingMs)}.`, 'info');
      logMeowConnectRequest(`Skipped ${refreshReason}; manual sync cooldown ${formatDuration(manualSyncRemainingMs)} remaining.`, 'info');
      return;
    }

    const startedAt = performance.now();
    logMeowConnectRequest(`Refresh started (${refreshReason}).`);
    isLoading = true;
    markMeowConnectConnecting(manual ? 'Syncing MeowConnect.' : 'Loading MeowConnect data.');

    try {
      currentProfile = await getCurrentSupabaseDiscordProfile();
      const snapshot = await loadMeowConnectLocalSnapshot();
      localSnapshot = snapshot;
      const uploadResult = manual || options.allowUpload
        ? await uploadMeowConnectSnapshotIfNeeded({ force: manual })
        : { snapshot, uploaded: false };
      localSnapshot = uploadResult.snapshot;
      friendConnections = await loadMeowConnectFriends();
      meowGroups = await loadAndMirrorMeowGroups();
      remoteSnapshots = await fetchMeowConnectRemoteSnapshots(String(uploadResult.snapshot.weeklyResetMs || 0));
      logMeowConnectRequest(
        `Refresh finished (${refreshReason}) in ${Math.round(performance.now() - startedAt)}ms: friends=${friendConnections.length}, groups=${meowGroups.length}, remoteSnapshots=${remoteSnapshots.length}, uploaded=${uploadResult.uploaded ? 'yes' : 'no'}.`,
        'info'
      );
      if (manual) {
        setStoredTimestamp(LAST_MANUAL_SYNC_STORAGE_KEY, Date.now());
        currentTime = Date.now();
      }
      if (uploadResult.duplicateCharacters?.length) {
        const duplicate = uploadResult.duplicateCharacters[0];
        const extra = uploadResult.duplicateCharacters.length > 1 ? ` +${uploadResult.duplicateCharacters.length - 1} more` : '';
        const syncedCount = uploadResult.uploadedCharacterCount ?? uploadResult.snapshot.characters.length;
        showToast(
          `Synced ${syncedCount} characters. Skipped ${duplicate.charName}${extra}: already exists under ${duplicate.ownerDisplayName}.`,
          'error'
        );
      } else {
        showToast(
          uploadResult.uploaded
            ? `Synced ${uploadResult.uploadedCharacterCount ?? uploadResult.snapshot.characters.length} characters.`
            : `Loaded MeowConnect. Last upload unchanged.`,
          uploadResult.uploaded ? 'success' : 'info'
        );
      }
      markMeowConnectActive(uploadResult.uploaded ? 'MeowConnect sync succeeded.' : 'MeowConnect is connected.');
    } catch (err) {
      logMeowConnectRequest(`Refresh failed (${refreshReason}): ${err}`, 'warn');
      markMeowConnectFailure(err);
      showToast(`Failed to load MeowConnect data: ${err}`, 'error');
    } finally {
      isLoading = false;
    }
  }

  function getStoredTimestamp(key: string): number {
    const value = Number(localStorage.getItem(key) || 0);
    return Number.isFinite(value) ? value : 0;
  }

  function setStoredTimestamp(key: string, value: number) {
    localStorage.setItem(key, String(value));
  }

  async function loadAndMirrorMeowGroups(): Promise<MeowConnectGroup[]> {
    const startedAt = performance.now();
    const groups = await loadMeowConnectGroups();
    await syncMeowConnectGroupTagsToLocal(groups);
    const assignmentCount = groups.reduce((count, group) => count + group.assignments.length, 0);
    logMeowConnectRequest(
      `Groups mirrored locally in ${Math.round(performance.now() - startedAt)}ms: groups=${groups.length}, assignments=${assignmentCount}.`
    );
    window.dispatchEvent(new CustomEvent('raid-settings-updated'));
    return groups;
  }

  function formatDuration(ms: number): string {
    const seconds = Math.max(0, Math.ceil(ms / 1000));
    const minutes = Math.floor(seconds / 60);
    const remainder = seconds % 60;
    return minutes > 0 ? `${minutes}m ${remainder}s` : `${remainder}s`;
  }

  function getFriendConnectionKey(connection: MeowConnectFriendConnection): string {
    return connection.profile.userId || connection.friendUserId || connection.profile.discordId || connection.userId;
  }

  function toggleTogetherFriend(connection: MeowConnectFriendConnection) {
    const key = getFriendConnectionKey(connection);
    const next = new Set(selectedTogetherFriendIds);
    if (next.has(key)) {
      next.delete(key);
    } else {
      next.add(key);
    }
    selectedTogetherFriendIds = next;
    saveTogetherFriendSelectionIds();
  }

  function groupRowsByProfile(raidId: string, rows: MeowConnectAvailabilityRow[]): ProfileRaidGroup[] {
    const groups = new Map<string, ProfileRaidGroup>();
    for (const row of rows) {
      const key = `${raidId}:${row.ownerId}`;
      const group = groups.get(key) || {
        key,
        raidId,
        ownerId: row.ownerId,
        ownerName: row.ownerName,
        ownerAvatarUrl: row.ownerAvatarUrl,
        rows: [],
        openCount: 0,
        clearedCount: 0,
        reservedCount: 0,
        favoriteCount: 0,
        raidName: row.raid.name,
        minIlvl: row.raid.gates[0]?.minIlvl || 0
      };
      group.rows.push(row);
      if (isAvailableRow(row)) group.openCount += 1;
      if (row.status === 'cleared') group.clearedCount += 1;
      if (row.status === 'open' && row.reservedForStatic && row.ownerId !== 'local' && !hasSharedGroupAssignment(row)) group.reservedCount += 1;
      if (row.favorite) group.favoriteCount += 1;
      groups.set(key, group);
    }

    return Array.from(groups.values()).sort(sortProfileGroups);
  }

  function sortProfileGroups(a: ProfileRaidGroup, b: ProfileRaidGroup): number {
    const aLocal = a.ownerId === 'local';
    const bLocal = b.ownerId === 'local';
    if (aLocal !== bLocal) return aLocal ? -1 : 1;

    return a.ownerName.localeCompare(b.ownerName, undefined, { sensitivity: 'base' });
  }

  function openProfileGroup(group: ProfileRaidGroup) {
    activeProfileGroup = group;
  }

  function closeProfileGroup() {
    activeProfileGroup = null;
  }

  function buildRaidTogetherRows(
    raids: Raid[],
    selectedConnections: MeowConnectFriendConnection[],
    currentLocalSnapshot: MeowConnectLocalSnapshot | null,
    currentRemoteSnapshots: MeowConnectRemoteSnapshot[],
    profile: MeowConnectProfile | null,
    difficultyFilters: Record<string, string>
  ): RaidTogetherRow[] {
    if (selectedConnections.length === 0) return [];

    return raids.flatMap((raid) => {
      const allRows = buildMeowConnectAvailabilityRows(currentLocalSnapshot, currentRemoteSnapshots, raid.id, getRaidDifficultyFilter(raid.id, difficultyFilters), new Set(), profile);
      const rows = allRows.filter(isAvailableRow);
      const myOpenCount = rows.filter((row) => row.ownerId === 'local').length;
      const participantCounts = selectedConnections.map((connection) => ({
        name: connection.profile.displayName,
        count: rows.filter((row) => isFriendAvailabilityRow(row, connection)).length
      }));
      const togetherCount = Math.min(myOpenCount, ...participantCounts.map((participant) => participant.count));
      const includedRows = allRows.filter((row) =>
        row.ownerId === 'local' ||
        selectedConnections.some((connection) => isFriendAvailabilityRow(row, connection))
      );

      return [{
        key: `${raid.id}:${selectedConnections.map(getFriendConnectionKey).sort().join('+')}`,
        raidName: raid.name,
        minIlvl: raid.gates[0].minIlvl,
        togetherCount,
        groups: groupRowsByProfile(raid.id, includedRows)
      }];
    });
  }

  function isAvailableRow(row: MeowConnectAvailabilityRow): boolean {
    return row.status === 'open' && (!row.reservedForStatic || row.ownerId === 'local' || hasSharedGroupAssignment(row));
  }

  function getRaidDifficultyOptions(raidId: string): string[] {
    return getMeowConnectRaidDifficulties(raidId).filter((difficulty) => difficulty.toLowerCase() !== 'solo');
  }

  function getRaidDifficultyFilter(raidId: string, filters = raidDifficultyFilters): string {
    const options = getRaidDifficultyOptions(raidId);
    const selected = filters[raidId];
    if (selected && options.includes(selected)) return selected;
    return 'all';
  }

  function setRaidDifficultyFilter(raidId: string, difficulty: string) {
    if (!getRaidDifficultyOptions(raidId).includes(difficulty)) return;
    const nextFilters = { ...raidDifficultyFilters };
    if (nextFilters[raidId] === difficulty) {
      delete nextFilters[raidId];
    } else {
      nextFilters[raidId] = difficulty;
    }
    raidDifficultyFilters = nextFilters;
    saveRaidDifficultyFilters();
    activeProfileGroup = null;
  }

  function isFriendAvailabilityRow(row: MeowConnectAvailabilityRow, connection: MeowConnectFriendConnection): boolean {
    const ownerIds = [row.ownerId, row.character.rosterId].map(normalizeId).filter(Boolean);
    const friendIds = [
      connection.profile.discordId,
      connection.profile.userId,
      connection.friendUserId,
      connection.userId
    ].map(normalizeId).filter(Boolean);

    return ownerIds.some((ownerId) => friendIds.includes(ownerId)) ||
      normalizeName(row.ownerName) === normalizeName(connection.profile.displayName);
  }

  function normalizeId(value?: string | null): string {
    return String(value || '').trim().toLowerCase();
  }

  function normalizeName(value?: string | null): string {
    return String(value || '').trim().toLowerCase();
  }

  function toggleRaidVisibility(raidId: string) {
    visibleRaidIds = visibleRaidIds.includes(raidId)
      ? visibleRaidIds.filter((id) => id !== raidId)
      : raidOptions.filter((raid) => raid.id === raidId || visibleRaidIds.includes(raid.id)).map((raid) => raid.id);
    saveVisibleRaidIds();
  }

  function selectAllRaids() {
    visibleRaidIds = raidOptions.map((raid) => raid.id);
    saveVisibleRaidIds();
  }

  function clearRaidSelection() {
    visibleRaidIds = [];
    saveVisibleRaidIds();
  }

  function loadVisibleRaidIds(): string[] {
    try {
      const stored = localStorage.getItem(RAID_VISIBILITY_STORAGE_KEY);
      if (stored === null) return raidOptions.map((raid) => raid.id);

      const values = JSON.parse(stored);
      const ids = Array.isArray(values) ? values.map((value) => String(value)).filter(Boolean) : [];
      return ids.filter((id) => raidOptions.some((raid) => raid.id === id));
    } catch {
      return raidOptions.map((raid) => raid.id);
    }
  }

  function saveVisibleRaidIds() {
    localStorage.setItem(RAID_VISIBILITY_STORAGE_KEY, JSON.stringify(visibleRaidIds));
  }

  function loadRaidDifficultyFilters(): Record<string, string> {
    try {
      const stored = localStorage.getItem(RAID_DIFFICULTY_FILTER_STORAGE_KEY);
      const parsed = stored ? JSON.parse(stored) : {};
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return {};

      return Object.fromEntries(
        Object.entries(parsed)
          .map(([raidId, difficulty]) => [raidId, String(difficulty)])
          .filter(([raidId, difficulty]) =>
            raidOptions.some((raid) => raid.id === raidId) &&
            getRaidDifficultyOptions(raidId).includes(difficulty)
          )
      );
    } catch {
      return {};
    }
  }

  function saveRaidDifficultyFilters() {
    localStorage.setItem(RAID_DIFFICULTY_FILTER_STORAGE_KEY, JSON.stringify(raidDifficultyFilters));
  }

  function loadTogetherFriendSelectionIds(connections: MeowConnectFriendConnection[]): Set<string> {
    try {
      const stored = localStorage.getItem(TOGETHER_FRIEND_SELECTION_STORAGE_KEY);
      if (stored === null) return new Set();

      const acceptedKeys = new Set(connections.map(getFriendConnectionKey));
      const values = JSON.parse(stored);
      const ids = Array.isArray(values) ? values.map((value) => String(value)).filter(Boolean) : [];
      return new Set(ids.filter((id) => acceptedKeys.has(id)));
    } catch {
      return new Set();
    }
  }

  function saveTogetherFriendSelectionIds() {
    localStorage.setItem(TOGETHER_FRIEND_SELECTION_STORAGE_KEY, JSON.stringify(Array.from(selectedTogetherFriendIds)));
  }

  async function openFriendPopover() {
    showFriendPopover = true;
    if (friendOptions.length > 0) return;

    try {
      friendOptions = await loadDiscordWhitelistMembers();
    } catch (err) {
      console.warn('Failed to load whitelist names:', err);
    }
  }

  function selectFriendOption(friend: FriendOption) {
    friendSearch = friend.name;
    friendDiscordId = friend.id;
  }

  async function sendFriendRequest() {
    friendActionBusy = true;

    try {
      const profile = await sendMeowConnectFriendRequest(friendDiscordId || friendSearch);
      friendSearch = '';
      friendDiscordId = '';
      showFriendPopover = false;
      friendConnections = await loadMeowConnectFriends();
      markMeowConnectActive('MeowConnect is connected.');
      showToast(`Friend request sent to ${profile.displayName}.`, 'success');
    } catch (err) {
      markMeowConnectFailure(err);
      showToast(`Failed to send friend request: ${err}`, 'error');
    } finally {
      friendActionBusy = false;
    }
  }

  async function acceptFriendRequest(connection: MeowConnectFriendConnection) {
    friendActionBusy = true;

    try {
      await acceptMeowConnectFriendRequest(connection.userId);
      friendConnections = await loadMeowConnectFriends();
      remoteSnapshots = localSnapshot
        ? await fetchMeowConnectRemoteSnapshots(String(localSnapshot.weeklyResetMs || 0))
        : remoteSnapshots;
      markMeowConnectActive('MeowConnect is connected.');
      showToast(`Friend request accepted for ${connection.profile.displayName}.`, 'success');
    } catch (err) {
      markMeowConnectFailure(err);
      showToast(`Failed to accept friend request: ${err}`, 'error');
    } finally {
      friendActionBusy = false;
    }
  }

  async function removeFriend(connection: MeowConnectFriendConnection) {
    friendActionBusy = true;

    try {
      await removeMeowConnectFriend(connection);
      friendConnections = await loadMeowConnectFriends();
      remoteSnapshots = localSnapshot
        ? await fetchMeowConnectRemoteSnapshots(String(localSnapshot.weeklyResetMs || 0))
        : remoteSnapshots;
      markMeowConnectActive('MeowConnect is connected.');
      showToast(`${connection.profile.displayName} removed.`, 'success');
    } catch (err) {
      markMeowConnectFailure(err);
      showToast(`Failed to update friend connection: ${err}`, 'error');
    } finally {
      friendActionBusy = false;
    }
  }

  async function createGroup() {
    groupActionBusy = true;

    try {
      await createMeowConnectGroup(groupName, groupTag);
      groupName = '';
      groupTag = '';
      meowGroups = await loadAndMirrorMeowGroups();
      markMeowConnectActive('MeowConnect is connected.');
      showToast('Group created.', 'success');
    } catch (err) {
      markMeowConnectFailure(err);
      showToast(`Failed to create group: ${err}`, 'error');
    } finally {
      groupActionBusy = false;
    }
  }

  async function inviteGroupMember(group: MeowConnectGroup) {
    const discordId = groupInviteInputs[group.groupId] || '';
    groupActionBusy = true;

    try {
      await inviteMeowConnectGroupMember(group.groupId, discordId);
      groupInviteInputs = { ...groupInviteInputs, [group.groupId]: '' };
      meowGroups = await loadAndMirrorMeowGroups();
      markMeowConnectActive('MeowConnect is connected.');
      showToast(`Invite sent for ${group.groupName}.`, 'success');
    } catch (err) {
      markMeowConnectFailure(err);
      showToast(`Failed to invite group member: ${err}`, 'error');
    } finally {
      groupActionBusy = false;
    }
  }

  async function renameGroup(group: MeowConnectGroup) {
    const nextName = (groupRenameInputs[group.groupId] ?? group.groupName).trim();
    if (nextName === group.groupName) return;
    groupActionBusy = true;

    try {
      await renameMeowConnectGroup(group.groupId, nextName);
      groupRenameInputs = { ...groupRenameInputs, [group.groupId]: '' };
      activeGroupRenameId = '';
      meowGroups = await loadAndMirrorMeowGroups();
      markMeowConnectActive('MeowConnect is connected.');
      showToast(`Renamed group to ${nextName}.`, 'success');
    } catch (err) {
      markMeowConnectFailure(err);
      showToast(`Failed to rename group: ${err}`, 'error');
    } finally {
      groupActionBusy = false;
    }
  }

  function startGroupRename(group: MeowConnectGroup) {
    activeGroupRenameId = group.groupId;
    groupRenameInputs = { ...groupRenameInputs, [group.groupId]: group.groupName };
  }

  function cancelGroupRename(group: MeowConnectGroup) {
    activeGroupRenameId = '';
    groupRenameInputs = { ...groupRenameInputs, [group.groupId]: group.groupName };
  }

  async function deleteGroup(group: MeowConnectGroup) {
    groupActionBusy = true;

    try {
      await deleteMeowConnectGroup(group.groupId);
      meowGroups = await loadAndMirrorMeowGroups();
      markMeowConnectActive('MeowConnect is connected.');
      showToast(`${group.groupName} deleted.`, 'success');
    } catch (err) {
      markMeowConnectFailure(err);
      showToast(`Failed to delete group: ${err}`, 'error');
    } finally {
      groupActionBusy = false;
    }
  }

  async function leaveGroup(group: MeowConnectGroup) {
    groupActionBusy = true;

    try {
      await leaveMeowConnectGroup(group.groupId);
      meowGroups = await loadAndMirrorMeowGroups();
      markMeowConnectActive('MeowConnect is connected.');
      showToast(`Left ${group.groupName}.`, 'success');
    } catch (err) {
      markMeowConnectFailure(err);
      showToast(`Failed to leave group: ${err}`, 'error');
    } finally {
      groupActionBusy = false;
    }
  }

  function scheduleGroupInviteProfileSearch(groupId: string, query: string) {
    activeGroupInviteId = groupId;
    if (groupInviteSearchTimer) clearTimeout(groupInviteSearchTimer);

    const cleanQuery = query.trim();
    if (cleanQuery.length === 0) {
      groupInviteOptions = [];
      return;
    }

    groupInviteSearchTimer = setTimeout(async () => {
      try {
        groupInviteOptions = await searchMeowConnectProfiles(cleanQuery);
      } catch (err) {
        console.warn('Failed to search MeowConnect profiles:', err);
        groupInviteOptions = [];
      }
    }, 180);
  }

  function selectGroupInviteOption(group: MeowConnectGroup, profile: MeowConnectProfile) {
    groupInviteInputs = { ...groupInviteInputs, [group.groupId]: profile.discordId };
    activeGroupInviteId = group.groupId;
    groupInviteOptions = [];
  }

  function hasSharedGroupAssignment(row: MeowConnectAvailabilityRow): boolean {
    return getAssignedGroupNames(row).length > 0;
  }

  function getGroupAssignmentKey(row: MeowConnectAvailabilityRow): string {
    return `${row.character.charId}:${row.raid.id}`;
  }

  function getRowGroupAssignments(row: MeowConnectAvailabilityRow) {
    const ownerUserId = getRowOwnerUserId(row);
    return assignableGroups.flatMap((group) =>
      group.assignments
        .filter((assignment) =>
          (!ownerUserId || assignment.userId === ownerUserId) &&
          assignment.charId === row.character.charId &&
          assignment.contentId === row.raid.id
        )
        .map((assignment) => ({ group, assignment }))
    );
  }

  function getRowOwnerUserId(row: MeowConnectAvailabilityRow): string {
    if (row.ownerId === 'local') return currentProfile?.userId || row.ownerUserId || '';
    return row.ownerUserId || '';
  }

  function getAssignedGroupNames(row: MeowConnectAvailabilityRow): string[] {
    return getRowGroupAssignments(row).map(({ group }) => group.groupName);
  }

  async function changeRowGroupAssignment(row: MeowConnectAvailabilityRow, groupId: string) {
    const busyKey = getGroupAssignmentKey(row);
    groupAssignmentBusyKey = busyKey;

    try {
      const existingAssignments = getRowGroupAssignments(row);
      await Promise.all(existingAssignments.map(({ assignment }) =>
        removeMeowConnectRaidGroupAssignment({
          groupId: assignment.groupId,
          charId: row.character.charId,
          contentId: row.raid.id
        })
      ));

      if (groupId) {
        await assignMeowConnectRaidToGroup({
          groupId,
          rosterId: row.character.rosterId,
          charId: row.character.charId,
          contentId: row.raid.id,
          difficulty: '',
          reservedForStatic: true
        });
      }

      meowGroups = await loadAndMirrorMeowGroups();
      showToast(groupId ? `Assigned ${row.character.charName} to group.` : `Removed group assignment for ${row.character.charName}.`, 'success');
      markMeowConnectActive('MeowConnect is connected.');
    } catch (err) {
      markMeowConnectFailure(err);
      showToast(`Failed to update group assignment: ${err}`, 'error');
    } finally {
      groupAssignmentBusyKey = '';
    }
  }

  async function acceptGroupInvite(group: MeowConnectGroup) {
    groupActionBusy = true;

    try {
      await acceptMeowConnectGroupInvite(group.groupId);
      meowGroups = await loadAndMirrorMeowGroups();
      markMeowConnectActive('MeowConnect is connected.');
      showToast(`Joined ${group.groupName}.`, 'success');
    } catch (err) {
      markMeowConnectFailure(err);
      showToast(`Failed to accept group invite: ${err}`, 'error');
    } finally {
      groupActionBusy = false;
    }
  }

  function getGroupMemberName(member: MeowConnectGroupMember): string {
    return member.profile?.displayName || (member.userId === currentProfile?.userId ? 'You' : 'Unknown');
  }

  function handleDocumentMouseDown(event: MouseEvent) {
    if (!showFriendPopover) return;
    const target = event.target as Node | null;
    if (target && friendPopoverEl?.contains(target)) return;
    showFriendPopover = false;
  }

  function showToast(message: string, kind: 'success' | 'error' | 'info') {
    toastMessage = message;
    toastKind = kind;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toastMessage = '';
    }, kind === 'error' ? 5000 : 3500);
  }

  function startRealtimeSubscription() {
    if (unsubscribeRealtime) return;
    if (!isMeowConnectRealtimeEnabled()) return;

    logMeowConnectRequest('Realtime subscription starting.');
    unsubscribeRealtime = subscribeMeowConnectChanges(() => {
      if (realtimeRefreshTimer) clearTimeout(realtimeRefreshTimer);

      realtimeChangeBurstCount += 1;
      realtimeRefreshTimer = setTimeout(() => {
        void refreshRemoteMeowConnectData();
      }, 1500);
    });
  }

  async function refreshRemoteMeowConnectData() {
    if (!localSnapshot) return;
    const startedAt = performance.now();
    const changeCount = realtimeChangeBurstCount;
    realtimeChangeBurstCount = 0;
    logMeowConnectRequest(`Remote refresh started (realtime, ${changeCount} change${changeCount === 1 ? '' : 's'}).`);

    try {
      friendConnections = await loadMeowConnectFriends();
      meowGroups = await loadAndMirrorMeowGroups();
      remoteSnapshots = await fetchMeowConnectRemoteSnapshots(String(localSnapshot.weeklyResetMs || 0));
      logMeowConnectRequest(
        `Remote refresh finished (realtime) in ${Math.round(performance.now() - startedAt)}ms: friends=${friendConnections.length}, groups=${meowGroups.length}, remoteSnapshots=${remoteSnapshots.length}.`,
        'info'
      );
      markMeowConnectActive('MeowConnect realtime refresh succeeded.');
    } catch (err) {
      logMeowConnectRequest(`Remote refresh failed (realtime): ${err}`, 'warn');
      markMeowConnectFailure(err);
      console.warn('Failed to refresh MeowConnect realtime data:', err);
    }
  }

  function getInitials(name: string): string {
    const parts = name.trim().split(/\s+/).filter(Boolean);
    return (parts[0]?.[0] || '?').toUpperCase() + (parts[1]?.[0] || '').toUpperCase();
  }

  function getProfileAvatar(discordId: string): string | undefined {
    return friendConnections.find((connection) => connection.profile.discordId === discordId)?.profile.avatarUrl;
  }

  function formatResetTime(timestamp: number) {
    if (!timestamp) return 'unknown';
    return new Date(timestamp).toLocaleString();
  }

  function formatLastSyncTime(timestamp: number, _now: number) {
    if (!timestamp) {
      return {
        value: 'Never',
        caption: 'last synced'
      };
    }

    return {
      value: new Date(timestamp).toLocaleString([], {
        month: 'short',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        hour12: false
      }),
      caption: 'last synced'
    };
  }

  function sortFriendConnections(a: MeowConnectFriendConnection, b: MeowConnectFriendConnection): number {
    const aIncoming = a.status === 'pending' && a.direction === 'incoming';
    const bIncoming = b.status === 'pending' && b.direction === 'incoming';
    if (aIncoming !== bIncoming) return aIncoming ? -1 : 1;
    return getFriendDisplayName(a).localeCompare(getFriendDisplayName(b), undefined, { sensitivity: 'base' });
  }

  function getFriendDisplayName(connection: MeowConnectFriendConnection): string {
    return connection.profile.displayName || connection.profile.userId || '';
  }

</script>

<section class="meow-connect" data-guide="meow-connect">
  {#if !consentAccepted}
    <header class="mc-header">
      <div class="mc-title">
        <img src={meowConnectIcon} alt="" />
        <h2>MeowConnect</h2>
      </div>
    </header>

    <section class="consent-panel" data-guide="meow-connect-consent">
      <div>
        <h3>Share character availability</h3>
        <p>
          MeowConnect syncs only characters you marked with the Connect toggle, plus their raid completion and
          static/friend reservation state.
        </p>
      </div>
      <button class="primary-button" type="button" on:click={acceptConsent}>Agree and sync</button>
    </section>
  {:else}
    {#if toastMessage}
      <div class:success={toastKind === 'success'} class:error={toastKind === 'error'} class:info={toastKind === 'info'} class="mc-toast">
        <span>{toastMessage}</span>
      </div>
    {/if}

    {#if activeSection === 'together'}
      <MeowConnectTogether
        {acceptedFriendConnections}
        {connectedFriends}
        {getFriendConnectionKey}
        {getInitials}
        {localSnapshot}
        {raidDifficultyFilterItems}
        {raidTogetherRows}
        {selectedTogetherConnections}
        {selectedTogetherFriendIds}
        {visibleRaids}
        on:openProfileGroup={(event) => openProfileGroup(event.detail)}
        on:setRaidDifficultyFilter={(event) => setRaidDifficultyFilter(event.detail.raidId, event.detail.difficulty)}
        on:toggleTogetherFriend={(event) => toggleTogetherFriend(event.detail)}
      />

      {#if activeProfileGroup}
        <MeowConnectProfilePopover
          profileGroup={activeProfileGroup}
          {assignableGroups}
          {currentProfile}
          {groupAssignmentBusyKey}
          {raidDifficultyFilters}
          {getInitials}
          on:close={closeProfileGroup}
          on:changeGroupAssignment={(event) => changeRowGroupAssignment(event.detail.row, event.detail.groupId)}
        />
      {/if}
    {:else if activeSection === 'logs'}
      <MeowConnectLogs
        {logEntries}
        {localSnapshot}
        {remoteSnapshots}
        {currentProfile}
      />
    {:else}
      <MeowConnectSettings
        bind:activeGroupInviteId
        bind:activeGroupRenameId
        bind:friendDiscordId
        bind:friendSearch
        bind:groupInviteInputs
        bind:groupName
        bind:groupRenameInputs
        bind:groupTag
        bind:popoverElement={friendPopoverEl}
        bind:showFriendPopover
        resetText={localSnapshot ? `Current weekly reset started ${formatResetTime(localSnapshot.weeklyResetMs)}` : 'No local snapshot loaded.'}
        syncLabel={manualSyncLabel}
        syncTitle={manualSyncBlocked ? `Manual sync is available in ${formatDuration(manualSyncRemainingMs)}.` : $meowConnectHasUnsyncedChanges ? 'Upload unsynced MeowConnect changes' : 'Upload your current MeowConnect snapshot'}
        syncDisabled={isLoading || manualSyncBlocked}
        {connectedCharacterCount}
        {connectedFriends}
        {consentAccepted}
        {filteredFriendOptions}
        {friendActionBusy}
        {friendConnections}
        {getGroupMemberName}
        {getInitials}
        {getProfileAvatar}
        {groupActionBusy}
        {groupInviteOptions}
        {meowGroups}
        {ownedGroupCount}
        {pendingGroupInvites}
        {pendingIncoming}
        {raidOptions}
        {sortedFriendConnections}
        {unsyncedRosterChangeCount}
        {visibleRaidIds}
        hasUnsyncedChanges={$meowConnectHasUnsyncedChanges}
        lastSyncValue={lastSyncLabel.value}
        lastSyncCaption={lastSyncLabel.caption}
        on:acceptFriendRequest={(event) => acceptFriendRequest(event.detail)}
        on:acceptGroupInvite={(event) => acceptGroupInvite(event.detail)}
        on:cancelGroupRename={(event) => cancelGroupRename(event.detail)}
        on:clearRaidSelection={clearRaidSelection}
        on:createGroup={createGroup}
        on:deleteGroup={(event) => deleteGroup(event.detail)}
        on:inviteGroupMember={(event) => inviteGroupMember(event.detail)}
        on:leaveGroup={(event) => leaveGroup(event.detail)}
        on:openFriendPopover={openFriendPopover}
        on:removeFriend={(event) => removeFriend(event.detail)}
        on:renameGroup={(event) => renameGroup(event.detail)}
        on:scheduleInviteSearch={(event) => scheduleGroupInviteProfileSearch(event.detail.groupId, event.detail.query)}
        on:selectAllRaids={selectAllRaids}
        on:selectFriendOption={(event) => selectFriendOption(event.detail)}
        on:selectInviteOption={(event) => selectGroupInviteOption(event.detail.group, event.detail.profile)}
        on:sendFriendRequest={sendFriendRequest}
        on:startGroupRename={(event) => startGroupRename(event.detail)}
        on:sync={() => refreshMeowConnect({ allowUpload: true, manual: true })}
        on:toggleRaidVisibility={(event) => toggleRaidVisibility(event.detail)}
      />
    {/if}
  {/if}
</section>

<style>
  .meow-connect {
    --app-control-accent: var(--app-meowconnect-accent);
    --app-control-on-accent: var(--md-sys-color-on-primary);
    --app-control-accent-container: var(--app-meowconnect-accent-soft);
    --app-control-hover-border: var(--app-meowconnect-accent);
    width: min(1280px, 100%);
    margin: 0 auto;
    padding: 0.65rem 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    box-sizing: border-box;
  }

  .mc-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .mc-header {
    position: sticky;
    top: 0;
    z-index: 10;
    padding: 0.35rem 0;
    background: var(--md-sys-color-background);
  }

  .mc-title {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }

  .mc-title img {
    width: 28px;
    height: 28px;
    object-fit: contain;
    display: block;
  }

  h2,
  h3,
  p {
    margin: 0;
  }

  h2 {
    color: var(--md-sys-color-on-surface);
    font-size: 1.18rem;
    line-height: 1.1;
  }

  h3 {
    color: var(--md-sys-color-on-surface);
    font-size: 0.88rem;
  }

  .primary-button {
    border: 0;
    border-radius: 6px;
    color: var(--md-sys-color-on-surface);
    background: transparent;
    font: inherit;
    font-size: 0.74rem;
    font-weight: 800;
    cursor: pointer;
    white-space: nowrap;
  }

  .primary-button {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
  }

  .primary-button {
    padding: 0.42rem 0.58rem;
  }

  .primary-button:disabled {
    cursor: default;
    opacity: 0.6;
  }

  .mc-toast {
    position: fixed;
    right: 1.5rem;
    bottom: 1.5rem;
    z-index: 1000;
    max-width: min(420px, calc(100vw - 2rem));
    padding: 0.75rem 0.95rem;
    border-radius: 8px;
    color: var(--md-sys-color-on-primary);
    background: var(--md-sys-color-primary);
    box-shadow: 0 4px 12px color-mix(in srgb, black 20%, transparent);
    font-size: 0.84rem;
    font-weight: 700;
    animation: slideIn 0.25s ease;
  }

  .mc-toast.success {
    background: var(--md-sys-color-tertiary);
    color: var(--md-sys-color-on-tertiary);
  }

  .mc-toast.error {
    background: var(--md-sys-color-error);
    color: var(--md-sys-color-on-error);
  }

  @keyframes slideIn {
    from {
      transform: translateY(100%);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  @media (max-width: 760px) {
    .meow-connect {
      padding: 0.75rem;
    }

    .mc-header,
    .consent-panel {
      display: grid;
      grid-template-columns: 1fr;
    }

  }
</style>
