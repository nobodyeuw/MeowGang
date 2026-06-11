<script lang="ts">
  import { loadDiscordWhitelistMembers } from '$lib/services/discord-whitelist';
  import {
    deleteRaidSignupSheet,
    cancelRaidSignupSheet,
    grantRaidManagementAccessMember,
    getRaidManagementAccessMembers,
    getRaidSignupSheets,
    hasRaidManagementAccess,
    loadRaidManagementAccessMembers,
    loadRaidManagementRequests,
    loadRaidSignupSheetsFromSupabase,
    publishRaidSignupSheet,
    removeRaidSignupEntry,
    removeRaidManagementAccessMember,
    revokeRaidManagementAccessMember,
    saveRaidSignupSheet,
    setRaidManagementAccessMember,
    updateRaidManagementRequestStatus,
    updateRaidSignupSheet
  } from '$lib/services/raid-management';
  import {
    RAID_SIGNUP_RAIDS,
    buildRaidSignupComposition,
    getRaidSignupRaid,
    getRaidSignupSelectedRaids,
    getRaidSignupTotalDpsSpots,
    getRaidSignupTotalSpots,
    getRaidSignupTotalSupportSpots,
    type RaidManagementAccessMember,
    type RaidManagementRequest,
    type RaidManagementRunType,
    type RaidSignupCustomRaid,
    type RaidSignupPreRegisteredMember,
    type RaidSignupRole,
    type RaidSignupSheet
  } from '$lib/data/raid-management';
  import { appAsset } from '$lib/assets';
  import type { FriendOption } from '$lib/components/meow-connect/types';

  export let discordId = '';
  export let discordName = '';
  export let accessGranted = false;

  let accessMembers: RaidManagementAccessMember[] = [];
  let whitelistMembers: FriendOption[] = [];
  let localSheets: RaidSignupSheet[] = [];
  let sharedSheets: RaidSignupSheet[] = [];
  let loadingWhitelist = false;
  let selectedAccessDiscordId = '';
  let activeMode: 'sheets' | 'requests' | 'events' | 'configuration' = 'sheets';
  let requestFilter = 'all';
  let requestSearch = '';
  let showDoneRequests = false;
  let requests: RaidManagementRequest[] = [];
  let requestsError = '';
  let globalRefreshLoading = false;
  let globalRefreshError = '';
  let globalRefreshLoadedAt = 0;
  let publishLoading = false;
  let publishMessage = '';
  let eventEditMessage = '';
  let editingSheetId = '';
  let editingEventId = '';
  let editingPublished = false;

  let title = '';
  let runType: RaidManagementRunType = 'learning';
  let selectedRaidOption = '';
  let selectedRaidIds: string[] = [];
  let customRaids: RaidSignupCustomRaid[] = [];
  let customRaidName = '';
  let customRaidSpots = 8;
  let startsAtLocal = '';
  let dpsSpots = 6;
  let supportSpots = 2;
  let anySpots = 0;
  let experiencedRequired = 1;
  let customCompositionEnabled = false;
  let builderErrors: Record<string, string> = {};
  let note = '';
  let preRegisteredMembers: RaidSignupPreRegisteredMember[] = [];
  let preRegisterDiscordId = '';
  let preRegisterDisplayName = '';
  let preRegisterRole: RaidSignupRole = 'dps';
  let preRegisterStatus: RaidSignupPreRegisteredMember['status'] = 'learner';

  const roleIcons = {
    dps: appAsset('meowtator_dps.png'),
    support: appAsset('meowtator_sup.png'),
    any: appAsset('meowtator_any.webp'),
    learning: appAsset('meowtator_learning.webp'),
    experienced: appAsset('meowtator_expierenced.webp'),
    canHelp: appAsset('meowtator_can_help.webp'),
    leader: appAsset('meowtator_leader.webp')
  };

  $: hasAccess = accessGranted || hasRaidManagementAccess(discordId);
  $: selectedRaidCapacity = getRaidSignupTotalSpots(selectedRaidIds, customRaids);
  $: suggestedDpsSpots = getRaidSignupTotalDpsSpots(selectedRaidIds, customRaids);
  $: suggestedSupportSpots = getRaidSignupTotalSupportSpots(selectedRaidIds, customRaids);
  $: selectedRoleTotal = dpsSpots + supportSpots + anySpots;
  $: selectedRaidCount = selectedRaidIds.length + customRaids.length;
  $: selectedRaids = getRaidSignupSelectedRaids(selectedRaidIds, customRaids);
  $: startsAtDiscord = formatDiscordTimestamp(startsAtLocal, 'F');
  $: startsAtRelative = formatDiscordTimestamp(startsAtLocal, 'R');
  $: accessibleWhitelistMembers = whitelistMembers.filter(
    (member) => !accessMembers.some((access) => access.discordId === member.id)
  );
  $: reviewerNames = new Map([
    ...whitelistMembers.map((member) => [member.id, member.name] as const),
    ...accessMembers.map((member) => [member.discordId, member.displayName] as const),
    ...(discordId ? [[discordId, discordName || discordId] as const] : [])
  ]);
  $: sheets = [
    ...localSheets,
    ...sharedSheets.filter((sharedSheet) => !localSheets.some((localSheet) => localSheet.id === sharedSheet.id))
  ];
  $: draftSheets = localSheets;
  $: orderedDraftSheets = [...draftSheets].sort((a, b) => {
    const startA = parseDiscordTimestamp(a.startsAt) || Number.MAX_SAFE_INTEGER;
    const startB = parseDiscordTimestamp(b.startsAt) || Number.MAX_SAFE_INTEGER;
    return startA - startB || a.updatedAt - b.updatedAt;
  });
  $: if (!customCompositionEnabled) {
    dpsSpots = suggestedDpsSpots;
    supportSpots = suggestedSupportSpots;
  }

  function refreshLocalState() {
    accessMembers = getRaidManagementAccessMembers();
    localSheets = getRaidSignupSheets();
  }

  async function loadWhitelist() {
    if (!hasAccess || loadingWhitelist || whitelistMembers.length > 0) return;
    try {
      loadingWhitelist = true;
      whitelistMembers = await loadDiscordWhitelistMembers();
    } catch (error) {
      console.warn('Failed to load whitelist members for Raid Management:', error);
    } finally {
      loadingWhitelist = false;
    }
  }

  async function refreshRaidManagementData() {
    if (globalRefreshLoading) return;
    try {
      globalRefreshLoading = true;
      requestsError = '';
      globalRefreshError = '';
      const [nextRequests, nextSharedSheets, nextWhitelistMembers, nextAccessMembers] = await Promise.all([
        loadRaidManagementRequests(),
        loadRaidSignupSheetsFromSupabase(),
        loadDiscordWhitelistMembers().catch(() => whitelistMembers),
        loadRaidManagementAccessMembers().catch(() => accessMembers)
      ]);
      requests = nextRequests;
      sharedSheets = nextSharedSheets;
      whitelistMembers = nextWhitelistMembers;
      accessMembers = nextAccessMembers;
      globalRefreshLoadedAt = Date.now();
    } catch (error) {
      globalRefreshError = error instanceof Error ? error.message : String(error);
      requestsError = globalRefreshError;
      console.warn('Failed to load raid management data:', error);
    } finally {
      globalRefreshLoading = false;
    }
  }

  function formatDiscordTimestamp(value: string, style: 'F' | 'R') {
    if (!value) return '';
    const date = new Date(value);
    const timestamp = Math.floor(date.getTime() / 1000);
    if (!Number.isFinite(timestamp)) return '';
    return `<t:${timestamp}:${style}>`;
  }

  function formatRunType(value: RaidManagementRunType) {
    if (value === 'raid-night') return 'Raid Night';
    return value === 'learning' ? 'Learning' : 'Reclear';
  }

  function parseDiscordTimestamp(value: string): number | null {
    const match = value.match(/<t:(\d+):[a-zA-Z]>/);
    if (!match) return null;
    return Number(match[1]) * 1000;
  }

  function isSheetUpcomingOrOngoing(sheet: RaidSignupSheet) {
    const startTime = parseDiscordTimestamp(sheet.startsAt);
    if (!startTime) return false;
    const now = Date.now();
    const ongoingUntil = startTime + 1000 * 60 * 60 * 2;
    return now <= ongoingUntil;
  }

  function formatDateTime(value: number) {
    return new Intl.DateTimeFormat(undefined, {
      day: '2-digit',
      month: '2-digit',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    }).format(new Date(value));
  }

  function toLocalDateTimeInput(value: string) {
    const timestamp = parseDiscordTimestamp(value);
    if (!timestamp) return '';
    const date = new Date(timestamp);
    const offsetDate = new Date(date.getTime() - date.getTimezoneOffset() * 60000);
    return offsetDate.toISOString().slice(0, 16);
  }

  function dateToLocalDateTimeInput(date: Date) {
    const offsetDate = new Date(date.getTime() - date.getTimezoneOffset() * 60000);
    return offsetDate.toISOString().slice(0, 16);
  }

  function parseServerDateTimeRequest(dateValue: string, timeValue: string): string {
    const dateMatch = dateValue.trim().match(/^(\d{1,2})\.(\d{1,2})\.(\d{4})$/);
    const timeMatch = timeValue.trim().match(/^(\d{1,2}):(\d{2})$/);
    if (!dateMatch || !timeMatch) return '';

    const day = Number(dateMatch[1]);
    const month = Number(dateMatch[2]);
    const year = Number(dateMatch[3]);
    const hour = Number(timeMatch[1]);
    const minute = Number(timeMatch[2]);
    if (
      day < 1 ||
      day > 31 ||
      month < 1 ||
      month > 12 ||
      hour < 0 ||
      hour > 23 ||
      minute < 0 ||
      minute > 59
    ) {
      return '';
    }

    const date = new Date(year, month - 1, day, hour, minute, 0, 0);
    if (
      date.getFullYear() !== year ||
      date.getMonth() !== month - 1 ||
      date.getDate() !== day ||
      date.getHours() !== hour ||
      date.getMinutes() !== minute
    ) {
      return '';
    }

    return dateToLocalDateTimeInput(date);
  }

  function formatReviewerName(discordIdValue: string) {
    const normalizedId = String(discordIdValue || '').trim();
    if (!normalizedId) return 'Unknown';
    return reviewerNames.get(normalizedId) || normalizedId;
  }

  function isRequestDone(request: RaidManagementRequest) {
    return request.status === 'closed';
  }

  async function markRequestDone(request: RaidManagementRequest) {
    if (isRequestDone(request)) return;
    try {
      await updateRaidManagementRequestStatus(request.id, 'closed', discordId);
      requests = requests.map((entry) =>
        entry.id === request.id ? { ...entry, status: 'closed', decidedBy: discordId || entry.decidedBy } : entry
      );
    } catch (error) {
      requestsError = error instanceof Error ? error.message : String(error);
      console.warn('Failed to mark raid request done:', error);
    }
  }

  async function reopenRequest(request: RaidManagementRequest) {
    try {
      await updateRaidManagementRequestStatus(request.id, 'accepted', discordId);
      requests = requests.map((entry) =>
        entry.id === request.id ? { ...entry, status: 'accepted', decidedBy: discordId || entry.decidedBy } : entry
      );
    } catch (error) {
      requestsError = error instanceof Error ? error.message : String(error);
      console.warn('Failed to reopen raid request:', error);
    }
  }

  function getSheetRaidNames(sheet: RaidSignupSheet): string[] {
    return getRaidSignupSelectedRaids(sheet.raidIds, sheet.customRaids || []).map((raid) => raid.name);
  }

  function getSheetRaidCount(sheet: RaidSignupSheet): number {
    return sheet.raidIds.length + (sheet.customRaids?.length || 0);
  }

  function isSharedSheet(sheet: RaidSignupSheet) {
    return sharedSheets.some((sharedSheet) => sharedSheet.id === sheet.id);
  }

  function getSheetMembers(sheet: RaidSignupSheet, role: RaidSignupRole) {
    return (sheet.preRegisteredMembers || []).filter((member) =>
      member.role === role && member.status !== 'can_help' && member.status !== 'leader'
    );
  }

  function getSheetCanHelpMembers(sheet: RaidSignupSheet) {
    return (sheet.preRegisteredMembers || []).filter((member) => member.status === 'can_help');
  }

  function getSheetLeader(sheet: RaidSignupSheet) {
    return (sheet.preRegisteredMembers || []).find((member) => member.status === 'leader');
  }

  function getSheetEventGroups(sheet: RaidSignupSheet) {
    const groups = [
      { key: 'leader', label: 'Leader', members: (sheet.preRegisteredMembers || []).filter((member) => member.status === 'leader') },
      { key: 'dps', label: 'DPS', members: getSheetMembers(sheet, 'dps') },
      { key: 'support', label: 'SUP', members: getSheetMembers(sheet, 'support') },
      { key: 'any', label: 'ANY', members: getSheetMembers(sheet, 'any') },
      { key: 'can_help', label: 'Can Help', members: getSheetCanHelpMembers(sheet) }
    ];
    return groups.filter((group) => group.members.length > 0);
  }

  function getRoleIcon(role: RaidSignupRole) {
    if (role === 'support') return roleIcons.support;
    if (role === 'any') return roleIcons.any;
    return roleIcons.dps;
  }

  function getStatusIcon(status: RaidSignupPreRegisteredMember['status']) {
    if (status === 'leader') return roleIcons.leader;
    if (status === 'experienced') return roleIcons.experienced;
    if (status === 'can_help') return roleIcons.canHelp;
    return roleIcons.learning;
  }

  function createEventId() {
    return `meow-${Date.now().toString(36)}`;
  }

  function addSelectedRaid() {
    if (!selectedRaidOption || selectedRaidOption === 'custom') return;
    if (!getRaidSignupRaid(selectedRaidOption) || selectedRaidIds.includes(selectedRaidOption)) return;
    selectedRaidIds = [...selectedRaidIds, selectedRaidOption];
    selectedRaidOption = '';
  }

  function removeFixedRaid(raidId: string) {
    selectedRaidIds = selectedRaidIds.filter((id) => id !== raidId);
  }

  function applySuggestedComposition() {
    dpsSpots = suggestedDpsSpots;
    supportSpots = suggestedSupportSpots;
    customCompositionEnabled = false;
    builderErrors = { ...builderErrors, composition: '' };
  }

  function validateSheetForm() {
    const errors: Record<string, string> = {};
    if (selectedRaidCount === 0) errors.raids = 'Choose at least one raid.';
    if (!startsAtLocal || !startsAtDiscord) errors.startsAt = 'Choose a valid start time.';
    if (selectedRoleTotal <= 0) errors.composition = 'Set at least one signup spot.';
    if (customCompositionEnabled && selectedRaidCapacity > 0 && selectedRoleTotal !== selectedRaidCapacity) {
      errors.composition = `Configured spots should match raid capacity (${selectedRaidCapacity}).`;
    }
    if (runType === 'learning' && experiencedRequired < 0) {
      errors.experienced = 'Minimum experienced cannot be negative.';
    }
    builderErrors = errors;
    return Object.keys(errors).length === 0;
  }

  function addCustomRaid() {
    const cleanName = customRaidName.trim();
    if (!cleanName) return;

    customRaids = [
      ...customRaids,
      {
        id: `custom-${Date.now()}`,
        name: cleanName,
        custom: true,
        ...buildRaidSignupComposition(customRaidSpots)
      }
    ];
    customRaidName = '';
    customRaidSpots = 8;
    selectedRaidOption = '';
  }

  function removeCustomRaid(raidId: string) {
    customRaids = customRaids.filter((raid) => raid.id !== raidId);
  }

  function getRequestRunType(request: RaidManagementRequest): RaidManagementRunType {
    const category = request.category.toLowerCase();
    if (category.includes('night') || category.includes('train')) return 'raid-night';
    if (category.includes('reclear')) return 'reclear';
    return 'learning';
  }

  function useRequest(request: RaidManagementRequest) {
    title = request.title;
    runType = getRequestRunType(request);
    selectedRaidIds = [];
    customRaids = [];
    startsAtLocal = parseServerDateTimeRequest(request.dateWindow, request.timeWindow);

    for (const raidName of request.raidNames) {
      const fixedRaid = RAID_SIGNUP_RAIDS.find((raid) => raid.name.toLowerCase() === raidName.toLowerCase());
      if (fixedRaid && !selectedRaidIds.includes(fixedRaid.id)) {
        selectedRaidIds = [...selectedRaidIds, fixedRaid.id];
      } else if (!fixedRaid && raidName) {
        customRaids = [
          ...customRaids,
          {
            id: `custom-${Date.now()}-${raidName.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`,
            name: raidName,
            custom: true,
            ...buildRaidSignupComposition(8)
          }
        ];
      }
    }

    note = [
      request.details,
      request.dateWindow ? `Requested server date: ${request.dateWindow}` : '',
      request.timeWindow ? `Requested server time: ${request.timeWindow}` : '',
      startsAtLocal ? `Converted Discord time: ${formatDiscordTimestamp(startsAtLocal, 'F')}` : '',
      `Requester can do sidereals: ${request.canDoSidereals ? 'yes' : 'no'}`,
      `Requester: ${request.requester}`
    ].filter(Boolean).join('\n');
    preRegisterDiscordId = request.discordId;
    preRegisterDisplayName = request.requester;
    preRegisterStatus = request.canDoSidereals ? 'leader' : 'learner';
    customCompositionEnabled = false;
    builderErrors = {};
    activeMode = 'sheets';
  }

  function addPreRegisteredMember() {
    const discordIdValue = preRegisterDiscordId.trim();
    if (!discordIdValue) return;
    const nextMember: RaidSignupPreRegisteredMember = {
      discordId: discordIdValue,
      displayName: preRegisterDisplayName.trim() || discordIdValue,
      role: preRegisterRole,
      status: preRegisterStatus === 'leader'
        ? 'leader'
        : runType === 'learning'
          ? preRegisterStatus
          : 'experienced'
    };
    preRegisteredMembers = [
      ...preRegisteredMembers.filter((member) => member.discordId !== nextMember.discordId),
      nextMember
    ];
    preRegisterDiscordId = '';
    preRegisterDisplayName = '';
  }

  function removePreRegisteredMember(discordIdValue: string) {
    preRegisteredMembers = preRegisteredMembers.filter((member) => member.discordId !== discordIdValue);
  }

  function editSheet(sheet: RaidSignupSheet) {
    const shared = isSharedSheet(sheet);
    title = sheet.title;
    runType = sheet.runType;
    selectedRaidOption = '';
    selectedRaidIds = [...sheet.raidIds];
    customRaids = [...(sheet.customRaids || [])];
    startsAtLocal = toLocalDateTimeInput(sheet.startsAt);
    dpsSpots = sheet.dpsSpots;
    supportSpots = sheet.supportSpots;
    anySpots = sheet.anySpots;
    experiencedRequired = sheet.experiencedRequired;
    note = sheet.note;
    preRegisteredMembers = [...(sheet.preRegisteredMembers || [])];
    editingSheetId = sheet.id;
    editingEventId = sheet.eventId || sheet.id;
    editingPublished = shared;
    customCompositionEnabled = false;
    builderErrors = {};
    publishMessage = shared
      ? `Editing published event ${editingEventId}. Updating will keep the same event id.`
      : `Editing draft ${sheet.eventId}.`;
    if (!shared) {
      deleteRaidSignupSheet(sheet.id);
    }
    refreshLocalState();
    activeMode = 'sheets';
  }

  function buildSheet(): RaidSignupSheet | null {
    const cleanTitle = title.trim() || `${formatRunType(runType)} Signup`;
    if (!validateSheetForm()) return null;

    return {
      id: editingSheetId || `raid-signup-${Date.now()}`,
      eventId: editingEventId || createEventId(),
      title: cleanTitle,
      runType,
      raidIds: selectedRaidIds,
      customRaids,
      startsAt: startsAtDiscord,
      dpsSpots: customCompositionEnabled ? dpsSpots : suggestedDpsSpots,
      supportSpots: customCompositionEnabled ? supportSpots : suggestedSupportSpots,
      anySpots,
      experiencedRequired: runType === 'learning' ? experiencedRequired : 0,
      note: note.trim(),
      preRegisteredMembers,
      createdAt: Date.now(),
      updatedAt: Date.now()
    };
  }

  function resetSheetForm() {
    title = '';
    selectedRaidOption = '';
    selectedRaidIds = [];
    customRaids = [];
    startsAtLocal = '';
    anySpots = 0;
    customCompositionEnabled = false;
    builderErrors = {};
    note = '';
    preRegisteredMembers = [];
    preRegisterDiscordId = '';
    preRegisterDisplayName = '';
    editingSheetId = '';
    editingEventId = '';
    editingPublished = false;
  }

  function createSheet() {
    const sheet = buildSheet();
    if (!sheet) return;

    const draftSheet = editingPublished
      ? {
          ...sheet,
          id: `raid-signup-${Date.now()}`,
          eventId: createEventId(),
          createdAt: Date.now(),
          updatedAt: Date.now()
        }
      : sheet;

    saveRaidSignupSheet(draftSheet);
    publishMessage = `Saved draft ${draftSheet.eventId}.`;
    resetSheetForm();
    refreshLocalState();
  }

  async function updatePublishedSheet() {
    const sheet = buildSheet();
    if (!sheet || publishLoading || !editingPublished) return;

    try {
      publishLoading = true;
      publishMessage = '';
      await updateRaidSignupSheet(sheet);
      publishMessage = `Updated ${sheet.eventId}. Meowtator should refresh the Discord signup shortly.`;
      resetSheetForm();
      await refreshRaidManagementData();
    } catch (error) {
      publishMessage = `Update failed: ${error instanceof Error ? error.message : String(error)}`;
      console.warn('Failed to update raid signup sheet:', error);
    } finally {
      publishLoading = false;
    }
  }

  async function publishExistingSheet(sheet: RaidSignupSheet) {
    if (publishLoading || isSharedSheet(sheet)) return;
    try {
      publishLoading = true;
      publishMessage = '';
      await publishRaidSignupSheet({
        ...sheet,
        preRegisteredMembers: sheet.preRegisteredMembers || []
      });
      deleteRaidSignupSheet(sheet.id);
      publishMessage = `Published ${sheet.eventId}. Meowtator should post it within 30 seconds.`;
      refreshLocalState();
      void refreshRaidManagementData();
    } catch (error) {
      publishMessage = `Publish failed: ${error instanceof Error ? error.message : String(error)}`;
      console.warn('Failed to publish saved raid signup sheet:', error);
    } finally {
      publishLoading = false;
    }
  }

  async function publishAllDrafts() {
    if (publishLoading || orderedDraftSheets.length === 0) return;
    const draftsToPublish = [...orderedDraftSheets];
    try {
      publishLoading = true;
      publishMessage = '';
      for (const sheet of draftsToPublish) {
        await publishRaidSignupSheet({
          ...sheet,
          preRegisteredMembers: sheet.preRegisteredMembers || []
        });
        deleteRaidSignupSheet(sheet.id);
      }
      publishMessage = `Published ${draftsToPublish.length} draft${draftsToPublish.length === 1 ? '' : 's'} in start-time order.`;
      refreshLocalState();
      void refreshRaidManagementData();
    } catch (error) {
      publishMessage = `Publish failed: ${error instanceof Error ? error.message : String(error)}`;
      console.warn('Failed to publish all raid signup drafts:', error);
    } finally {
      publishLoading = false;
    }
  }

  async function grantAccess() {
    const member = whitelistMembers.find((value) => value.id === selectedAccessDiscordId);
    if (!member) return;

    try {
      const accessMember = {
        discordId: member.id,
        displayName: member.name
      };
      setRaidManagementAccessMember(accessMember);
      await grantRaidManagementAccessMember(accessMember, discordId);
      selectedAccessDiscordId = '';
      accessMembers = await loadRaidManagementAccessMembers();
    } catch (error) {
      globalRefreshError = error instanceof Error ? error.message : String(error);
      console.warn('Failed to grant raid management access:', error);
      refreshLocalState();
    }
  }

  async function revokeAccess(discordIdValue: string) {
    try {
      removeRaidManagementAccessMember(discordIdValue);
      await revokeRaidManagementAccessMember(discordIdValue);
      accessMembers = await loadRaidManagementAccessMembers();
    } catch (error) {
      globalRefreshError = error instanceof Error ? error.message : String(error);
      console.warn('Failed to revoke raid management access:', error);
      refreshLocalState();
    }
  }

  function removeSheet(sheetId: string) {
    deleteRaidSignupSheet(sheetId);
    refreshLocalState();
  }

  async function deleteSignupMember(sheet: RaidSignupSheet, member: RaidSignupPreRegisteredMember) {
    try {
      eventEditMessage = '';
      await removeRaidSignupEntry(sheet.id, member.discordId);
      eventEditMessage = `Removed ${member.displayName} from ${sheet.eventId}.`;
      await refreshRaidManagementData();
    } catch (error) {
      eventEditMessage = `Remove failed: ${error instanceof Error ? error.message : String(error)}`;
      console.warn('Failed to remove signup member:', error);
    }
  }

  async function deleteOngoingSignup(sheet: RaidSignupSheet) {
    const confirmed = window.confirm(`Delete ${sheet.title} and remove the Discord signup message?`);
    if (!confirmed) return;

    try {
      eventEditMessage = '';
      await cancelRaidSignupSheet(sheet.id);
      eventEditMessage = `Deleted ${sheet.title}. Meowtator will remove the Discord message shortly.`;
      await refreshRaidManagementData();
    } catch (error) {
      eventEditMessage = `Delete failed: ${error instanceof Error ? error.message : String(error)}`;
      console.warn('Failed to delete ongoing signup:', error);
    }
  }

  $: filteredRequests = requests.filter((request) => {
    const matchesStatus = requestFilter === 'all' || request.status === requestFilter;
    const matchesDoneState =
      request.status !== 'closed' || showDoneRequests;
    const haystack = [
      request.title,
      request.requester,
      request.discordId,
      request.raidNames.join(' '),
      request.category,
      request.canDoSidereals ? 'sidereal leader host yes' : 'sidereal no',
      request.decidedBy,
      formatReviewerName(request.decidedBy),
      request.reviewNote
    ].join(' ').toLowerCase();
    return matchesStatus && matchesDoneState && haystack.includes(requestSearch.trim().toLowerCase());
  });
  $: upcomingSheets = sheets.filter((sheet) => isSheetUpcomingOrOngoing(sheet));

  $: if (hasAccess && activeMode === 'configuration') {
    void loadWhitelist();
  }

  refreshLocalState();
