<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { MeowConnectGroup, MeowConnectGroupMember, MeowConnectProfile } from '$lib/services/meow-connect';

  export let meowGroups: MeowConnectGroup[] = [];
  export let ownedGroupCount = 0;
  export let pendingGroupInvites: MeowConnectGroup[] = [];
  export let groupName = '';
  export let groupTag = '';
  export let groupActionBusy = false;
  export let activeGroupRenameId = '';
  export let activeGroupInviteId = '';
  export let groupRenameInputs: Record<string, string> = {};
  export let groupInviteInputs: Record<string, string> = {};
  export let groupInviteOptions: MeowConnectProfile[] = [];
  export let getInitials: (name: string) => string = () => '?';
  export let getGroupMemberName: (member: MeowConnectGroupMember) => string = () => 'Unknown';

  const dispatch = createEventDispatcher<{
    createGroup: void;
    renameGroup: MeowConnectGroup;
    startGroupRename: MeowConnectGroup;
    cancelGroupRename: MeowConnectGroup;
    inviteGroupMember: MeowConnectGroup;
    scheduleInviteSearch: { groupId: string; query: string };
    selectInviteOption: { group: MeowConnectGroup; profile: MeowConnectProfile };
    deleteGroup: MeowConnectGroup;
    acceptGroupInvite: MeowConnectGroup;
    leaveGroup: MeowConnectGroup;
  }>();

  let expandedGroupMemberIds = new Set<string>();

  function getActiveGroupMembers(group: MeowConnectGroup): MeowConnectGroupMember[] {
    return group.members.filter((member) => member.status !== 'removed' && member.status !== 'declined');
  }

  function isGroupMembersExpanded(group: MeowConnectGroup): boolean {
    return expandedGroupMemberIds.has(group.groupId);
  }

  function toggleGroupMembersExpanded(group: MeowConnectGroup) {
    const next = new Set(expandedGroupMemberIds);
    if (next.has(group.groupId)) {
      next.delete(group.groupId);
    } else {
      next.add(group.groupId);
    }
    expandedGroupMemberIds = next;
  }

  function getVisibleGroupMembers(group: MeowConnectGroup): MeowConnectGroupMember[] {
    const members = getActiveGroupMembers(group);
    return isGroupMembersExpanded(group) ? members : members.slice(0, 4);
  }

  function getOverflowGroupMemberCount(group: MeowConnectGroup): number {
    if (isGroupMembersExpanded(group)) return 0;
    return Math.max(getActiveGroupMembers(group).length - getVisibleGroupMembers(group).length, 0);
  }

  function isGroupMemberStackCompact(group: MeowConnectGroup): boolean {
    return getActiveGroupMembers(group).length > 4 && !isGroupMembersExpanded(group);
  }
</script>

