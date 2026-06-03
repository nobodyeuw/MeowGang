<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { classAsset, iconAsset } from '$lib/assets';
  import { getGameClassDisplayName, getGameClassIconId } from '$lib/data/classes';
  import { RAIDS } from '$lib/data/raids';
  import type {
    MeowConnectAvailabilityRow,
    MeowConnectGroup,
    MeowConnectGroupRaidAssignment,
    MeowConnectProfile
  } from '$lib/services/meow-connect';
  import {
    getAvailabilityDifficultyLabel,
    getProfileGroupIlvlLabel
  } from './profile-helpers';
  import type { ProfileRaidGroup } from './types';

  export let profileGroup: ProfileRaidGroup;
  export let assignableGroups: MeowConnectGroup[] = [];
  export let currentProfile: MeowConnectProfile | null = null;
  export let groupAssignmentBusyKey = '';
  export let raidDifficultyFilters: Record<string, string> = {};
  export let getInitials: (name: string) => string = () => '?';

  const dispatch = createEventDispatcher<{
    close: void;
    changeGroupAssignment: { row: MeowConnectAvailabilityRow; groupId: string };
  }>();
  const goldIcon = iconAsset('gold.png');

  function formatCharacterItemLevel(value: number): string {
    return String(Math.round(value || 0));
  }

  function formatCharacterPower(value: number): string {
    if (!value || value <= 0) return 'CP ?';
    return `CP ${Math.round(value)}`;
  }

  function getClassName(classId: string): string {
    return getGameClassDisplayName(classId);
  }

  function getClassIcon(classId: string): string {
    return getGameClassIconId(classId);
  }

  function getRowOwnerUserId(row: MeowConnectAvailabilityRow): string {
    if (row.ownerId === 'local') return currentProfile?.userId || row.ownerUserId || '';
    return row.ownerUserId || '';
  }

  function getGroupAssignmentKey(row: MeowConnectAvailabilityRow): string {
    return `${row.character.charId}:${row.raid.id}`;
  }

  function getRowGroupAssignments(row: MeowConnectAvailabilityRow): Array<{ group: MeowConnectGroup; assignment: MeowConnectGroupRaidAssignment }> {
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

  function getAssignedGroupId(row: MeowConnectAvailabilityRow): string {
    return getRowGroupAssignments(row)[0]?.group.groupId || '';
  }

  function getAssignedGroupNames(row: MeowConnectAvailabilityRow): string[] {
    return getRowGroupAssignments(row).map(({ group }) => group.groupName);
  }

  function getAssignedGroupTags(row: MeowConnectAvailabilityRow): string[] {
    return getRowGroupAssignments(row).map(({ group }) => group.groupTag).filter(Boolean);
  }

  function getGroupAssignmentLabel(row: MeowConnectAvailabilityRow): string {
    const groupNames = getAssignedGroupNames(row);
    if (groupNames.length > 0) return groupNames.join(', ');
    if (row.ownerId === 'local') return 'No group assigned';
    return 'Reserved';
  }

  function getGroupStateText(row: MeowConnectAvailabilityRow): string {
    return getAssignedGroupTags(row)[0] || getAssignedGroupNames(row)[0] || (row.ownerId === 'local' ? 'No group' : 'Reserved');
  }

  function canAssignRowToGroup(row: MeowConnectAvailabilityRow): boolean {
    return row.ownerId === 'local' && row.reservedForStatic && assignableGroups.length > 0;
  }

  function hasGroupState(row: MeowConnectAvailabilityRow): boolean {
    return row.reservedForStatic || getAssignedGroupNames(row).length > 0;
  }

  function hasSharedGroupAssignment(row: MeowConnectAvailabilityRow): boolean {
    return getAssignedGroupNames(row).length > 0;
  }

  function isAvailableRow(row: MeowConnectAvailabilityRow): boolean {
    return row.status === 'open' && (!row.reservedForStatic || row.ownerId === 'local' || hasSharedGroupAssignment(row));
  }

  function getGroupIlvlLabel(group: ProfileRaidGroup): string {
    return getProfileGroupIlvlLabel(group, RAIDS, raidDifficultyFilters);
  }

  function dispatchGroupChange(row: MeowConnectAvailabilityRow, event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    dispatch('changeGroupAssignment', { row, groupId: select.value });
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  class="profile-detail-backdrop"
  role="presentation"
  on:click={() => dispatch('close')}
>
  <div
    class="profile-detail-popover"
    role="dialog"
    aria-modal="true"
    aria-label={`${profileGroup.ownerName} raid availability`}
    tabindex="-1"
    on:click|stopPropagation
  >
    <header>
      <div class="profile-detail-title">
        {#if profileGroup.ownerAvatarUrl}
          <img src={profileGroup.ownerAvatarUrl} alt="" />
        {:else}
          <span class="avatar-fallback">{getInitials(profileGroup.ownerName)}</span>
        {/if}
        <div>
          <h3>{profileGroup.raidName}</h3>
          <strong>{profileGroup.ownerName}</strong>
          <span>{profileGroup.openCount} available | {profileGroup.clearedCount} cleared | {getGroupIlvlLabel(profileGroup)}</span>
        </div>
      </div>
      <button type="button" class="icon-button" aria-label="Close availability details" on:click={() => dispatch('close')}>X</button>
    </header>

    <div class="availability-stack">
      {#each profileGroup.rows.filter(isAvailableRow) as row}
        <article class:shared-static={hasSharedGroupAssignment(row)} class="availability-card">
          <img src={classAsset(getClassIcon(row.character.classId))} alt="" class="class-icon" />

          <div class="character-copy">
            <div class="character-title-line">
              <strong>{row.character.charName}</strong>
              <span class="class-name">{getClassName(row.character.classId)}</span>
              <img
                src={goldIcon}
                alt={row.character.earnsGold ? 'Gold earner' : 'Non-gold earner'}
                class:inactive={!row.character.earnsGold}
                class="gold-earner-icon"
                title={row.character.earnsGold ? 'Gold earner' : 'Non-gold earner'}
              />
            </div>
            <span class="character-stats-line">
              <span class="stat-field">iLvl {formatCharacterItemLevel(row.character.itemLevel)}</span>
              <span class="stat-field combat-power">{formatCharacterPower(row.character.combatPower)}</span>
            </span>
            <small class="availability-meta">
              <span>{row.openGates}/{row.totalGates} gates open | {getAvailabilityDifficultyLabel(row)}</span>
            </small>
          </div>

          {#if canAssignRowToGroup(row)}
            <label class="group-assignment-control" title={getGroupAssignmentLabel(row)}>
              <span>Group</span>
              <select
                value={getAssignedGroupId(row)}
                disabled={groupAssignmentBusyKey === getGroupAssignmentKey(row)}
                on:change={(event) => dispatchGroupChange(row, event)}
              >
                <option value="">None</option>
                {#each assignableGroups as group}
                  <option value={group.groupId}>{group.groupName}</option>
                {/each}
              </select>
            </label>
          {:else if hasGroupState(row)}
            <span class:reserved={getAssignedGroupNames(row).length === 0 && row.ownerId !== 'local'} class="group-assignment-badge" title={getGroupAssignmentLabel(row)}>
              {getGroupStateText(row)}
            </span>
          {/if}
        </article>
      {/each}

      {#each profileGroup.rows.filter((row) => row.status === 'open' && row.reservedForStatic && row.ownerId !== 'local' && !hasSharedGroupAssignment(row)) as row}
        <article class="availability-card reserved">
          <img src={classAsset(getClassIcon(row.character.classId))} alt="" class="class-icon" />

          <div class="character-copy">
            <div class="character-title-line">
              <strong>{row.character.charName}</strong>
              <span class="class-name">{getClassName(row.character.classId)}</span>
              <img
                src={goldIcon}
                alt={row.character.earnsGold ? 'Gold earner' : 'Non-gold earner'}
                class:inactive={!row.character.earnsGold}
                class="gold-earner-icon"
                title={row.character.earnsGold ? 'Gold earner' : 'Non-gold earner'}
              />
            </div>
            <span class="character-stats-line">
              <span class="stat-field">iLvl {formatCharacterItemLevel(row.character.itemLevel)}</span>
              <span class="stat-field combat-power">{formatCharacterPower(row.character.combatPower)}</span>
            </span>
            <small class="availability-meta">
              <span>{getAvailabilityDifficultyLabel(row)}</span>
            </small>
          </div>

          {#if canAssignRowToGroup(row)}
            <label class="group-assignment-control" title={getGroupAssignmentLabel(row)}>
              <span>Group</span>
              <select
                value={getAssignedGroupId(row)}
                disabled={groupAssignmentBusyKey === getGroupAssignmentKey(row)}
                on:change={(event) => dispatchGroupChange(row, event)}
              >
                <option value="">None</option>
                {#each assignableGroups as group}
                  <option value={group.groupId}>{group.groupName}</option>
                {/each}
              </select>
            </label>
          {:else if hasGroupState(row)}
            <span class:reserved={getAssignedGroupNames(row).length === 0 && row.ownerId !== 'local'} class="group-assignment-badge" title={getGroupAssignmentLabel(row)}>
              {getGroupStateText(row)}
            </span>
          {/if}
        </article>
      {/each}

      {#each profileGroup.rows.filter((row) => row.status === 'cleared') as row}
        <article class="availability-card cleared">
          <img src={classAsset(getClassIcon(row.character.classId))} alt="" class="class-icon" />

          <div class="character-copy">
            <div class="character-title-line">
              <strong>{row.character.charName}</strong>
              <span class="class-name">{getClassName(row.character.classId)}</span>
              <img
                src={goldIcon}
                alt={row.character.earnsGold ? 'Gold earner' : 'Non-gold earner'}
                class:inactive={!row.character.earnsGold}
                class="gold-earner-icon"
                title={row.character.earnsGold ? 'Gold earner' : 'Non-gold earner'}
              />
            </div>
            <span class="character-stats-line">
              <span class="stat-field">iLvl {formatCharacterItemLevel(row.character.itemLevel)}</span>
              <span class="stat-field combat-power">{formatCharacterPower(row.character.combatPower)}</span>
            </span>
            <small class="availability-meta">
              <span>cleared | {getAvailabilityDifficultyLabel(row)}</span>
            </small>
          </div>

          {#if canAssignRowToGroup(row)}
            <label class="group-assignment-control" title={getGroupAssignmentLabel(row)}>
              <span>Group</span>
              <select
                value={getAssignedGroupId(row)}
                disabled={groupAssignmentBusyKey === getGroupAssignmentKey(row)}
                on:change={(event) => dispatchGroupChange(row, event)}
              >
                <option value="">None</option>
                {#each assignableGroups as group}
                  <option value={group.groupId}>{group.groupName}</option>
                {/each}
              </select>
            </label>
          {:else if hasGroupState(row)}
            <span class:reserved={getAssignedGroupNames(row).length === 0 && row.ownerId !== 'local'} class="group-assignment-badge" title={getGroupAssignmentLabel(row)}>
              {getGroupStateText(row)}
            </span>
          {/if}
        </article>
      {/each}

      {#if profileGroup.rows.length === 0}
        <p class="column-empty">No matching characters.</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .profile-detail-backdrop {
    position: fixed;
    inset: 5rem 0 0;
    z-index: 90;
    display: grid;
    place-items: start center;
    padding: 1rem;
    background: color-mix(in srgb, black 18%, transparent);
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
    box-shadow: var(--app-shadow-md);
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

  .profile-detail-title div,
  .character-copy {
    min-width: 0;
    display: grid;
    gap: 0.1rem;
  }

  .profile-detail-title h3 {
    min-width: 0;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.92rem;
  }

  .profile-detail-title strong,
  .character-copy strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--md-sys-color-on-surface);
    font-size: 0.76rem;
    font-weight: 650;
  }

  .profile-detail-title span,
  .column-empty {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.74rem;
  }

  .availability-stack {
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
    font-size: 1rem;
    line-height: 1;
  }

  .icon-button:hover {
    border-color: var(--md-sys-color-primary);
    color: var(--md-sys-color-primary);
  }

  .availability-card {
    display: grid;
    grid-template-columns: 1.65rem minmax(0, 1fr) auto;
    gap: 0.45rem;
    align-items: center;
    min-height: 3.35rem;
    padding: 0.42rem 0.48rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
  }

  .availability-card.reserved {
    opacity: 0.78;
    border-color: color-mix(in srgb, var(--md-sys-color-error) 30%, var(--md-sys-color-outline));
    background:
      linear-gradient(90deg, color-mix(in srgb, var(--md-sys-color-error) 11%, transparent), transparent 58%),
      color-mix(in srgb, var(--md-sys-color-surface-variant) 62%, var(--md-sys-color-surface));
  }

  .availability-card.shared-static {
    opacity: 1;
    border-color: color-mix(in srgb, var(--md-sys-color-success) 30%, var(--md-sys-color-outline));
    background:
      linear-gradient(90deg, color-mix(in srgb, var(--md-sys-color-success) 10%, transparent), transparent 58%),
      color-mix(in srgb, var(--md-sys-color-surface-variant) 62%, var(--md-sys-color-surface));
  }

  .availability-card.cleared {
    opacity: 0.58;
  }

  .availability-card.cleared .character-title-line,
  .availability-card.cleared .character-stats-line,
  .availability-card.cleared .availability-meta > span:first-child {
    text-decoration: line-through;
    text-decoration-thickness: 1px;
    text-decoration-color: var(--md-sys-color-on-surface-variant);
  }

  .class-icon {
    width: 1.65rem;
    height: 1.65rem;
    object-fit: contain;
  }

  .character-title-line {
    min-width: 0;
    display: inline-flex;
    align-items: baseline;
    gap: 0.35rem;
  }

  .class-name {
    min-width: 0;
    overflow: hidden;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.68rem;
    font-style: italic;
    font-weight: 650;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .character-stats-line,
  .availability-meta {
    min-width: 0;
    display: inline-flex;
    align-items: center;
    gap: 0.18rem;
    font-size: 0.68rem;
  }

  .stat-field {
    width: 5.05rem;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stat-field:first-child {
    width: 4rem;
  }

  .combat-power {
    color: var(--app-color-accent-muted);
    font-weight: 500;
  }

  .gold-earner-icon {
    width: 0.9rem;
    height: 0.9rem;
    flex: 0 0 0.9rem;
    object-fit: contain;
  }

  .gold-earner-icon.inactive {
    filter: grayscale(1);
    opacity: 0.38;
  }

  .availability-meta > span:first-child {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .group-assignment-control {
    min-width: 8.4rem;
    display: grid;
    gap: 0.12rem;
    justify-self: end;
  }

  .group-assignment-control span {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.58rem;
    font-weight: 700;
    letter-spacing: 0;
    line-height: 1;
    text-transform: uppercase;
  }

  .group-assignment-control select {
    width: 8.4rem;
    min-width: 0;
    padding: 0.28rem 0.42rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 7px;
    background: color-mix(in srgb, var(--md-sys-color-surface) 88%, transparent);
    color: var(--md-sys-color-on-surface);
    font-size: 0.68rem;
    font-weight: 600;
  }

  .group-assignment-control select:disabled {
    opacity: 0.58;
    cursor: wait;
  }

  .group-assignment-badge {
    justify-self: end;
    max-width: 8.4rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0.28rem 0.48rem;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-success) 30%, var(--md-sys-color-outline-variant));
    border-radius: 7px;
    background: color-mix(in srgb, var(--md-sys-color-success) 10%, var(--md-sys-color-surface));
    color: var(--md-sys-color-on-surface);
    font-size: 0.68rem;
    font-weight: 700;
  }

  .group-assignment-badge.reserved {
    border-color: color-mix(in srgb, var(--md-sys-color-error) 28%, var(--md-sys-color-outline-variant));
    background: color-mix(in srgb, var(--md-sys-color-error) 8%, var(--md-sys-color-surface));
    color: var(--md-sys-color-on-surface-variant);
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

  .column-empty {
    line-height: 1.45;
  }

  @media (max-width: 760px) {
    .profile-detail-backdrop {
      place-items: end center;
      padding: 0.75rem;
    }

    .profile-detail-popover {
      max-height: min(82vh, 42rem);
    }

    .availability-card {
      grid-template-columns: 1.65rem minmax(0, 1fr);
    }

    .group-assignment-control {
      grid-column: 2;
      width: 100%;
      justify-self: stretch;
    }

    .group-assignment-control select {
      width: 100%;
    }

    .group-assignment-badge {
      grid-column: 2;
      justify-self: stretch;
      max-width: none;
    }
  }
</style>