</script>

<section class="raid-management">
  <header class="feature-header">
    <div>
      <p class="eyebrow">MeowGang</p>
      <h2>Raid Management</h2>
      <p>Create Discord signup sheet drafts for learning, reclear, and multi-raid evenings.</p>
    </div>
    {#if hasAccess}
      <div class="global-refresh">
        <button class="secondary-action" disabled={globalRefreshLoading} on:click={refreshRaidManagementData}>
          {globalRefreshLoading ? 'Loading...' : 'Load Data'}
        </button>
        {#if globalRefreshError}
          <small class="error-text">{globalRefreshError}</small>
        {:else if globalRefreshLoadedAt}
          <small>Last refresh: {formatDateTime(globalRefreshLoadedAt)}</small>
        {/if}
      </div>
    {/if}
  </header>

  {#if !hasAccess}
    <article class="access-card">
      <p class="eyebrow">Restricted</p>
      <h3>Raid Management access required</h3>
      <p>
        This area is limited to selected Discord IDs. Ask a Raid Management admin to add
        <strong>{discordName || discordId || 'your Discord account'}</strong>.
      </p>
    </article>
  {:else}
    <div class="mode-tabs">
      <button class:active={activeMode === 'sheets'} on:click={() => activeMode = 'sheets'}>Signup Sheets</button>
      <button class:active={activeMode === 'requests'} on:click={() => activeMode = 'requests'}>Requests</button>
      <button class:active={activeMode === 'events'} on:click={() => activeMode = 'events'}>On-going Events</button>
      <button class:active={activeMode === 'configuration'} on:click={() => activeMode = 'configuration'}>Configuration</button>
    </div>

    {#if activeMode === 'sheets'}
      <div class="management-grid">
        <article class="sheet-builder">
          <h3>Create signup sheet</h3>
          <label>
            Title
            <input bind:value={title} placeholder="e.g. Kazeros Learning Night" />
          </label>

          <label>
            Run type
            <select bind:value={runType}>
              <option value="learning">Learning</option>
              <option value="reclear">Reclear</option>
              <option value="raid-night">Raid Night</option>
            </select>
          </label>

          <label>
            Start time
            <input class:field-error={builderErrors.startsAt} type="datetime-local" bind:value={startsAtLocal} />
            {#if builderErrors.startsAt}
              <small class="field-error-text">{builderErrors.startsAt}</small>
            {/if}
          </label>

          {#if startsAtDiscord}
            <div class="timestamp-preview">
              <span>Discord time</span>
              <code>{startsAtDiscord}</code>
              <small>{startsAtRelative}</small>
            </div>
          {/if}

          <div class="raid-select" class:field-group-error={builderErrors.raids}>
            <span>Raid</span>
            <div class="raid-select-row">
              <select bind:value={selectedRaidOption}>
                <option value="">Choose raid</option>
                {#each RAID_SIGNUP_RAIDS as raid}
                  <option value={raid.id}>{raid.name} ({raid.dpsSpots} DPS / {raid.supportSpots} SUP)</option>
                {/each}
                <option value="custom">Custom raid / raid train</option>
              </select>
              <button
                type="button"
                disabled={!selectedRaidOption || selectedRaidOption === 'custom'}
                on:click={addSelectedRaid}
              >
                Add
              </button>
            </div>

            {#if selectedRaidOption === 'custom'}
              <div class="custom-raid-inputs">
                <input bind:value={customRaidName} placeholder="e.g. Random reclear train" />
                <input type="number" min="1" bind:value={customRaidSpots} aria-label="Custom raid spots" />
                <button type="button" on:click={addCustomRaid}>Add custom</button>
              </div>
            {/if}

            {#if selectedRaids.length > 0}
              <div class="selected-raid-list">
                {#each selectedRaids as raid}
                  <button
                    type="button"
                    on:click={() => raid.custom ? removeCustomRaid(raid.id) : removeFixedRaid(raid.id)}
                  >
                    {raid.name}
                    <small>{raid.dpsSpots} DPS / {raid.supportSpots} SUP</small>
                  </button>
                {/each}
              </div>
            {/if}
            {#if builderErrors.raids}
              <small class="field-error-text">{builderErrors.raids}</small>
            {/if}
          </div>

          <label class="inline-check compact-check">
            <input type="checkbox" bind:checked={customCompositionEnabled} />
            Customize DPS/SUP/ANY spots
          </label>

          <div class="role-grid" class:field-group-error={builderErrors.composition}>
            <label>
              DPS
              <input type="number" min="0" bind:value={dpsSpots} disabled={!customCompositionEnabled} />
            </label>
            <label>
              SUP
              <input type="number" min="0" bind:value={supportSpots} disabled={!customCompositionEnabled} />
            </label>
            <label>
              ANY
              <input type="number" min="0" bind:value={anySpots} disabled={!customCompositionEnabled} />
            </label>
            {#if runType === 'learning'}
              <label>
                Minimum experienced
                <input class:field-error={builderErrors.experienced} type="number" min="0" bind:value={experiencedRequired} />
              </label>
            {/if}
          </div>
          {#if builderErrors.composition}
            <small class="field-error-text">{builderErrors.composition}</small>
          {/if}
          {#if builderErrors.experienced}
            <small class="field-error-text">{builderErrors.experienced}</small>
          {/if}

          <button
            class="secondary-action"
            type="button"
            disabled={selectedRaidCapacity === 0 || !customCompositionEnabled}
            on:click={applySuggestedComposition}
          >
            Use suggested {suggestedDpsSpots} DPS / {suggestedSupportSpots} SUP
          </button>

          <label>
            Note
            <textarea bind:value={note} rows="3" placeholder="Optional signup instructions or requirements"></textarea>
          </label>

          <div class="pre-register-panel">
            <div>
              <strong>Pre-register</strong>
              <small>Add requester or planned helpers before publishing.</small>
            </div>
            <div class="pre-register-grid">
              <input bind:value={preRegisterDiscordId} placeholder="Discord ID" />
              <input bind:value={preRegisterDisplayName} placeholder="Display name" />
              <select bind:value={preRegisterRole}>
                <option value="dps">DPS</option>
                <option value="support">SUP</option>
                <option value="any">ANY</option>
              </select>
              <select bind:value={preRegisterStatus}>
                <option value="learner">Learning Kitten</option>
                <option value="experienced">Experienced</option>
                <option value="leader">Leader / Sidereal</option>
                <option value="can_help">Can Help</option>
              </select>
              <button type="button" disabled={!preRegisterDiscordId.trim()} on:click={addPreRegisteredMember}>
                Add
              </button>
            </div>

            {#if preRegisteredMembers.length > 0}
              <div class="pre-register-list">
                {#each preRegisteredMembers as member}
                  <button type="button" on:click={() => removePreRegisteredMember(member.discordId)}>
                    {member.displayName}
                    <small>{member.role.toUpperCase()} | {member.status.replace('_', ' ')}</small>
                  </button>
                {/each}
              </div>
            {/if}
          </div>

          <div class="sheet-summary">
            <span>Raid capacity: {selectedRaidCapacity}</span>
            <span>Configured spots: {selectedRoleTotal}</span>
          </div>

          {#if publishMessage}
            <p class:published-message-error={publishMessage.startsWith('Publish failed')} class="published-message">
              {publishMessage}
            </p>
          {/if}

          <div class="sheet-actions">
            <button class="secondary-action" disabled={selectedRaidCount === 0} on:click={createSheet}>
              {editingPublished ? 'Save as Draft Copy' : 'Save Draft'}
            </button>
            {#if editingPublished}
              <button class="primary-action" disabled={selectedRaidCount === 0 || publishLoading} on:click={updatePublishedSheet}>
                {publishLoading ? 'Updating...' : 'Update Event'}
              </button>
            {/if}
          </div>
        </article>

        <article class="sheet-list">
          <div class="panel-heading">
            <div>
              <h3>Draft preview</h3>
              <p class="muted">Publishing happens from saved drafts only.</p>
            </div>
            {#if orderedDraftSheets.length > 1}
              <button type="button" class="secondary-action" disabled={publishLoading} on:click={publishAllDrafts}>
                {publishLoading ? 'Publishing...' : 'Publish All'}
              </button>
            {/if}
          </div>
          {#if orderedDraftSheets.length === 0}
            <p class="muted">No signup drafts yet.</p>
          {:else}
            {#each orderedDraftSheets as sheet}
              <div class="discord-preview">
                <div class="discord-preview-accent"></div>
                <div class="discord-preview-body">
                  <div class="discord-preview-header">
                    <div>
                      <strong>{sheet.title}</strong>
                      <span>{formatRunType(sheet.runType)}</span>
                    </div>
                    <div class="preview-actions">
                      <button type="button" on:click={() => editSheet(sheet)}>
                        Edit
                      </button>
                      <button type="button" disabled={publishLoading} on:click={() => publishExistingSheet(sheet)}>
                        Publish
                      </button>
                      <button type="button" on:click={() => removeSheet(sheet.id)}>Delete</button>
                    </div>
                  </div>

                  <div class="discord-preview-field">
                    <small>Event Info</small>
                    <p>Event ID: {sheet.eventId || sheet.id}</p>
                    <p>Raids: {getSheetRaidNames(sheet).join(', ') || 'No raid selected'}</p>
                    <p>Start: {sheet.startsAt || 'No time set'}</p>
                  </div>

                  <div class="discord-preview-field">
                    <small>Lobby Host / Sidereal</small>
                    {#if getSheetLeader(sheet)}
                      <p class="signup-member">
                        <img src={getRoleIcon(getSheetLeader(sheet)?.role || 'dps')} alt="" />
                        {getSheetLeader(sheet)?.displayName}
                        <img src={roleIcons.leader} alt="Leader" />
                      </p>
                    {:else}
                      <p>Open leader spot</p>
                    {/if}
                  </div>

                  <div class="discord-signup-columns">
                    <div>
                      <strong>DPS</strong>
                      {#if getSheetMembers(sheet, 'dps').length}
                        {#each getSheetMembers(sheet, 'dps') as member}
                          <span class="signup-member">
                            <img src={getRoleIcon(member.role)} alt="" />
                            {member.displayName}
                            {#if getStatusIcon(member.status)}
                              <img src={getStatusIcon(member.status)} alt="" />
                            {/if}
                          </span>
                        {/each}
                      {:else}
                        <span>-</span>
                      {/if}
                    </div>
                    <div>
                      <strong>SUP</strong>
                      {#if getSheetMembers(sheet, 'support').length}
                        {#each getSheetMembers(sheet, 'support') as member}
                          <span class="signup-member">
                            <img src={getRoleIcon(member.role)} alt="" />
                            {member.displayName}
                            {#if getStatusIcon(member.status)}
                              <img src={getStatusIcon(member.status)} alt="" />
                            {/if}
                          </span>
                        {/each}
                      {:else}
                        <span>-</span>
                      {/if}
                    </div>
                    <div>
                      <strong>ANY</strong>
                      {#if getSheetMembers(sheet, 'any').length}
                        {#each getSheetMembers(sheet, 'any') as member}
                          <span class="signup-member">
                            <img src={getRoleIcon(member.role)} alt="" />
                            {member.displayName}
                            {#if getStatusIcon(member.status)}
                              <img src={getStatusIcon(member.status)} alt="" />
                            {/if}
                          </span>
                        {/each}
                      {:else}
                        <span>-</span>
                      {/if}
                    </div>
                  </div>

                  {#if sheet.note}
                    <div class="discord-preview-field">
                      <small>Note</small>
                      <p>{sheet.note}</p>
                    </div>
                  {/if}

                  <div class="discord-backup">
                    <strong>Can Help</strong>
                    {#if getSheetCanHelpMembers(sheet).length}
                      {#each getSheetCanHelpMembers(sheet) as member}
                        <span class="signup-member">
                          <img src={getRoleIcon(member.role)} alt="" />
                          {member.displayName}
                          {#if getStatusIcon(member.status)}
                            <img src={getStatusIcon(member.status)} alt="" />
                          {/if}
                        </span>
                      {/each}
                    {:else}
                      <span>-</span>
                    {/if}
                  </div>

                  <div class="discord-preview-actions">
                    <span><img src={roleIcons.dps} alt="" /> DPS</span>
                    <span><img src={roleIcons.support} alt="" /> SUP</span>
                    <span>ANY</span>
                    <span class="danger">Sign Off</span>
                    <small>No signups yet - {getSheetRaidCount(sheet)} raid{getSheetRaidCount(sheet) === 1 ? '' : 's'}</small>
                  </div>
                  {#if sheet.runType === 'learning'}
                    <small class="discord-preview-hint">
                      DPS/SUP/ANY opens: Learning Kitten, Experienced, Can Help, or Leader while the leader spot is open.
                    </small>
                  {:else}
                    <small class="discord-preview-hint">
                      DPS/SUP/ANY opens: Can Help or Leader while the leader spot is open. Reclear signups treat normal signups as experienced by default.
                    </small>
                  {/if}
                  <small class="discord-event-id">Event ID: {sheet.eventId || sheet.id}</small>
                </div>
              </div>
            {/each}
          {/if}
        </article>
      </div>
    {:else if activeMode === 'requests'}
      <article class="access-card">
        <div class="panel-heading">
          <div>
            <h3>Raid requests</h3>
            <p class="muted">
              Accepted and declined Meowtator requests. Use refresh when you want the latest bot backlog state.
            </p>
          </div>
        </div>

        {#if requestsError}
          <p class="error-text">Could not load requests: {requestsError}</p>
        {/if}

        <div class="request-filters">
          <label>
            Status
            <select bind:value={requestFilter}>
              <option value="all">Accepted + Declined</option>
              <option value="accepted">Accepted</option>
              <option value="declined">Declined</option>
              <option value="closed">Done</option>
            </select>
          </label>
          <label>
            Search
            <input
              bind:value={requestSearch}
              placeholder="Discord name/id, date, raid, type, or manager"
            />
          </label>
          <label class="inline-check">
            <input type="checkbox" bind:checked={showDoneRequests} />
            Show done
          </label>
        </div>

        <div class="request-list">
          {#if filteredRequests.length === 0}
            <p class="muted">No accepted or declined requests loaded yet.</p>
          {/if}
          {#each filteredRequests as request}
            <div class="request-row">
              <div>
                <strong>{request.title}</strong>
                <span>{request.requester}</span>
              </div>
              <div>
                <small>Raid</small>
                <span>{request.raidNames.join(', ')}</span>
              </div>
              <div>
                <small>Type</small>
                <span>{request.category}</span>
              </div>
              <div>
                <small>Server time</small>
                <span>{request.dateWindow} | {request.timeWindow}</span>
                <small>Requested {formatDateTime(request.createdAt)}</small>
              </div>
              <div>
                <small>Sidereals</small>
                <span>{request.canDoSidereals ? 'Can lead' : 'Needs leader'}</span>
              </div>
              <div>
                <small>
                  {request.status === 'declined' ? 'Declined by' : request.status === 'closed' ? 'Done by' : 'Accepted by'}
                </small>
                <span>{formatReviewerName(request.decidedBy)}</span>
              </div>
              <span
                class:accepted={request.status === 'accepted'}
                class:declined={request.status === 'declined'}
                class:closed={request.status === 'closed'}
                class="request-status"
              >
                {request.status === 'closed' ? 'done' : request.status}
              </span>
              {#if request.status === 'declined' && request.reviewNote}
                <span class="request-info" title={request.reviewNote} aria-label={`Decline reason: ${request.reviewNote}`}>
                  i
                </span>
              {/if}
              {#if request.status === 'accepted' || request.status === 'closed'}
                <button type="button" class="request-use-button" on:click={() => useRequest(request)}>
                  Use
                </button>
                {#if isRequestDone(request)}
                  <button type="button" class="request-use-button" on:click={() => reopenRequest(request)}>
                    Reopen
                  </button>
                {:else}
                  <button type="button" class="request-use-button" on:click={() => markRequestDone(request)}>
                    Done
                  </button>
                {/if}
              {/if}
            </div>
          {/each}
        </div>
      </article>
    {:else if activeMode === 'events'}
      <article class="access-card">
        <div class="panel-heading">
          <div>
            <h3>On-going events</h3>
            <p class="muted">Signup sheets stay visible here until two hours after their start time.</p>
          </div>
        </div>

        {#if upcomingSheets.length === 0}
          <p class="muted">No planned or on-going events yet.</p>
        {:else}
          {#if eventEditMessage}
            <p class:error-text={eventEditMessage.includes('failed')} class="event-edit-message">{eventEditMessage}</p>
          {/if}
          <div class="event-list">
            {#each upcomingSheets as sheet}
              <div class="event-row">
                <div>
                  <strong>{sheet.title}</strong>
                  <span>{sheet.eventId}</span>
                </div>
                <div>
                  <small>Raid(s)</small>
                  <span>{getSheetRaidNames(sheet).join(', ') || 'No raid selected'}</span>
                </div>
                <div>
                  <small>Type</small>
                  <span>{formatRunType(sheet.runType)}</span>
                </div>
                <div>
                  <small>Start</small>
                  <span>{sheet.startsAt || 'No time set'}</span>
                </div>
                <div class="event-actions">
                  <button type="button" title="Edit in builder" on:click={() => editSheet(sheet)}>
                    Edit
                  </button>
                  <button
                    type="button"
                    class="danger-action"
                    title="Delete signup"
                    on:click={() => deleteOngoingSignup(sheet)}
                  >
                    Delete
                  </button>
                </div>
                {#if sheet.preRegisteredMembers?.length}
                  <div class="event-signups">
                    <small>Signups</small>
                    <div class="event-signup-groups">
                      {#each getSheetEventGroups(sheet) as group, groupIndex}
                        {#if groupIndex > 0}
                          <span class="event-signup-separator" aria-hidden="true">|</span>
                        {/if}
                        <div class="event-signup-group">
                          <strong>{group.label}</strong>
                          <div>
                            {#each group.members as member}
                              <button type="button" on:click={() => deleteSignupMember(sheet, member)}>
                                <img src={getRoleIcon(member.role)} alt="" />
                                {member.displayName}
                                {#if getStatusIcon(member.status)}
                                  <img src={getStatusIcon(member.status)} alt="" />
                                {/if}
                              </button>
                            {/each}
                          </div>
                        </div>
                      {/each}
                    </div>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </article>
    {:else if activeMode === 'configuration'}
      <article class="access-card">
        <h3>Allowed Discord IDs</h3>
        <p class="muted">Grant access to raid managers and keep the bot handoff basics in one place.</p>

        <div class="access-picker">
          <select bind:value={selectedAccessDiscordId} on:focus={loadWhitelist}>
            <option value="">Select whitelisted member</option>
            {#each accessibleWhitelistMembers as member}
              <option value={member.id}>{member.name} ({member.id})</option>
            {/each}
          </select>
          <button disabled={!selectedAccessDiscordId} on:click={grantAccess}>Grant</button>
        </div>

        <div class="access-list">
          {#each accessMembers as member}
            <div class="access-row">
              <span>{member.displayName}</span>
              <small>{member.discordId}</small>
              <button on:click={() => revokeAccess(member.discordId)}>Remove</button>
            </div>
          {/each}
        </div>

        <h3>Discord bot handoff</h3>
        <p>
          The bot should be a separate private Node service. The desktop app can prepare the
          signup sheet data, but the bot token must never be bundled into the app.
        </p>
        <ul>
          <li>Bot reads approved signup sheets from a backend table/API.</li>
          <li>Bot posts embeds with DPS/SUP/ANY signup buttons or select menus.</li>
          <li>Learning runs show experienced as a minimum target while helpers still sign as DPS or SUP.</li>
          <li>ANY fills missing DPS or SUP needs but still counts toward total raid capacity.</li>
          <li>Each 4-player party should have one support where possible.</li>
          <li>Reclear runs support multiple raids in one evening.</li>
        </ul>
      </article>
    {/if}
  {/if}
</section>

<style>
  .raid-management {
    width: 100%;
    max-width: 1180px;
    margin: 0 auto;
    padding: 0.75rem;
    color: var(--md-sys-color-on-surface);
    box-sizing: border-box;
  }

  .feature-header,
  .access-card,
  .sheet-builder,
  .sheet-list {
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface-container);
    box-shadow: var(--app-shadow-sm);
  }

  .feature-header {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: center;
    padding: 0.45rem 0.65rem;
    margin-bottom: 0.55rem;
  }

  .global-refresh {
    display: grid;
    justify-items: end;
    gap: 0.15rem;
    min-width: 11rem;
  }

  .global-refresh small {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
    text-align: right;
  }

  .feature-header h2,
  .feature-header p,
  .access-card h3,
  .access-card p,
  .sheet-builder h3,
  .sheet-list h3 {
    margin: 0;
  }

  .feature-header h2 {
    font-size: 1rem;
    line-height: 1.15;
  }

  .feature-header p:not(.eyebrow) {
    font-size: 0.76rem;
    line-height: 1.2;
  }

  .feature-header p,
  .muted,
  .access-card p {
    color: var(--md-sys-color-on-surface-variant);
  }

  .error-text {
    margin: 0;
    color: var(--md-sys-color-error);
    font-size: 0.82rem;
  }

  .eyebrow {
    margin: 0 0 0.25rem;
    color: var(--md-sys-color-primary);
    font-size: 0.62rem;
    font-weight: 800;
    text-transform: uppercase;
  }

  .mode-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.8rem;
  }

  .mode-tabs button,
  .primary-action,
  .secondary-action,
  .raid-select-row button,
  .custom-raid-inputs button,
  .selected-raid-list button,
  .discord-preview-header button,
  .access-picker button,
  .access-row button {
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 6px;
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font: inherit;
  }

  .mode-tabs button,
  .primary-action,
  .secondary-action,
  .access-picker button {
    padding: 0.55rem 0.8rem;
  }

  .mode-tabs button.active,
  .primary-action {
    border-color: var(--md-sys-color-primary);
    background: var(--md-sys-color-primary-container);
    color: var(--md-sys-color-on-primary-container);
  }

  .management-grid {
    display: grid;
    grid-template-columns: minmax(300px, 390px) minmax(0, 1fr);
    gap: 0.8rem;
    align-items: start;
    min-width: 0;
  }

  .sheet-builder,
  .sheet-list,
  .access-card {
    display: grid;
    gap: 0.6rem;
    min-width: 0;
    padding: 0.8rem;
  }

  .panel-heading {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: start;
  }

  .request-filters {
    display: grid;
    grid-template-columns: minmax(12rem, 16rem) minmax(0, 1fr) max-content;
    gap: 0.6rem;
    align-items: end;
  }

  .inline-check {
    display: inline-flex;
    gap: 0.4rem;
    align-items: center;
    min-height: 2.3rem;
    white-space: nowrap;
  }

  .inline-check input {
    width: auto;
  }

  .request-list,
  .event-list {
    display: grid;
    gap: 0.5rem;
  }

  .request-row,
  .event-row {
    display: grid;
    grid-template-columns: minmax(10rem, 1.3fr) minmax(8rem, 1fr) minmax(6rem, 0.65fr) minmax(10rem, 1fr) minmax(8rem, 0.8fr) auto auto auto;
    gap: 0.65rem;
    align-items: center;
    padding: 0.7rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 6px;
    background: var(--md-sys-color-surface);
  }

  .event-row {
    grid-template-columns: minmax(10rem, 1.1fr) minmax(10rem, 1fr) minmax(6rem, 0.6fr) minmax(10rem, 1fr) auto;
  }

  .request-row > div,
  .event-row > div {
    display: grid;
    gap: 0.15rem;
    min-width: 0;
  }

  .request-row strong,
  .event-row strong {
    overflow: hidden;
    color: var(--md-sys-color-on-surface);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .request-row span,
  .event-row span {
    overflow: hidden;
    color: var(--md-sys-color-on-surface);
    font-size: 0.8rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .request-row small,
  .event-row small {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.7rem;
  }

  .request-status {
    justify-self: end;
    border: 1px solid var(--md-sys-color-error);
    border-radius: 999px;
    color: var(--md-sys-color-on-error-container) !important;
    background: var(--md-sys-color-error-container);
    padding: 0.25rem 0.5rem;
    text-transform: capitalize;
  }

  .request-status.accepted {
    border-color: var(--md-sys-color-success);
    color: var(--md-sys-color-on-primary-container) !important;
    background: var(--md-sys-color-primary-container);
  }

  .request-status.closed {
    border-color: var(--md-sys-color-outline-variant);
    color: var(--md-sys-color-on-surface-variant) !important;
    background: var(--md-sys-color-surface-container-high);
  }

  .request-status.declined {
    border-color: var(--md-sys-color-error);
    color: var(--md-sys-color-on-error-container) !important;
    background: var(--md-sys-color-error-container);
  }

  .request-info {
    display: inline-grid;
    place-items: center;
    justify-self: end;
    width: 1.35rem;
    height: 1.35rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 50%;
    color: var(--md-sys-color-on-surface-variant);
    background: var(--md-sys-color-surface-container-high);
    cursor: help;
    font-size: 0.72rem;
    font-weight: 700;
    line-height: 1;
  }

  .request-use-button {
    border: 1px solid var(--md-sys-color-primary);
    border-radius: 6px;
    background: var(--md-sys-color-primary-container);
    color: var(--md-sys-color-on-primary-container);
    cursor: pointer;
    padding: 0.35rem 0.55rem;
    font: inherit;
    font-size: 0.76rem;
  }

  label {
    display: grid;
    gap: 0.3rem;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.78rem;
    font-weight: 650;
  }

  input,
  select,
  textarea {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 6px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    padding: 0.55rem;
    font: inherit;
  }

  input:disabled,
  select:disabled,
  textarea:disabled {
    opacity: 0.72;
    cursor: not-allowed;
  }

  .field-error,
  .field-group-error input,
  .field-group-error select {
    border-color: var(--md-sys-color-error) !important;
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--md-sys-color-error) 35%, transparent);
  }

  .field-error-text {
    color: var(--md-sys-color-error);
    font-size: 0.72rem;
    line-height: 1.2;
  }

  .compact-check {
    justify-content: start;
    min-height: auto;
    color: var(--md-sys-color-on-surface);
    font-size: 0.76rem;
  }

  .timestamp-preview,
  .raid-select {
    display: grid;
    gap: 0.4rem;
  }

  .timestamp-preview > span,
  .raid-select > span {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.78rem;
    font-weight: 650;
  }

  .timestamp-preview {
    padding: 0.55rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 6px;
    background: var(--md-sys-color-surface);
  }

  .timestamp-preview code {
    color: var(--md-sys-color-primary);
    font-family: inherit;
    font-size: 0.8rem;
  }

  .timestamp-preview small {
    color: var(--md-sys-color-on-surface-variant);
  }

  .raid-select-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) max-content;
    gap: 0.45rem;
  }

  .raid-select-row button,
  .custom-raid-inputs button {
    padding: 0.55rem 0.75rem;
  }

  .custom-raid-inputs {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(4.5rem, 5rem) max-content;
    gap: 0.4rem;
  }

  .selected-raid-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .selected-raid-list button {
    display: inline-flex;
    gap: 0.4rem;
    align-items: center;
    padding: 0.45rem 0.55rem;
    font-size: 0.78rem;
  }

  .selected-raid-list small {
    opacity: 0.72;
  }

  .role-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr));
    gap: 0.5rem;
  }

  .pre-register-panel {
    display: grid;
    gap: 0.5rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 6px;
    background: var(--md-sys-color-surface);
    padding: 0.65rem;
  }

  .pre-register-panel strong,
  .pre-register-panel small {
    display: block;
  }

  .pre-register-panel small {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
  }

  .pre-register-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.4rem;
  }

  .pre-register-grid button {
    grid-column: span 2;
  }

  .pre-register-grid button,
  .pre-register-list button,
  .preview-actions button {
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 6px;
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font: inherit;
  }

  .pre-register-grid button {
    padding: 0.45rem 0.6rem;
  }

  .pre-register-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .pre-register-list button {
    display: inline-grid;
    gap: 0.1rem;
    padding: 0.4rem 0.55rem;
    text-align: left;
  }

  .sheet-summary {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.78rem;
  }

  .sheet-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .published-message {
    margin: 0;
    color: var(--md-sys-color-success);
    font-size: 0.8rem;
  }

  .published-message.published-message-error {
    color: var(--md-sys-color-error);
  }

  .discord-preview {
    display: grid;
    grid-template-columns: 5px minmax(0, 1fr);
    overflow: hidden;
    border: 1px solid #3f4147;
    border-radius: 6px;
    background: #1e1f22;
    color: #f2f3f5;
  }

  .discord-preview-accent {
    background: var(--md-sys-color-primary);
  }

  .discord-preview-body {
    display: grid;
    gap: 0.58rem;
    min-width: 0;
    padding: 0.72rem;
  }

  .discord-preview-header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.6rem;
    align-items: start;
  }

  .preview-actions {
    display: flex;
    gap: 0.35rem;
    align-items: center;
  }

  .discord-preview-header strong,
  .discord-preview-header span,
  .discord-preview-field p {
    display: block;
    margin: 0;
  }

  .discord-preview-header strong {
    color: #ffffff;
    font-size: 0.95rem;
  }

  .discord-preview-header span,
  .discord-preview-field small {
    color: #b5bac1;
    font-size: 0.74rem;
  }

  .discord-preview-header button {
    padding: 0.35rem 0.5rem;
    color: #f2f3f5;
    font-size: 0.74rem;
  }

  .discord-preview-field p {
    color: #f2f3f5;
    font-size: 0.82rem;
    overflow-wrap: anywhere;
  }

  .discord-signup-columns {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .discord-signup-columns strong,
  .discord-backup strong {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    color: #ffffff;
    font-size: 0.82rem;
  }

  .discord-preview-actions img {
    width: 1rem;
    height: 1rem;
    object-fit: contain;
  }

  .discord-signup-columns span,
  .discord-backup span {
    display: block;
    color: #d7d9df;
    font-size: 0.8rem;
  }

  .signup-member {
    display: flex !important;
    gap: 0.25rem;
    align-items: center;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .signup-member img {
    width: 1rem;
    height: 1rem;
    flex: 0 0 auto;
    object-fit: contain;
  }

  .discord-backup {
    padding-top: 0.55rem;
    border-top: 1px dashed #6b6f78;
  }

  .discord-preview-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem;
    align-items: center;
    min-width: 0;
  }

  .discord-preview-actions span {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    border: 1px solid #5865f2;
    border-radius: 4px;
    background: #5865f2;
    color: #ffffff;
    padding: 0.35rem 0.5rem;
    font-size: 0.74rem;
    font-weight: 650;
  }

  .discord-preview-actions span.danger {
    border-color: #da373c;
    background: #da373c;
  }

  .discord-preview-actions small {
    color: #b5bac1;
    font-size: 0.74rem;
  }

  .discord-preview-hint {
    color: #b5bac1;
    font-size: 0.74rem;
  }

  .discord-event-id {
    justify-self: end;
    color: #8d939d;
    font-size: 0.62rem;
  }

  .event-edit-message {
    margin: 0;
    color: var(--md-sys-color-success);
    font-size: 0.8rem;
  }

  .event-actions {
    display: flex;
    gap: 0.3rem;
    align-items: center;
    justify-content: end;
  }

  .event-actions button,
  .event-signups button {
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 6px;
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font: inherit;
  }

  .event-actions button {
    min-width: 3.1rem;
    padding: 0.28rem 0.45rem;
    font-size: 0.72rem;
    line-height: 1.1;
  }

  .event-actions .danger-action {
    border-color: var(--md-sys-color-error);
    color: var(--md-sys-color-error);
  }

  .event-signups {
    grid-column: 1 / -1;
    display: grid;
    gap: 0.35rem;
  }

  .event-signup-groups {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    align-items: center;
  }

  .event-signup-group {
    display: inline-flex;
    gap: 0.35rem;
    align-items: center;
    min-width: 0;
  }

  .event-signup-group strong {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.68rem;
    font-weight: 600;
    text-transform: uppercase;
  }

  .event-signup-group > div {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    min-width: 0;
  }

  .event-signup-separator {
    color: var(--md-sys-color-outline-variant);
    font-size: 0.78rem;
  }

  .event-signups button {
    display: inline-flex;
    gap: 0.2rem;
    align-items: center;
    padding: 0.24rem 0.38rem;
    font-size: 0.72rem;
    line-height: 1.1;
  }

  .event-signups img {
    width: 1rem;
    height: 1rem;
    object-fit: contain;
  }

  .primary-action:disabled,
  .secondary-action:disabled,
  .raid-select-row button:disabled,
  .access-picker button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .access-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.6rem;
    padding: 0.65rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 6px;
    background: var(--md-sys-color-surface);
  }

  .access-row {
    min-width: 0;
  }

  .access-row small {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.76rem;
  }

  .access-picker {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.5rem;
  }

  @media (max-width: 1180px) {
    .management-grid {
      grid-template-columns: 1fr;
    }

    .sheet-builder {
      max-width: none;
    }
  }

  @media (max-width: 900px) {
    .feature-header {
      display: grid;
    }

    .global-refresh {
      justify-items: start;
    }

    .request-filters,
    .request-row,
    .event-row,
    .pre-register-grid {
      grid-template-columns: 1fr;
    }

    .pre-register-grid button {
      grid-column: auto;
    }

    .request-status {
      justify-self: start;
    }
  }
</style>
