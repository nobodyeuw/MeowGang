<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import MeowConnectFriendsPanel from './MeowConnectFriendsPanel.svelte';
  import MeowConnectGroupsPanel from './MeowConnectGroupsPanel.svelte';
  import MeowConnectRaidVisibility from './MeowConnectRaidVisibility.svelte';
  import MeowConnectSyncPanel from './MeowConnectSyncPanel.svelte';
  import type { Raid } from '$lib/data/raids';
  import type {
    MeowConnectFriendConnection,
    MeowConnectGroup,
    MeowConnectGroupMember,
    MeowConnectProfile
  } from '$lib/services/meow-connect';
  import type { FriendOption } from './types';

  export let activeGroupInviteId = '';
  export let activeGroupRenameId = '';
  export let connectedCharacterCount = 0;
  export let connectedFriends = 0;
  export let consentAccepted = false;
  export let filteredFriendOptions: FriendOption[] = [];
  export let friendActionBusy = false;
  export let friendConnections: MeowConnectFriendConnection[] = [];
  export let friendDiscordId = '';
  export let friendSearch = '';
  export let getGroupMemberName: (member: MeowConnectGroupMember) => string = () => 'Unknown';
  export let getInitials: (name: string) => string = () => '?';
  export let getProfileAvatar: (discordId: string) => string | undefined = () => undefined;
  export let groupActionBusy = false;
  export let groupInviteInputs: Record<string, string> = {};
  export let groupInviteOptions: MeowConnectProfile[] = [];
  export let groupName = '';
  export let groupRenameInputs: Record<string, string> = {};
  export let groupTag = '';
  export let hasUnsyncedChanges = false;
  export let lastSyncCaption = 'last synced';
  export let lastSyncValue = 'Never';
  export let meowGroups: MeowConnectGroup[] = [];
  export let ownedGroupCount = 0;
  export let pendingGroupInvites: MeowConnectGroup[] = [];
  export let pendingIncoming: MeowConnectFriendConnection[] = [];
  export let popoverElement: HTMLElement | null = null;
  export let raidOptions: Raid[] = [];
  export let resetText = 'No local snapshot loaded.';
  export let showFriendPopover = false;
  export let sortedFriendConnections: MeowConnectFriendConnection[] = [];
  export let syncDisabled = false;
  export let syncLabel = 'Sync now';
  export let syncTitle = 'Upload your current MeowConnect snapshot';
  export let unsyncedRosterChangeCount = '0';
  export let visibleRaidIds: string[] = [];

  const dispatch = createEventDispatcher<{
    acceptFriendRequest: MeowConnectFriendConnection;
    acceptGroupInvite: MeowConnectGroup;
    cancelGroupRename: MeowConnectGroup;
    clearRaidSelection: void;
    createGroup: void;
    deleteGroup: MeowConnectGroup;
    inviteGroupMember: MeowConnectGroup;
    leaveGroup: MeowConnectGroup;
    openFriendPopover: void;
    removeFriend: MeowConnectFriendConnection;
    renameGroup: MeowConnectGroup;
    scheduleInviteSearch: { groupId: string; query: string };
    selectAllRaids: void;
    selectFriendOption: FriendOption;
    selectInviteOption: { group: MeowConnectGroup; profile: MeowConnectProfile };
    sendFriendRequest: void;
    startGroupRename: MeowConnectGroup;
    sync: void;
    toggleRaidVisibility: string;
  }>();
</script>

<section class="settings-grid">
  <MeowConnectSyncPanel
    {resetText}
    {syncLabel}
    {syncTitle}
    {syncDisabled}
    {connectedCharacterCount}
    {unsyncedRosterChangeCount}
    {hasUnsyncedChanges}
    {lastSyncValue}
    {lastSyncCaption}
    on:sync={() => dispatch('sync')}
  />

  <MeowConnectRaidVisibility
    {raidOptions}
    {visibleRaidIds}
    on:selectAll={() => dispatch('selectAllRaids')}
    on:clear={() => dispatch('clearRaidSelection')}
    on:toggle={(event) => dispatch('toggleRaidVisibility', event.detail)}
  />

  <MeowConnectGroupsPanel
    bind:activeGroupInviteId
    bind:activeGroupRenameId
    bind:groupInviteInputs
    bind:groupName
    bind:groupRenameInputs
    bind:groupTag
    {getGroupMemberName}
    {getInitials}
    {groupActionBusy}
    {groupInviteOptions}
    {meowGroups}
    {ownedGroupCount}
    {pendingGroupInvites}
    on:acceptGroupInvite={(event) => dispatch('acceptGroupInvite', event.detail)}
    on:cancelGroupRename={(event) => dispatch('cancelGroupRename', event.detail)}
    on:createGroup={() => dispatch('createGroup')}
    on:deleteGroup={(event) => dispatch('deleteGroup', event.detail)}
    on:inviteGroupMember={(event) => dispatch('inviteGroupMember', event.detail)}
    on:leaveGroup={(event) => dispatch('leaveGroup', event.detail)}
    on:renameGroup={(event) => dispatch('renameGroup', event.detail)}
    on:scheduleInviteSearch={(event) => dispatch('scheduleInviteSearch', event.detail)}
    on:selectInviteOption={(event) => dispatch('selectInviteOption', event.detail)}
    on:startGroupRename={(event) => dispatch('startGroupRename', event.detail)}
  />

  <MeowConnectFriendsPanel
    bind:friendDiscordId
    bind:friendSearch
    bind:popoverElement
    bind:showFriendPopover
    {connectedFriends}
    {consentAccepted}
    {filteredFriendOptions}
    {friendActionBusy}
    {friendConnections}
    {getInitials}
    {getProfileAvatar}
    {pendingIncoming}
    {sortedFriendConnections}
    on:acceptFriendRequest={(event) => dispatch('acceptFriendRequest', event.detail)}
    on:openPopover={() => dispatch('openFriendPopover')}
    on:removeFriend={(event) => dispatch('removeFriend', event.detail)}
    on:selectFriendOption={(event) => dispatch('selectFriendOption', event.detail)}
    on:sendFriendRequest={() => dispatch('sendFriendRequest')}
  />
</section>

<style>
  .settings-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    align-items: start;
    gap: 0.85rem;
    padding: 1rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 12px;
    background: var(--md-sys-color-surface-container);
  }

  @media (max-width: 760px) {
    .settings-grid {
      grid-template-columns: 1fr;
      padding: 0.75rem;
    }
  }
</style>
