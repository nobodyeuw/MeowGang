<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { GAME_CLASSES } from '$lib/data/classes';
  import type { Raid } from '$lib/data/raids';
  import {
    acceptMeowConnectFriendRequest,
    buildMeowConnectAvailabilityRows,
    buildMeowConnectLogEntries,
    fetchMeowConnectRemoteSnapshots,
    getMeowConnectRaidOptions,
    hasMeowConnectConsent,
    isMeowConnectRealtimeEnabled,
    loadFavoritePlayerIds,
    loadMeowConnectFriends,
    loadMeowConnectLocalSnapshot,
    markMeowConnectActive,
    markMeowConnectConnecting,
    markMeowConnectFailure,
    meowConnectHasUnsyncedChanges,
    removeMeowConnectFriend,
    sendMeowConnectFriendRequest,
    setMeowConnectStaticFriend,
    setMeowConnectConsent,
    subscribeMeowConnectChanges,
    toggleFavoritePlayerId,
    uploadMeowConnectSnapshotIfNeeded,
    type MeowConnectAvailabilityRow,
    type MeowConnectFriendConnection,
    type MeowConnectLogEntry,
    type MeowConnectLocalSnapshot,
    type MeowConnectProfile,
    type MeowConnectRemoteSnapshot
  } from '$lib/services/meow-connect';
  import { getCurrentSupabaseDiscordProfile } from '$lib/services/supabase-auth';

  interface FriendOption {
    id: string;
    name: string;
  }

  type MeowConnectTab = 'together' | 'logs' | 'settings';
  type AvailabilityMode = 'open' | 'favorites';
  type RaidTogetherView = 'availability' | 'pairs';

  interface ProfileRaidGroup {
    key: string;
    ownerId: string;
    ownerName: string;
    ownerAvatarUrl?: string;
    rows: MeowConnectAvailabilityRow[];
    openCount: number;
    clearedCount: number;
    favoriteCount: number;
    raidName: string;
    minIlvl: number;
  }

  interface RaidTogetherRow {
    key: string;
    raidName: string;
    minIlvl: number;
    friendNames: string;
    friendAvatars: { name: string; avatarUrl?: string }[];
    myOpenCount: number;
    participantCounts: { name: string; count: number }[];
    togetherCount: number;
  }

  const RAID_VISIBILITY_STORAGE_KEY = 'meowConnect.visibleRaidIds';
  const AVAILABILITY_MODE_STORAGE_KEY = 'meowConnect.availabilityMode';
  const LAST_MANUAL_SYNC_STORAGE_KEY = 'meowConnect.lastManualSyncAt';
  const MANUAL_SYNC_COOLDOWN_MS = 5 * 60 * 1000;
  const raidOptions = getMeowConnectRaidOptions()
    .sort((a, b) => a.gates[0].minIlvl - b.gates[0].minIlvl || a.name.localeCompare(b.name));

  export let activeSection: MeowConnectTab = 'together';

  let consentAccepted = false;
  let availabilityMode: AvailabilityMode = 'open';
  let raidTogetherView: RaidTogetherView = 'availability';
  let visibleRaidIds = raidOptions.map((raid) => raid.id);
  let focusedAvailabilityRaidIds = new Set<string>();
  let activeProfileGroup: ProfileRaidGroup | null = null;
  let friendPopoverEl: HTMLElement | null = null;
  let currentProfile: MeowConnectProfile | null = null;
  let localSnapshot: MeowConnectLocalSnapshot | null = null;
  let remoteSnapshots: MeowConnectRemoteSnapshot[] = [];
  let friendConnections: MeowConnectFriendConnection[] = [];
  let favoriteIds = new Set<string>();
  let selectedTogetherFriendIds = new Set<string>();
  let togetherFriendSelectionInitialized = false;
  let friendOptions: FriendOption[] = [];
  let friendSearch = '';
  let friendDiscordId = '';
  let showFriendPopover = false;
  let isLoading = false;
  let friendActionBusy = false;
  let currentTime = Date.now();
  let toastMessage = '';
  let toastKind: 'success' | 'error' | 'info' = 'info';
  let unsubscribeRealtime: (() => void) | null = null;
  let realtimeRefreshTimer: ReturnType<typeof setTimeout> | null = null;
  let clockTimer: ReturnType<typeof setInterval> | null = null;
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  $: manualSyncRemainingMs = Math.max(0, getStoredTimestamp(LAST_MANUAL_SYNC_STORAGE_KEY) + MANUAL_SYNC_COOLDOWN_MS - currentTime);
  $: manualSyncBlocked = manualSyncRemainingMs > 0;
  $: manualSyncLabel = isLoading
    ? 'Syncing...'
    : manualSyncBlocked
      ? `Sync in ${formatDuration(manualSyncRemainingMs)}`
      : $meowConnectHasUnsyncedChanges
        ? 'Sync changes'
        : 'Sync now';
  $: visibleRaids = raidOptions.filter((raid) => visibleRaidIds.includes(raid.id));
  $: raidSections = visibleRaids.map((raid) => ({
    raid,
    rows: filterRows(buildMeowConnectAvailabilityRows(localSnapshot, remoteSnapshots, raid.id, 'all', favoriteIds, currentProfile)),
    groups: groupRowsByProfile(
      raid.id,
      filterRows(buildMeowConnectAvailabilityRows(localSnapshot, remoteSnapshots, raid.id, 'all', favoriteIds, currentProfile))
    )
  }));
  $: if (focusedAvailabilityRaidIds.size > 0) {
    const validFocusedIds = new Set(Array.from(focusedAvailabilityRaidIds).filter((raidId) => visibleRaidIds.includes(raidId)));
    if (validFocusedIds.size !== focusedAvailabilityRaidIds.size) focusedAvailabilityRaidIds = validFocusedIds;
  }
  $: displayedRaidSections = focusedAvailabilityRaidIds.size > 0
    ? raidSections.filter((section) => focusedAvailabilityRaidIds.has(section.raid.id))
    : raidSections;
  $: logEntries = buildMeowConnectLogEntries(localSnapshot, remoteSnapshots, visibleRaidIds, currentProfile).slice(0, 80);
  $: acceptedFriendConnections = friendConnections.filter((connection) => connection.status === 'accepted');
  $: if (!togetherFriendSelectionInitialized && acceptedFriendConnections.length > 0) {
    selectedTogetherFriendIds = new Set(acceptedFriendConnections.map(getFriendConnectionKey));
    togetherFriendSelectionInitialized = true;
  }
  $: selectedTogetherConnections = acceptedFriendConnections.filter((connection) =>
    selectedTogetherFriendIds.has(getFriendConnectionKey(connection))
  );
  $: raidTogetherRows = buildRaidTogetherRows(visibleRaids, selectedTogetherConnections, localSnapshot, remoteSnapshots, favoriteIds, currentProfile);
  $: activeProfileGroup = activeProfileGroup
    ? displayedRaidSections.flatMap((section) => section.groups).find((group) => group.key === activeProfileGroup?.key) || null
    : null;
  $: totalOpenCount = raidSections.reduce(
    (total, section) => total + section.rows.filter((row) => row.status === 'open').length,
    0
  );
  $: connectedFriends = acceptedFriendConnections.length;
  $: pendingIncoming = friendConnections.filter(
    (connection) => connection.status === 'pending' && connection.direction === 'incoming'
  );
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
      favoriteIds = loadFavoritePlayerIds();
      visibleRaidIds = loadVisibleRaidIds();
      availabilityMode = loadAvailabilityMode();
      if (consentAccepted) {
        startRealtimeSubscription();
        await refreshMeowConnect();
      }
    })();

    return () => {
      unsubscribeRealtime?.();
      if (realtimeRefreshTimer) clearTimeout(realtimeRefreshTimer);
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
    if (manual && manualSyncBlocked) {
      showToast(`Manual sync was recent. Try again in ${formatDuration(manualSyncRemainingMs)}.`, 'info');
      return;
    }

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
      remoteSnapshots = await fetchMeowConnectRemoteSnapshots(String(uploadResult.snapshot.weeklyResetMs || 0));
      if (manual) {
        setStoredTimestamp(LAST_MANUAL_SYNC_STORAGE_KEY, Date.now());
        currentTime = Date.now();
      }
      showToast(
        uploadResult.uploaded
          ? `Synced ${uploadResult.snapshot.characters.length} characters.`
          : `Loaded MeowConnect. Last upload unchanged.`,
        uploadResult.uploaded ? 'success' : 'info'
      );
      markMeowConnectActive(uploadResult.uploaded ? 'MeowConnect sync succeeded.' : 'MeowConnect is connected.');
    } catch (err) {
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

  function formatDuration(ms: number): string {
    const seconds = Math.max(0, Math.ceil(ms / 1000));
    const minutes = Math.floor(seconds / 60);
    const remainder = seconds % 60;
    return minutes > 0 ? `${minutes}m ${remainder}s` : `${remainder}s`;
  }

  function filterRows(rows: MeowConnectAvailabilityRow[]): MeowConnectAvailabilityRow[] {
    return rows.filter((row) =>
      (availabilityMode !== 'open' || row.status === 'open') &&
      (availabilityMode !== 'favorites' || row.favorite)
    );
  }

  function toggleFavorite(favoriteKey: string) {
    favoriteIds = toggleFavoritePlayerId(favoriteKey);
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
  }

  function groupRowsByProfile(raidId: string, rows: MeowConnectAvailabilityRow[]): ProfileRaidGroup[] {
    const groups = new Map<string, ProfileRaidGroup>();
    for (const row of rows) {
      const key = `${raidId}:${row.ownerId}`;
      const group = groups.get(key) || {
        key,
        ownerId: row.ownerId,
        ownerName: row.ownerName,
        ownerAvatarUrl: row.ownerAvatarUrl,
        rows: [],
        openCount: 0,
        clearedCount: 0,
        favoriteCount: 0,
        raidName: row.raid.name,
        minIlvl: row.raid.gates[0]?.minIlvl || 0
      };
      group.rows.push(row);
      if (row.status === 'open') group.openCount += 1;
      if (row.status === 'cleared') group.clearedCount += 1;
      if (row.favorite) group.favoriteCount += 1;
      groups.set(key, group);
    }

    return Array.from(groups.values()).sort((a, b) =>
      Number(b.ownerId === 'local') - Number(a.ownerId === 'local') ||
      b.openCount - a.openCount ||
      a.ownerName.localeCompare(b.ownerName)
    );
  }

  function setAvailabilityRaidFilter(raidId: string) {
    const next = new Set(focusedAvailabilityRaidIds);
    if (next.has(raidId)) {
      next.delete(raidId);
    } else {
      next.add(raidId);
    }
    focusedAvailabilityRaidIds = next;
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
    currentFavoriteIds: Set<string>,
    profile: MeowConnectProfile | null
  ): RaidTogetherRow[] {
    if (selectedConnections.length === 0) return [];

    return raids.flatMap((raid) => {
      const rows = buildMeowConnectAvailabilityRows(currentLocalSnapshot, currentRemoteSnapshots, raid.id, 'all', currentFavoriteIds, profile)
        .filter((row) => row.status === 'open');
      const myOpenCount = rows.filter((row) => row.ownerId === 'local').length;
      const participantCounts = selectedConnections.map((connection) => ({
        name: connection.profile.displayName,
        count: rows.filter((row) => isFriendAvailabilityRow(row, connection)).length
      }));
      const togetherCount = Math.min(myOpenCount, ...participantCounts.map((participant) => participant.count));

      return [{
        key: `${raid.id}:${selectedConnections.map(getFriendConnectionKey).sort().join('+')}`,
        raidName: raid.name,
        minIlvl: raid.gates[0].minIlvl,
        friendNames: selectedConnections.map((connection) => connection.profile.displayName).join(', '),
        friendAvatars: selectedConnections.map((connection) => ({
          name: connection.profile.displayName,
          avatarUrl: connection.profile.avatarUrl
        })),
        myOpenCount,
        participantCounts,
        togetherCount
      }];
    });
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

  function setAvailabilityMode(mode: AvailabilityMode) {
    availabilityMode = mode;
    localStorage.setItem(AVAILABILITY_MODE_STORAGE_KEY, mode);
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

  function loadAvailabilityMode(): AvailabilityMode {
    const value = localStorage.getItem(AVAILABILITY_MODE_STORAGE_KEY);
    return value === 'favorites' || value === 'open' ? value : 'open';
  }

  async function openFriendPopover() {
    showFriendPopover = true;
    if (friendOptions.length > 0) return;

    try {
      friendOptions = await invoke<FriendOption[]>('get_discord_whitelist_members');
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

  async function toggleStaticFriend(connection: MeowConnectFriendConnection) {
    friendActionBusy = true;

    try {
      await setMeowConnectStaticFriend(connection, !connection.sharesStatic);
      friendConnections = await loadMeowConnectFriends();
      remoteSnapshots = localSnapshot
        ? await fetchMeowConnectRemoteSnapshots(String(localSnapshot.weeklyResetMs || 0))
        : remoteSnapshots;
      markMeowConnectActive('MeowConnect is connected.');
      showToast(
        connection.sharesStatic
          ? `${connection.profile.displayName} will only see generic static reservations.`
          : `${connection.profile.displayName} can now see your static reservation details.`,
        'success'
      );
    } catch (err) {
      markMeowConnectFailure(err);
      showToast(`Failed to update static sharing: ${err}`, 'error');
    } finally {
      friendActionBusy = false;
    }
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

    unsubscribeRealtime = subscribeMeowConnectChanges(() => {
      if (realtimeRefreshTimer) clearTimeout(realtimeRefreshTimer);

      realtimeRefreshTimer = setTimeout(() => {
        void refreshRemoteMeowConnectData();
      }, 1500);
    });
  }

  async function refreshRemoteMeowConnectData() {
    if (!localSnapshot) return;

    try {
      friendConnections = await loadMeowConnectFriends();
      remoteSnapshots = await fetchMeowConnectRemoteSnapshots(String(localSnapshot.weeklyResetMs || 0));
      markMeowConnectActive('MeowConnect realtime refresh succeeded.');
    } catch (err) {
      markMeowConnectFailure(err);
      console.warn('Failed to refresh MeowConnect realtime data:', err);
    }
  }

  function getClassName(classId: string): string {
    return getClassInfo(classId)?.displayName || classId;
  }

  function getClassIcon(classId: string): string {
    return getClassInfo(classId)?.iconId || '0';
  }

  function getClassInfo(classId: string) {
    const normalized = classId.trim().toLowerCase().replace(/\s+/g, '_');
    const aliasMap: Record<string, string> = {
      gunlancer: 'warlord',
      paladin: 'holyknight',
      slayer: 'berserker_female',
      arcanist: 'arcana',
      sorceress: 'elemental_master',
      wardancer: 'battle_master',
      soulfist: 'force_master',
      glaivier: 'lance_master',
      breaker: 'infighter_male',
      deathblade: 'blade',
      shadowhunter: 'demonic',
      souleater: 'soul_eater',
      deadeye: 'devil_hunter',
      artillerist: 'blaster',
      scouter: 'machinist',
      artist: 'yinyangshi',
      aeromancer: 'weather_artist',
      wildsoul: 'alchemist',
      guardianknight: 'dragon_knight',
      valkyrie: 'holyknight_female'
    };

    return GAME_CLASSES[normalized] ||
      GAME_CLASSES[aliasMap[normalized]] ||
      Object.values(GAME_CLASSES).find((entry) => entry.displayName.toLowerCase() === classId.trim().toLowerCase());
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

  function formatLogTime(timestamp: number) {
    if (!timestamp) return 'unknown';
    return new Date(timestamp).toLocaleString([], {
      month: 'short',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit'
    });
  }
</script>

<section class="meow-connect">
  {#if !consentAccepted}
    <header class="mc-header">
      <div class="mc-title">
        <img src="/images/meowconnect_tab.png" alt="" />
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
      <section class="connect-toolbar-row">
        <div class="mc-title">
          <img src="/images/meowconnect_tab.png" alt="" />
          <h2>MeowConnect</h2>
        </div>

        <div class="overview-toolbar">
          <div class="summary-pill">
            <strong>{totalOpenCount}</strong>
            <span>open</span>
          </div>
          <div class="summary-pill">
            <strong>{connectedFriends}</strong>
            <span>friends</span>
          </div>

          {#if raidTogetherView === 'availability'}
            <div class="mode-toggle">
              <button type="button" class:active={availabilityMode === 'open'} on:click={() => setAvailabilityMode('open')}>
                Available
              </button>
              <button type="button" class:active={availabilityMode === 'favorites'} on:click={() => setAvailabilityMode('favorites')}>
                Favorites
              </button>
            </div>

            <div class="mode-toggle raid-filter-toggle">
              {#each visibleRaids as raid}
                <button
                  type="button"
                  class:active={focusedAvailabilityRaidIds.has(raid.id)}
                  on:click={() => setAvailabilityRaidFilter(raid.id)}
                  title={focusedAvailabilityRaidIds.has(raid.id) ? `Hide ${raid.name} focus` : `Add ${raid.name} focus`}
                >
                  {raid.name}
                </button>
              {/each}
            </div>
          {/if}

          <div class="mode-toggle view-toggle">
            <button type="button" class:active={raidTogetherView === 'availability'} on:click={() => raidTogetherView = 'availability'}>
              Availability
            </button>
            <button type="button" class:active={raidTogetherView === 'pairs'} on:click={() => raidTogetherView = 'pairs'}>
              Together
            </button>
          </div>
        </div>
      </section>

      {#if raidTogetherView === 'availability'}
        <section class="raid-board">
          {#if visibleRaids.length === 0}
            <div class="empty-state">No raids selected. Enable raids in Settings.</div>
          {:else}
            {#each displayedRaidSections as section}
              <article class="raid-column">
                <header>
                  <div>
                    <h3>{section.raid.name}</h3>
                    <span>{section.raid.gates[0].minIlvl}+ · all difficulties</span>
                  </div>
                  <strong>{section.rows.filter((row) => row.status === 'open').length} open</strong>
                </header>

                <div class="profile-group-grid">
                  {#if section.groups.length === 0}
                    <p class="column-empty">No matching characters.</p>
                  {:else}
                    {#each section.groups as group}
                      <article class="profile-group-card">
                        <button
                          class="profile-group-summary"
                          type="button"
                          on:click={() => openProfileGroup(group)}
                          aria-haspopup="dialog"
                        >
                          {#if group.ownerAvatarUrl}
                            <img src={group.ownerAvatarUrl} alt="" />
                          {:else}
                            <span class="avatar-fallback">{getInitials(group.ownerName)}</span>
                          {/if}
                          <span>
                            <strong>{group.ownerName}</strong>
                            <small>
                              {#if availabilityMode === 'favorites'}
                                {group.favoriteCount} favorite{group.favoriteCount === 1 ? '' : 's'}
                              {:else}
                                {group.openCount} available · {group.rows.length} shown
                              {/if}
                            </small>
                          </span>
                        </button>

                      </article>
                    {/each}
                  {/if}
                </div>
              </article>
            {/each}
          {/if}
        </section>

        {#if activeProfileGroup}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div
            class="profile-detail-backdrop"
            role="presentation"
            on:click={closeProfileGroup}
          >
            <section
              class="profile-detail-popover"
              role="dialog"
              aria-modal="true"
              aria-label={`${activeProfileGroup.ownerName} raid availability`}
              on:click|stopPropagation
            >
              <header>
                <div class="profile-detail-title">
                  {#if activeProfileGroup.ownerAvatarUrl}
                    <img src={activeProfileGroup.ownerAvatarUrl} alt="" />
                  {:else}
                    <span class="avatar-fallback">{getInitials(activeProfileGroup.ownerName)}</span>
                  {/if}
                  <div>
                    <h3>{activeProfileGroup.raidName}</h3>
                    <strong>{activeProfileGroup.ownerName}</strong>
                    <span>
                      {#if availabilityMode === 'favorites'}
                        {activeProfileGroup.favoriteCount} favorite{activeProfileGroup.favoriteCount === 1 ? '' : 's'} · {activeProfileGroup.minIlvl}+
                      {:else}
                        {activeProfileGroup.openCount} available · {activeProfileGroup.rows.length} matching characters · {activeProfileGroup.minIlvl}+
                      {/if}
                    </span>
                  </div>
                </div>
                <button type="button" class="icon-button" aria-label="Close availability details" on:click={closeProfileGroup}>×</button>
              </header>

              <div class="availability-stack">
                {#each activeProfileGroup.rows.filter((row) => availabilityMode !== 'favorites' || row.favorite) as row}
                  <article class:cleared={row.status === 'cleared'} class="availability-card">
                    <button
                      class:active={row.favorite}
                      class="favorite-button"
                      type="button"
                      title={row.favorite ? 'Remove favorite' : 'Mark favorite'}
                      aria-label={row.favorite ? 'Remove favorite' : 'Mark favorite'}
                      on:click={() => toggleFavorite(row.favoriteKey)}
                    >
                      {row.favorite ? '★' : '☆'}
                    </button>

                    <img src={`/images/classes/${getClassIcon(row.character.classId)}.png`} alt="" class="class-icon" />

                    <div class="character-copy">
                      <strong>{row.character.charName}</strong>
                      <span>{getClassName(row.character.classId)} · {Math.round(row.character.itemLevel)}</span>
                      <small>
                        {row.status === 'open' ? `${row.openGates}/${row.totalGates} gates open` : 'cleared'}
                        · {row.raid.difficulty}
                        {#if row.reservedForStatic}
                          · {row.staticReservationDetailsVisible ? 'reserved for static' : 'preserved for static'}
                        {/if}
                      </small>
                    </div>
                  </article>
                {/each}
                {#if availabilityMode === 'favorites' && activeProfileGroup.favoriteCount === 0}
                  <p class="column-empty">0 favorites</p>
                {/if}
              </div>
            </section>
          </div>
        {/if}
      {:else}
        <section class="together-panel">
          <div class="panel-title">
            <div>
              <h3>Raid Together</h3>
              <p>Select friends first, then compare the shared open-run limit across everyone selected.</p>
            </div>
          </div>

          <div class="together-friend-picker">
            {#if acceptedFriendConnections.length === 0}
              <p class="column-empty">Add or accept a MeowConnect friend to compare open runs.</p>
            {:else}
              {#each acceptedFriendConnections as connection}
                <button
                  type="button"
                  class:active={selectedTogetherFriendIds.has(getFriendConnectionKey(connection))}
                  on:click={() => toggleTogetherFriend(connection)}
                >
                  {#if connection.profile.avatarUrl}
                    <img src={connection.profile.avatarUrl} alt="" />
                  {:else}
                    <span class="avatar-fallback">{getInitials(connection.profile.displayName)}</span>
                  {/if}
                  <span>{connection.profile.displayName}</span>
                </button>
              {/each}
            {/if}
          </div>

          <div class="together-grid">
            {#if raidTogetherRows.length === 0}
              <p class="column-empty">
                {#if visibleRaids.length === 0}
                  Select at least one raid to compare open runs.
                {:else if acceptedFriendConnections.length === 0}
                  Add or accept a MeowConnect friend to compare open runs.
                {:else if selectedTogetherConnections.length === 0}
                  Select at least one friend to compare open runs.
                {:else if !localSnapshot || localSnapshot.characters.length === 0}
                  Enable Connect on at least one local character, then sync MeowConnect.
                {:else}
                  No open shared raid runs found for the current selection.
                {/if}
              </p>
            {:else}
              {#each raidTogetherRows as row}
                <article class:empty={row.togetherCount === 0} class="together-card">
                  <div class="together-main">
                    <div class="together-avatar-stack" style={`--avatar-count: ${row.friendAvatars.length}`}>
                      {#each row.friendAvatars as avatar, avatarIndex}
                        {#if avatar.avatarUrl}
                          <img src={avatar.avatarUrl} alt="" title={avatar.name} style={`--avatar-index: ${avatarIndex}`} />
                        {:else}
                          <span class="avatar-fallback" title={avatar.name} style={`--avatar-index: ${avatarIndex}`}>{getInitials(avatar.name)}</span>
                        {/if}
                      {/each}
                    </div>
                    <div>
                      <strong>{row.raidName}</strong>
                      <span>{row.friendNames} · {row.minIlvl}+</span>
                    </div>
                  </div>

                  <div class="together-count">
                    <strong>{row.togetherCount}</strong>
                    <span>together</span>
                  </div>

                  <div class="together-meta">
                    <span>You: {row.myOpenCount}</span>
                    {#each row.participantCounts as participant}
                      <span>{participant.name}: {participant.count}</span>
                    {/each}
                  </div>
                </article>
              {/each}
            {/if}
          </div>
        </section>
      {/if}
    {:else if activeSection === 'logs'}
      <section class="logs-panel">
        <div class="panel-title">
          <div>
            <h3>Clear Logs</h3>
            <p>Recent MeowConnect clears from selected raids.</p>
          </div>
        </div>

        <div class="log-list">
          {#if logEntries.length === 0}
            <p class="column-empty">No shared clears for the selected raids yet.</p>
          {:else}
            {#each logEntries as entry}
              <article class="log-row">
                <div class="log-avatar-stack" style={`--avatar-count: ${entry.participants?.length || 1}`}>
                  {#each (entry.participants?.length ? entry.participants : [{ ownerId: entry.ownerId, ownerName: entry.ownerName, ownerAvatarUrl: entry.ownerAvatarUrl, localPlayer: entry.localPlayer }]) as participant, participantIndex}
                    {#if participant.ownerAvatarUrl}
                      <img src={participant.ownerAvatarUrl} alt="" title={`${participant.ownerName} (${participant.localPlayer})`} style={`--avatar-index: ${participantIndex}`} />
                    {:else}
                      <span class="avatar-fallback" title={`${participant.ownerName} (${participant.localPlayer})`} style={`--avatar-index: ${participantIndex}`}>{getInitials(participant.ownerName)}</span>
                    {/if}
                  {/each}
                </div>
                <div>
                  <strong>{entry.ownerName} cleared {entry.raidName} {entry.difficulty}</strong>
                  <span>
                    with {entry.localPlayer}
                    {#if entry.gate}
                      · {entry.gate}
                    {/if}
                    · {entry.source}
                    · {formatLogTime(entry.fightStart)}
                  </span>
                  {#if entry.players.length > 0}
                    <small>{entry.players.join(', ')}</small>
                  {/if}
                </div>
              </article>
            {/each}
          {/if}
        </div>
      </section>
    {:else}
      <section class="settings-grid">
        <article class="settings-panel compact-panel">
          <div class="panel-title">
            <div>
              <h3>Sync</h3>
              <p>
                {localSnapshot ? `${localSnapshot.characters.length} connected characters · current weekly reset started ${formatResetTime(localSnapshot.weeklyResetMs)}` : 'No local snapshot loaded.'}
                {#if $meowConnectHasUnsyncedChanges}
                  · Unsynced local changes
                {/if}
              </p>
            </div>
            <button
              class="primary-button"
              type="button"
              on:click={() => refreshMeowConnect({ allowUpload: true, manual: true })}
              disabled={isLoading || manualSyncBlocked}
              title={manualSyncBlocked ? `Manual sync is available in ${formatDuration(manualSyncRemainingMs)}.` : $meowConnectHasUnsyncedChanges ? 'Upload unsynced MeowConnect changes' : 'Upload your current MeowConnect snapshot'}
            >
              {manualSyncLabel}
            </button>
          </div>
        </article>

        <article class="settings-panel compact-panel">
          <div class="panel-title">
            <div>
              <h3>Reset Window</h3>
              <p>MeowConnect shares current in-game reset data only. Raid clears and encounter logs roll into a fresh weekly window after reset.</p>
            </div>
          </div>
        </article>

        <article class="settings-panel wide-panel">
          <div class="panel-title">
            <div>
              <h3>Raid Visibility</h3>
              <p>Select the raids shown on the Raid Together board.</p>
            </div>
            <div class="panel-actions">
              <button type="button" on:click={selectAllRaids}>All</button>
              <button type="button" on:click={clearRaidSelection}>None</button>
            </div>
          </div>

          <div class="raid-toggle-grid">
            {#each raidOptions as raid}
              <label class:active={visibleRaidIds.includes(raid.id)}>
                <input
                  type="checkbox"
                  checked={visibleRaidIds.includes(raid.id)}
                  on:change={() => toggleRaidVisibility(raid.id)}
                />
                <span>{raid.name}</span>
                <small>{raid.gates[0].minIlvl}+</small>
              </label>
            {/each}
          </div>
        </article>

        <article class="settings-panel wide-panel">
          <div class="panel-title">
            <div>
              <h3>Friends</h3>
              <p>{connectedFriends} connected · {pendingIncoming.length} incoming request{pendingIncoming.length === 1 ? '' : 's'}</p>
            </div>
            <div class="friend-add-control">
              {#if consentAccepted && showFriendPopover}
                <div class="friend-popover" bind:this={friendPopoverEl}>
                  <button
                    class="popover-close"
                    type="button"
                    aria-label="Close add friends"
                    on:click={() => showFriendPopover = false}
                  >
                    X
                  </button>
                  <div class="friend-search-row">
                    <input
                      bind:value={friendSearch}
                      placeholder="Type whitelist name"
                      disabled={friendActionBusy}
                      on:input={() => {
                        friendDiscordId = '';
                      }}
                      on:keydown={(event) => {
                        if (event.key === 'Enter') {
                          event.preventDefault();
                          void sendFriendRequest();
                        }
                        if (event.key === 'Escape') {
                          showFriendPopover = false;
                        }
                      }}
                    />
                    <button
                      type="button"
                      disabled={friendActionBusy || !(friendDiscordId || friendSearch).trim()}
                      on:click={sendFriendRequest}
                    >
                      Add
                    </button>
                  </div>

                  <div class="friend-suggestion-list">
                    {#if filteredFriendOptions.length === 0}
                      <p>No whitelist name matches.</p>
                    {:else}
                      {#each filteredFriendOptions as friend}
                        <button type="button" on:click={() => selectFriendOption(friend)}>
                          {#if getProfileAvatar(friend.id)}
                            <img src={getProfileAvatar(friend.id)} alt="" />
                          {:else}
                            <span class="avatar-fallback">{getInitials(friend.name)}</span>
                          {/if}
                          <strong>{friend.name}</strong>
                        </button>
                      {/each}
                    {/if}
                  </div>
                </div>
              {/if}

              <button class="primary-button" type="button" on:click={openFriendPopover}>
                Add friend
              </button>
            </div>
          </div>

          <div class="friend-list">
            {#if friendConnections.length === 0}
              <p class="column-empty">No MeowConnect friends yet.</p>
            {:else}
              {#each friendConnections as connection}
                <div class="friend-row">
                  {#if connection.profile.avatarUrl}
                    <img src={connection.profile.avatarUrl} alt="" />
                  {:else}
                    <span class="avatar-fallback">{getInitials(connection.profile.displayName)}</span>
                  {/if}
                  <div>
                    <strong>{connection.profile.displayName}</strong>
                    <span>{connection.status}{connection.status === 'pending' && connection.direction === 'incoming' ? ' incoming' : ''}{connection.sharesStatic ? ' · static' : ''}</span>
                  </div>
                  <div class="friend-actions">
                    {#if connection.status === 'pending' && connection.direction === 'incoming'}
                      <button class="mini-button" type="button" disabled={friendActionBusy} on:click={() => acceptFriendRequest(connection)}>
                        Accept
                      </button>
                    {/if}
                    {#if connection.status === 'accepted'}
                      <button
                        class:active={connection.sharesStatic}
                        class="mini-button"
                        type="button"
                        disabled={friendActionBusy}
                        title={connection.sharesStatic ? 'This friend can see your exact static reservations' : 'Let this friend see your exact static reservations'}
                        on:click={() => toggleStaticFriend(connection)}
                      >
                        {connection.sharesStatic ? 'Static' : 'Mark static'}
                      </button>
                    {/if}
                    <button class="mini-button subtle" type="button" disabled={friendActionBusy} on:click={() => removeFriend(connection)}>
                      Remove
                    </button>
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        </article>
      </section>
    {/if}
  {/if}
</section>

<style>
  .meow-connect {
    width: min(1280px, 100%);
    margin: 0 auto;
    padding: 0.65rem 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    box-sizing: border-box;
  }

  .mc-header,
  .connect-toolbar-row,
  .overview-toolbar,
  .panel-title,
  .friend-row {
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

  .connect-toolbar-row {
    align-self: center;
    width: min(100%, calc((320px * 3) + (0.6rem * 2)));
    flex-wrap: wrap;
  }

  .connect-toolbar-row .overview-toolbar {
    flex: 1;
    min-width: min(100%, 28rem);
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

  .panel-actions,
  .friend-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .mode-toggle {
    display: inline-flex;
    padding: 0.14rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 7px;
    background: var(--md-sys-color-surface);
  }

  .mode-toggle button,
  .panel-actions button,
  .mini-button,
  .primary-button,
  .friend-search-row button {
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

  .mode-toggle button {
    padding: 0.34rem 0.52rem;
    font-weight: 600;
  }

  .mode-toggle button.active,
  .primary-button {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
  }

  .primary-button {
    padding: 0.42rem 0.58rem;
  }

  .friend-popover {
    position: absolute;
    top: 0;
    right: calc(100% + 0.55rem);
    z-index: 80;
    width: min(340px, 72vw);
    padding: 0.58rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 10px;
    background: var(--md-sys-color-surface);
    box-shadow: 0 10px 30px color-mix(in srgb, black 24%, transparent);
  }

  .friend-add-control {
    position: relative;
    display: flex;
    justify-content: flex-end;
    align-items: center;
  }

  .popover-close {
    position: absolute;
    top: 0.28rem;
    right: 0.28rem;
    width: 1.35rem;
    height: 1.35rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
    font-size: 0.7rem;
    font-weight: 700;
  }

  .popover-close:hover {
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface);
  }

  .friend-search-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.4rem;
    padding-right: 1.45rem;
  }

  .friend-search-row input {
    min-width: 0;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 8px;
    padding: 0.46rem 0.58rem;
    background: var(--md-sys-color-surface-container);
    color: var(--md-sys-color-on-surface);
    font: inherit;
    font-size: 0.78rem;
  }

  .friend-search-row input:focus {
    outline: none;
    border-color: var(--md-sys-color-primary);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--md-sys-color-primary) 18%, transparent);
  }

  .friend-search-row button {
    padding: 0.46rem 0.62rem;
    border-radius: 8px;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    font-size: 0.76rem;
    font-weight: 600;
  }

  .friend-search-row button:disabled,
  .primary-button:disabled,
  .mini-button:disabled {
    cursor: default;
    opacity: 0.6;
  }

  .friend-suggestion-list {
    display: grid;
    gap: 0.26rem;
    margin-top: 0.45rem;
    max-height: 220px;
    overflow: auto;
  }

  .friend-suggestion-list button {
    min-width: 0;
    display: grid;
    grid-template-columns: 1.7rem minmax(0, 1fr);
    gap: 0.48rem;
    align-items: center;
    border: 1px solid transparent;
    border-radius: 8px;
    padding: 0.32rem;
    background: transparent;
    color: var(--md-sys-color-on-surface);
    text-align: left;
    cursor: pointer;
  }

  .friend-suggestion-list button:hover {
    border-color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 8%, transparent);
  }

  .friend-suggestion-list strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.78rem;
    font-weight: 600;
  }

  .friend-suggestion-list img,
  .friend-suggestion-list .avatar-fallback {
    width: 1.7rem;
    height: 1.7rem;
  }

  .friend-suggestion-list img,
  .friend-row img,
  .profile-group-summary img,
  .together-main img,
  .log-row img,
  .avatar-fallback {
    width: 2rem;
    height: 2rem;
    border-radius: 50%;
  }

  .avatar-fallback {
    display: grid;
    place-items: center;
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
    font-weight: 900;
  }

  .consent-panel,
  .settings-panel,
  .empty-state {
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
  }

  .consent-panel {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 1rem;
    align-items: center;
    padding: 1rem;
  }

  .consent-panel p,
  .panel-title p,
  .column-empty,
  .friend-suggestion-list p {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.82rem;
    line-height: 1.45;
  }

  .overview-toolbar {
    flex-wrap: wrap;
    gap: 0.28rem;
  }

  .view-toggle {
    margin-left: auto;
  }

  .raid-filter-toggle {
    max-width: min(42rem, 100%);
    overflow-x: auto;
    scrollbar-width: thin;
  }

  .summary-pill {
    display: inline-flex;
    align-items: baseline;
    gap: 0.28rem;
    min-width: 0;
    padding: 0.34rem 0.48rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 7px;
    background: var(--md-sys-color-surface);
  }

  .summary-pill strong {
    color: var(--md-sys-color-on-surface);
    font-size: 0.86rem;
  }

  .summary-pill span {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
    text-transform: uppercase;
  }

  .raid-board {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(245px, 320px));
    justify-content: center;
    gap: 0.6rem;
    overflow-y: auto;
    max-height: min(68vh, calc(100vh - 15rem));
    padding-bottom: 0.35rem;
    scrollbar-width: thin;
    align-items: start;
  }

  .raid-column {
    min-width: 0;
    min-height: 16rem;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
    overflow: hidden;
  }

  .raid-column > header {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.55rem 0.65rem;
    border-bottom: 1px solid var(--md-sys-color-outline);
    background: var(--md-sys-color-surface-variant);
  }

  .raid-column span,
  .character-copy span,
  .character-copy small,
  .friend-row span {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.74rem;
  }

  .raid-column header strong {
    color: var(--md-sys-color-primary);
    white-space: nowrap;
    font-size: 0.82rem;
  }

  .availability-stack {
    display: grid;
    gap: 0.35rem;
    padding: 0.4rem;
  }

  .profile-group-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 0.45rem;
    padding: 0.45rem;
    align-items: start;
  }

  .profile-group-card {
    min-width: 0;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
    overflow: hidden;
  }

  .profile-group-summary {
    width: 100%;
    min-width: 0;
    display: grid;
    grid-template-columns: 2rem minmax(0, 1fr);
    gap: 0.5rem;
    align-items: center;
    padding: 0.5rem;
    border: 0;
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface);
    text-align: left;
    cursor: pointer;
  }

  .profile-group-summary:hover {
    background: color-mix(in srgb, var(--md-sys-color-primary) 9%, var(--md-sys-color-surface-variant));
  }

  .profile-detail-backdrop {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: start center;
    padding: clamp(1rem, 6vh, 4rem) 1rem 1rem;
    background: rgba(0, 0, 0, 0.18);
  }

  .profile-detail-popover {
    width: min(34rem, 100%);
    max-height: min(72vh, 42rem);
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    overflow: hidden;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
    box-shadow: 0 16px 44px rgba(0, 0, 0, 0.22);
  }

  .profile-detail-popover > header {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: center;
    padding: 0.7rem 0.75rem;
    border-bottom: 1px solid var(--md-sys-color-outline);
    background: var(--md-sys-color-surface-variant);
  }

  .profile-detail-title {
    min-width: 0;
    flex: 1;
    display: grid;
    grid-template-columns: 3rem minmax(0, 1fr);
    gap: 0.8rem;
    align-items: center;
  }

  .profile-detail-title > img,
  .profile-detail-title > .avatar-fallback {
    width: 3rem;
    height: 3rem;
  }

  .profile-detail-title div {
    min-width: 0;
    display: grid;
    gap: 0.08rem;
  }

  .profile-detail-title h3 {
    min-width: 0;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.92rem;
  }

  .profile-detail-title strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--md-sys-color-on-surface);
    font-size: 0.78rem;
    font-weight: 650;
  }

  .profile-detail-title span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.74rem;
  }

  .profile-detail-popover .availability-stack {
    overflow-y: auto;
    padding: 0.55rem;
  }

  .icon-button {
    width: 2rem;
    height: 2rem;
    display: grid;
    place-items: center;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 50%;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font-size: 1.1rem;
    line-height: 1;
  }

  .icon-button:hover {
    border-color: var(--md-sys-color-primary);
    color: var(--md-sys-color-primary);
  }

  .profile-group-summary span {
    min-width: 0;
    display: grid;
    gap: 0.05rem;
  }

  .profile-group-summary strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.8rem;
  }

  .profile-group-summary small {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
  }

  .availability-card {
    position: relative;
    display: grid;
    grid-template-columns: 1.65rem minmax(0, 1fr);
    gap: 0.45rem;
    align-items: center;
    min-height: 3.35rem;
    padding: 0.42rem 0.48rem 0.42rem 2.25rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
  }

  .availability-card.cleared {
    opacity: 0.58;
  }

  .availability-card.cleared .character-copy {
    text-decoration: line-through;
    text-decoration-thickness: 1px;
    text-decoration-color: var(--md-sys-color-on-surface-variant);
  }

  .favorite-button {
    position: absolute;
    left: 0.35rem;
    top: 50%;
    transform: translateY(-50%);
    width: 1.45rem;
    height: 1.45rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
    font-size: 1rem;
  }

  .favorite-button.active,
  .mini-button.active {
    color: #ffbf00;
  }

  .class-icon {
    width: 1.65rem;
    height: 1.65rem;
    object-fit: contain;
  }

  .character-copy {
    min-width: 0;
    display: grid;
    gap: 0.1rem;
  }

  .character-copy strong,
  .friend-row strong,
  .friend-suggestion-list strong,
  .log-row strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--md-sys-color-on-surface);
    font-size: 0.8rem;
  }

  .together-panel,
  .logs-panel {
    display: grid;
    gap: 0.6rem;
    padding: 0.65rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
  }

  .together-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
    gap: 0.55rem;
  }

  .together-friend-picker {
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem;
    align-items: center;
  }

  .together-friend-picker button {
    min-width: 0;
    display: inline-grid;
    grid-template-columns: 1.5rem minmax(0, auto);
    gap: 0.4rem;
    align-items: center;
    max-width: 12rem;
    padding: 0.34rem 0.5rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 999px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
    font-size: 0.76rem;
  }

  .together-friend-picker button.active {
    border-color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 12%, var(--md-sys-color-surface));
    color: var(--md-sys-color-on-surface);
  }

  .together-friend-picker img,
  .together-friend-picker .avatar-fallback {
    width: 1.5rem;
    height: 1.5rem;
  }

  .together-friend-picker span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .together-card {
    min-width: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.55rem;
    align-items: center;
    padding: 0.65rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface-variant);
  }

  .together-card.empty {
    opacity: 0.62;
  }

  .together-main {
    min-width: 0;
    display: grid;
    grid-template-columns: 4.2rem minmax(0, 1fr);
    gap: 0.55rem;
    align-items: center;
  }

  .together-avatar-stack {
    position: relative;
    width: 4.2rem;
    height: 2rem;
  }

  .together-avatar-stack img,
  .together-avatar-stack .avatar-fallback {
    position: absolute;
    top: 0;
    left: calc(var(--avatar-index, 0) * 1.05rem);
    border: 2px solid var(--md-sys-color-surface-variant);
    box-sizing: border-box;
  }

  .together-main div {
    min-width: 0;
    display: grid;
    gap: 0.05rem;
  }

  .together-main strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--md-sys-color-on-surface);
    font-size: 0.84rem;
  }

  .together-main span,
  .together-count span,
  .together-meta span {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
  }

  .together-count {
    display: grid;
    min-width: 3.8rem;
    text-align: right;
  }

  .together-count strong {
    color: var(--md-sys-color-primary);
    font-size: 1.15rem;
    line-height: 1;
  }

  .together-meta {
    grid-column: 1 / -1;
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
  }

  .log-list {
    display: grid;
    gap: 0.45rem;
  }

  .log-row {
    min-width: 0;
    display: grid;
    grid-template-columns: 4.4rem minmax(0, 1fr);
    gap: 0.6rem;
    align-items: start;
    padding: 0.55rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface-variant);
  }

  .log-row > div:not(.log-avatar-stack) {
    min-width: 0;
    display: grid;
    gap: 0.15rem;
  }

  .log-avatar-stack {
    --avatar-count: 1;
    position: relative;
    min-width: 2rem;
    width: 4.4rem;
    height: 2rem;
  }

  .log-avatar-stack img,
  .log-avatar-stack .avatar-fallback {
    position: absolute;
    top: 0;
    left: calc(var(--avatar-index, 0) * 1.15rem);
    border: 2px solid var(--md-sys-color-surface-variant);
    box-sizing: border-box;
  }

  .log-row span,
  .log-row small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.74rem;
  }

  .settings-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.85rem;
    padding: 1rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 12px;
    background: var(--md-sys-color-surface-container);
  }

  .settings-panel {
    display: grid;
    gap: 0.75rem;
    padding: 0.85rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 12px;
    background: var(--md-sys-color-surface);
    transition: border-color 0.18s ease, box-shadow 0.18s ease;
  }

  .settings-panel:hover {
    border-color: color-mix(in srgb, var(--md-sys-color-primary) 65%, var(--md-sys-color-outline-variant));
    box-shadow: 0 2px 8px color-mix(in srgb, var(--md-sys-color-primary) 16%, transparent);
  }

  .settings-panel .panel-title {
    align-items: flex-start;
    gap: 0.75rem;
  }

  .settings-panel h3 {
    margin-bottom: 0.12rem;
    font-size: 0.94rem;
    font-weight: 600;
  }

  .settings-panel .panel-title p {
    max-width: 60rem;
    font-size: 0.76rem;
    line-height: 1.35;
  }

  .settings-panel .primary-button,
  .settings-panel .panel-actions button {
    border-radius: 8px;
    font-size: 0.76rem;
    font-weight: 600;
    transition: background 0.18s ease, color 0.18s ease, border-color 0.18s ease;
  }

  .settings-panel .panel-actions button {
    padding: 0.38rem 0.58rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface-variant);
  }

  .settings-panel .panel-actions button:hover {
    border-color: var(--md-sys-color-primary);
    color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 8%, transparent);
  }

  .wide-panel {
    grid-column: 1 / -1;
  }

  .raid-toggle-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(145px, 1fr));
    gap: 0.5rem;
  }

  .raid-toggle-grid label {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 0.5rem;
    align-items: center;
    padding: 0.52rem 0.58rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 8px;
    color: var(--md-sys-color-on-surface);
    background: var(--md-sys-color-surface);
    cursor: pointer;
    transition: border-color 0.18s ease, background 0.18s ease;
  }

  .raid-toggle-grid label:hover {
    border-color: var(--md-sys-color-primary);
  }

  .raid-toggle-grid label.active {
    border-color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 12%, transparent);
  }

  .raid-toggle-grid input {
    accent-color: var(--md-sys-color-primary);
  }

  .raid-toggle-grid span,
  .raid-toggle-grid small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .raid-toggle-grid small {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
  }

  .friend-list {
    display: grid;
    gap: 0.5rem;
  }

  .friend-row {
    padding: 0.58rem 0.65rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
    transition: border-color 0.18s ease, background 0.18s ease;
  }

  .friend-row:hover {
    border-color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 6%, var(--md-sys-color-surface));
  }

  .friend-row > div:first-of-type {
    min-width: 0;
    flex: 1;
  }

  .mini-button {
    padding: 0.36rem 0.52rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    background: var(--md-sys-color-surface);
    font-weight: 600;
    transition: border-color 0.18s ease, background 0.18s ease;
  }

  .mini-button:hover:not(:disabled) {
    border-color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 8%, var(--md-sys-color-surface));
  }

  .mini-button.subtle {
    color: var(--md-sys-color-on-surface-variant);
  }

  .empty-state {
    padding: 0.75rem 0.85rem;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.86rem;
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
    .connect-toolbar-row,
    .consent-panel,
    .panel-title,
    .overview-toolbar {
      display: grid;
      grid-template-columns: 1fr;
    }

    .connect-toolbar-row {
      width: 100%;
    }

    .settings-grid {
      grid-template-columns: 1fr;
      padding: 0.75rem;
    }

    .friend-add-control {
      justify-content: flex-start;
    }

    .friend-popover {
      top: calc(100% + 0.45rem);
      right: auto;
      left: 0;
      width: min(340px, calc(100vw - 2.5rem));
    }

    .wide-panel {
      grid-column: auto;
    }

    .raid-board {
      grid-template-columns: 1fr;
      max-height: none;
      overflow: visible;
    }

    .profile-detail-backdrop {
      place-items: end center;
      padding: 0.75rem;
    }

    .profile-detail-popover {
      max-height: min(82vh, 42rem);
    }

    .friend-row {
      display: grid;
      grid-template-columns: 2rem minmax(0, 1fr);
    }

    .friend-actions {
      grid-column: 2;
      justify-content: flex-start;
    }
  }
</style>