<article class="settings-panel" data-guide="meow-connect-groups">
  <div class="panel-title">
    <div>
      <h3>Groups</h3>
      <p>{ownedGroupCount} owned | {pendingGroupInvites.length} pending invite{pendingGroupInvites.length === 1 ? '' : 's'}</p>
    </div>
    <div class="group-create-control">
      <input
        bind:value={groupName}
        maxlength="24"
        placeholder="Group name"
        disabled={groupActionBusy}
        on:keydown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault();
            dispatch('createGroup');
          }
        }}
      />
      <input
        class="group-tag-input"
        bind:value={groupTag}
        maxlength="5"
        placeholder="Tag"
        title="Optional group tag, maximum 5 characters"
        disabled={groupActionBusy}
        on:input={() => groupTag = groupTag.toUpperCase()}
        on:keydown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault();
            dispatch('createGroup');
          }
        }}
      />
      <button
        class="primary-button"
        type="button"
        disabled={groupActionBusy || groupName.trim().length < 2}
        on:click={() => dispatch('createGroup')}
      >
        Create
      </button>
    </div>
  </div>

  <div class="group-list">
    {#if meowGroups.length === 0}
      <p class="column-empty">No groups yet.</p>
    {:else}
      {#each meowGroups as group}
        <div class:invited={group.role === 'invited'} class="group-row">
          <div class="group-row-main">
            <div class="group-title-line">
              <strong>
                {#if group.groupTag}
                  <span class="group-tag-chip">{group.groupTag}</span>
                {/if}
                {group.groupName}
              </strong>
              <span class="group-role-label">{group.role === 'owner' ? 'Owner' : group.role === 'invited' ? 'Invite pending' : 'Member'}</span>
            </div>
            <div
              class:compact={isGroupMemberStackCompact(group)}
              class:expanded={isGroupMembersExpanded(group)}
              class="group-member-stack"
              title={getActiveGroupMembers(group).map(getGroupMemberName).join(', ') || 'No members yet'}
            >
              {#each getVisibleGroupMembers(group) as member}
                {#if member.profile?.avatarUrl}
                  <img src={member.profile.avatarUrl} alt={getGroupMemberName(member)} />
                {:else}
                  <span class="avatar-fallback">{getInitials(getGroupMemberName(member))}</span>
                {/if}
              {/each}
              {#if getOverflowGroupMemberCount(group) > 0}
                <button
                  class="group-avatar-overflow"
                  type="button"
                  aria-label={`Show all ${getActiveGroupMembers(group).length} group members`}
                  title="Show all members"
                  on:click={() => toggleGroupMembersExpanded(group)}
                >
                  +{getOverflowGroupMemberCount(group)}
                </button>
              {:else if isGroupMembersExpanded(group) && getActiveGroupMembers(group).length > 4}
                <button class="group-member-collapse" type="button" on:click={() => toggleGroupMembersExpanded(group)}>
                  Show less
                </button>
              {/if}
            </div>
          </div>

          {#if group.role === 'owner'}
            <div class="group-owner-controls">
              {#if activeGroupRenameId === group.groupId}
                <div class="group-rename-control active">
                  <input
                    value={groupRenameInputs[group.groupId] || group.groupName}
                    maxlength="24"
                    placeholder="Group name"
                    disabled={groupActionBusy}
                    on:input={(event) => {
                      const input = event.currentTarget as HTMLInputElement;
                      groupRenameInputs = { ...groupRenameInputs, [group.groupId]: input.value };
                    }}
                    on:keydown={(event) => {
                      if (event.key === 'Enter') {
                        event.preventDefault();
                        dispatch('renameGroup', group);
                      } else if (event.key === 'Escape') {
                        dispatch('cancelGroupRename', group);
                      }
                    }}
                  />
                  <button
                    class="mini-button"
                    type="button"
                    disabled={groupActionBusy || !(groupRenameInputs[group.groupId] || group.groupName).trim() || (groupRenameInputs[group.groupId] || group.groupName).trim() === group.groupName}
                    on:click={() => dispatch('renameGroup', group)}
                  >
                    Save
                  </button>
                  <button
                    class="mini-button subtle"
                    type="button"
                    disabled={groupActionBusy}
                    on:click={() => dispatch('cancelGroupRename', group)}
                  >
                    Cancel
                  </button>
                </div>
              {:else}
                <button
                  class="mini-button"
                  type="button"
                  disabled={groupActionBusy}
                  on:click={() => dispatch('startGroupRename', group)}
                >
                  Rename
                </button>
              {/if}

              <div class="group-invite-control">
                <input
                  value={groupInviteInputs[group.groupId] || ''}
                  placeholder="Type MeowConnect name"
                  disabled={groupActionBusy}
                  on:focus={() => dispatch('scheduleInviteSearch', { groupId: group.groupId, query: groupInviteInputs[group.groupId] || '' })}
                  on:input={(event) => {
                    const input = event.currentTarget as HTMLInputElement;
                    groupInviteInputs = { ...groupInviteInputs, [group.groupId]: input.value };
                    dispatch('scheduleInviteSearch', { groupId: group.groupId, query: input.value });
                  }}
                  on:keydown={(event) => {
                    if (event.key === 'Enter') {
                      event.preventDefault();
                      dispatch('inviteGroupMember', group);
                    }
                  }}
                />
                {#if activeGroupInviteId === group.groupId && groupInviteOptions.length > 0}
                  <div class="group-invite-suggestions">
                    {#each groupInviteOptions as profile}
                      <button type="button" on:click={() => dispatch('selectInviteOption', { group, profile })}>
                        {#if profile.avatarUrl}
                          <img src={profile.avatarUrl} alt="" />
                        {:else}
                          <span class="avatar-fallback">{getInitials(profile.displayName)}</span>
                        {/if}
                        <span>
                          <strong>{profile.displayName}</strong>
                        </span>
                      </button>
                    {/each}
                  </div>
                {/if}
                <button
                  class="mini-button"
                  type="button"
                  disabled={groupActionBusy || !(groupInviteInputs[group.groupId] || '').trim()}
                  on:click={() => dispatch('inviteGroupMember', group)}
                >
                  Invite
                </button>
              </div>

              <button
                class="mini-icon-button danger"
                type="button"
                aria-label={`Delete ${group.groupName}`}
                title="Delete group"
                disabled={groupActionBusy}
                on:click={() => dispatch('deleteGroup', group)}
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M9 3h6l1 2h4v2H4V5h4l1-2Zm1 7v8h2v-8h-2Zm4 0v8h2v-8h-2ZM7 8h10l-1 13H8L7 8Z" />
                </svg>
              </button>
            </div>
          {:else if group.role === 'invited'}
            <button
              class="mini-button"
              type="button"
              disabled={groupActionBusy}
              on:click={() => dispatch('acceptGroupInvite', group)}
            >
              Accept
            </button>
          {:else}
            <button
              class="mini-button subtle"
              type="button"
              disabled={groupActionBusy}
              on:click={() => dispatch('leaveGroup', group)}
            >
              Leave
            </button>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</article>

<style>
  .settings-panel {
    display: grid;
    align-content: start;
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

  .panel-title {
    min-width: 0;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
  }

  h3,
  p {
    margin: 0;
  }

  h3 {
    margin-bottom: 0.12rem;
    color: var(--md-sys-color-on-surface);
    font-size: 0.94rem;
    font-weight: 600;
  }

  .panel-title p,
  .column-empty {
    max-width: 60rem;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.76rem;
    line-height: 1.35;
  }

  .primary-button,
  .mini-button {
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
    padding: 0.42rem 0.58rem;
    border-radius: 8px;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    font-size: 0.76rem;
    font-weight: 600;
    transition: background 0.18s ease, color 0.18s ease, border-color 0.18s ease;
  }

  .primary-button:disabled,
  .mini-button:disabled {
    cursor: default;
    opacity: 0.6;
  }

  .group-create-control,
  .group-owner-controls,
  .group-rename-control,
  .group-invite-control {
    min-width: 0;
    display: flex;
    gap: 0.45rem;
    align-items: center;
  }

  .group-owner-controls {
    display: grid;
    grid-template-columns: auto minmax(8.5rem, 1fr) auto;
    flex-wrap: nowrap;
    justify-content: flex-end;
    width: min(100%, 22rem);
  }

  .group-rename-control.active {
    grid-column: 1 / -1;
    justify-content: flex-end;
  }

  .group-invite-control {
    position: relative;
  }

  .group-create-control input,
  .group-rename-control input,
  .group-invite-control input {
    min-width: 0;
    width: 11rem;
    padding: 0.42rem 0.55rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    font-size: 0.76rem;
  }

  .group-invite-control input {
    width: 9.5rem;
  }

  .group-rename-control input {
    width: 7.9rem;
  }

  .group-create-control .group-tag-input {
    width: 4.5rem;
    text-transform: uppercase;
  }

  .group-tag-chip {
    display: inline-flex;
    align-items: center;
    max-width: 4.7rem;
    margin-right: 0.35rem;
    padding: 0.08rem 0.26rem;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 32%, var(--md-sys-color-outline-variant));
    border-radius: 5px;
    background: color-mix(in srgb, var(--md-sys-color-primary) 10%, var(--md-sys-color-surface));
    color: var(--md-sys-color-primary);
    font-size: 0.58rem;
    font-weight: 800;
    line-height: 1.1;
    vertical-align: middle;
  }

  .group-invite-suggestions {
    position: absolute;
    top: calc(100% + 0.35rem);
    left: 0;
    z-index: 25;
    width: min(18rem, calc(100vw - 2rem));
    max-height: 13rem;
    overflow-y: auto;
    display: grid;
    gap: 0.18rem;
    padding: 0.35rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 8px;
    background: var(--md-sys-color-surface-container-highest);
    box-shadow: 0 8px 18px color-mix(in srgb, black 24%, transparent);
  }

  .group-invite-suggestions button {
    min-width: 0;
    display: grid;
    grid-template-columns: 1.55rem minmax(0, 1fr);
    gap: 0.45rem;
    align-items: center;
    padding: 0.36rem 0.42rem;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--md-sys-color-on-surface);
    text-align: left;
    cursor: pointer;
  }

  .group-invite-suggestions button:hover {
    background: color-mix(in srgb, var(--md-sys-color-primary) 10%, transparent);
  }

  .group-invite-suggestions img,
  .group-invite-suggestions .avatar-fallback {
    width: 1.55rem;
    height: 1.55rem;
  }

  .group-invite-suggestions span {
    min-width: 0;
  }

  .group-invite-suggestions strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.76rem;
  }

  .group-list {
    display: grid;
    gap: 0.32rem;
  }

  .group-row {
    min-width: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.5rem;
    align-items: start;
    padding: 0.38rem 0.5rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
  }

  .group-row.invited {
    border-color: color-mix(in srgb, var(--md-sys-color-primary) 52%, var(--md-sys-color-outline-variant));
    background: color-mix(in srgb, var(--md-sys-color-primary) 8%, var(--md-sys-color-surface));
  }

  .group-row-main {
    min-width: 0;
    display: grid;
    gap: 0.24rem;
    align-items: start;
  }

  .group-title-line {
    min-width: 0;
    display: flex;
    gap: 0.45rem;
    align-items: baseline;
  }

  .group-title-line strong,
  .group-role-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .group-title-line strong {
    color: var(--md-sys-color-on-surface);
    font-size: 0.8rem;
  }

  .group-role-label {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
  }

  .group-member-stack {
    min-width: 0;
    display: flex;
    align-items: center;
    width: fit-content;
    max-width: 100%;
    height: 2rem;
    padding-left: 0.02rem;
  }

  .group-member-stack.compact {
    height: 1.7rem;
  }

  .group-member-stack.expanded {
    flex-wrap: wrap;
    gap: 0.18rem;
    width: 100%;
    height: auto;
    padding-top: 0.05rem;
  }

  .group-member-stack img,
  .group-member-stack .avatar-fallback {
    flex: 0 0 auto;
    width: 2rem;
    height: 2rem;
    margin-left: -0.42rem;
    border: 2px solid var(--md-sys-color-surface);
    border-radius: 50%;
    box-sizing: border-box;
  }

  .group-member-stack img:first-child,
  .group-member-stack .avatar-fallback:first-child {
    margin-left: 0;
  }

  .group-member-stack.compact img,
  .group-member-stack.compact .avatar-fallback {
    width: 1.58rem;
    height: 1.58rem;
    margin-left: -0.34rem;
  }

  .group-member-stack.expanded img,
  .group-member-stack.expanded .avatar-fallback {
    margin-left: 0;
  }

  .group-avatar-overflow {
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    width: 1.58rem;
    height: 1.58rem;
    margin-left: -0.34rem;
    border: 2px solid var(--md-sys-color-surface);
    border-radius: 50%;
    background: var(--md-sys-color-surface-container-highest);
    color: var(--md-sys-color-on-surface);
    font-size: 0.58rem;
    font-weight: 800;
    cursor: pointer;
  }

  .group-member-collapse {
    align-self: center;
    padding: 0.14rem 0.38rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 999px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.62rem;
    font-weight: 700;
    cursor: pointer;
  }

  .avatar-fallback {
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
    font-weight: 900;
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

  .mini-icon-button {
    display: grid;
    place-items: center;
    width: 2rem;
    height: 2rem;
    padding: 0;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 7px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
    transition: border-color 0.18s ease, background 0.18s ease, color 0.18s ease;
  }

  .mini-icon-button svg {
    width: 1rem;
    height: 1rem;
    fill: currentColor;
  }

  .mini-icon-button:hover:not(:disabled) {
    border-color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 8%, var(--md-sys-color-surface));
    color: var(--md-sys-color-primary);
  }

  .mini-icon-button.danger {
    border-color: color-mix(in srgb, var(--md-sys-color-error) 38%, var(--md-sys-color-outline-variant));
    color: var(--md-sys-color-error);
  }

  .mini-icon-button.danger:hover:not(:disabled) {
    border-color: var(--md-sys-color-error);
    background: color-mix(in srgb, var(--md-sys-color-error) 8%, var(--md-sys-color-surface));
  }

  .mini-icon-button:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  @media (max-width: 760px) {
    .panel-title,
    .group-create-control,
    .group-owner-controls,
    .group-rename-control,
    .group-invite-control,
    .group-row {
      display: grid;
      grid-template-columns: 1fr;
      justify-items: stretch;
    }

    .group-create-control input,
    .group-rename-control input,
    .group-invite-control input {
      width: 100%;
    }
  }
</style>
