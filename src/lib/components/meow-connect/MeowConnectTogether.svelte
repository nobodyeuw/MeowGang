<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { appAsset } from '$lib/assets';
  import type {
    MeowConnectFriendConnection,
    MeowConnectLocalSnapshot
  } from '$lib/services/meow-connect';
  import type { ProfileRaidGroup, RaidDifficultyFilterItem, RaidTogetherRow } from './types';
  import type { Raid } from '$lib/data/raids';

  export let acceptedFriendConnections: MeowConnectFriendConnection[] = [];
  export let connectedFriends = 0;
  export let getFriendConnectionKey: (connection: MeowConnectFriendConnection) => string;
  export let getInitials: (name: string) => string = () => '?';
  export let localSnapshot: MeowConnectLocalSnapshot | null = null;
  export let raidDifficultyFilterItems: RaidDifficultyFilterItem[] = [];
  export let raidTogetherRows: RaidTogetherRow[] = [];
  export let selectedTogetherConnections: MeowConnectFriendConnection[] = [];
  export let selectedTogetherFriendIds = new Set<string>();
  export let visibleRaids: Raid[] = [];

  const dispatch = createEventDispatcher<{
    openProfileGroup: ProfileRaidGroup;
    setRaidDifficultyFilter: { raidId: string; difficulty: string };
    toggleTogetherFriend: MeowConnectFriendConnection;
  }>();
  const meowConnectIcon = appAsset('meowconnect_tab.png');
</script>

<section class="together-panel">
  <div class="panel-title together-panel-title">
    <div class="mc-title">
      <img src={meowConnectIcon} alt="" />
      <h2>MeowConnect</h2>
    </div>

    <div class="overview-toolbar raid-filter-center">
      <div class="raid-filter-toggle">
        {#each raidDifficultyFilterItems as filter (filter.raid.id)}
          <div class="raid-difficulty-filter">
            <span>{filter.raid.name}</span>
            <div class="mode-toggle">
              {#each filter.difficulties as difficulty (`${filter.raid.id}:${difficulty}`)}
                <button
                  type="button"
                  class:active={filter.selectedDifficulty === difficulty}
                  on:click|stopPropagation={() => dispatch('setRaidDifficultyFilter', { raidId: filter.raid.id, difficulty })}
                  title={filter.selectedDifficulty === difficulty ? `Clear ${filter.raid.name} filter` : `${filter.raid.name} ${difficulty}`}
                >
                  {difficulty}
                </button>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    </div>

    <div class="summary-pill">
      <strong>{connectedFriends}</strong>
      <span>friends</span>
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
          on:click={() => dispatch('toggleTogetherFriend', connection)}
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

  <div class="together-grid" data-guide="meow-connect-profile-details">
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
            <div>
              <strong>{row.raidName}</strong>
              <span>{row.minIlvl}+</span>
            </div>
          </div>

          <div class="together-count">
            <strong>{row.togetherCount}</strong>
            <span>together</span>
          </div>

          <div class="profile-group-grid together-profile-grid">
            {#each row.groups as group}
              <article class:empty={group.openCount === 0} class="profile-group-card">
                <button
                  class="profile-group-summary"
                  type="button"
                  on:click={() => dispatch('openProfileGroup', group)}
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
                      {group.openCount} available
                      {#if group.clearedCount > 0}
                        | {group.clearedCount} cleared
                      {/if}
                    </small>
                  </span>
                </button>
              </article>
            {/each}
          </div>
        </article>
      {/each}
    {/if}
  </div>
</section>

<style>
  .together-panel {
    display: grid;
    gap: 0.6rem;
    padding: 0.65rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
  }

  .panel-title,
  .overview-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
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
  p {
    margin: 0;
  }

  h2 {
    color: var(--md-sys-color-on-surface);
    font-size: 1.18rem;
    line-height: 1.1;
  }

  .mode-toggle {
    display: inline-flex;
    padding: 0.14rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 7px;
    background: var(--md-sys-color-surface);
  }

  .mode-toggle button {
    padding: 0.34rem 0.52rem;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font: inherit;
    font-size: 0.74rem;
    font-weight: 600;
    white-space: nowrap;
  }

  .mode-toggle button.active {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
  }

  .together-panel-title {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.45rem 0.55rem;
  }

  .together-panel-title .overview-toolbar {
    justify-content: center;
    min-width: 0;
  }

  .raid-filter-center {
    justify-self: center;
    width: min(100%, 54rem);
  }

  .raid-filter-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: 1 1 auto;
    flex-wrap: wrap;
    gap: 0.35rem;
    min-width: 0;
    overflow: visible;
  }

  .raid-difficulty-filter {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.18rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
    white-space: nowrap;
  }

  .raid-difficulty-filter > span {
    padding: 0 0.32rem;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
    font-weight: 800;
  }

  .raid-difficulty-filter .mode-toggle {
    border: 0;
    padding: 0;
    background: transparent;
  }

  .raid-difficulty-filter .mode-toggle button {
    padding: 0.28rem 0.42rem;
    font-size: 0.68rem;
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

  .together-friend-picker span,
  .profile-group-summary strong,
  .profile-group-summary small,
  .together-main strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .together-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
    gap: 0.55rem;
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

  .together-card.empty,
  .profile-group-card.empty {
    opacity: 0.68;
  }

  .together-main {
    min-width: 0;
    display: block;
  }

  .together-main div {
    min-width: 0;
    display: grid;
    gap: 0.05rem;
  }

  .together-main strong {
    color: var(--md-sys-color-on-surface);
    font-size: 0.84rem;
  }

  .together-main span,
  .together-count span,
  .profile-group-summary small {
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

  .profile-group-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 0.45rem;
    padding: 0.45rem;
    align-items: start;
  }

  .together-profile-grid {
    grid-column: 1 / -1;
    padding: 0;
    grid-template-columns: repeat(auto-fit, minmax(165px, 1fr));
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

  .profile-group-summary > span {
    min-width: 0;
    display: grid;
    gap: 0.05rem;
  }

  .profile-group-summary strong {
    font-size: 0.8rem;
  }

  .profile-group-summary img,
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

  .column-empty {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.82rem;
    line-height: 1.45;
  }

  @media (max-width: 760px) {
    .together-panel-title,
    .overview-toolbar {
      display: grid;
      grid-template-columns: 1fr;
    }
  }
</style>
