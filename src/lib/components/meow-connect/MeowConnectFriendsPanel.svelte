<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { MeowConnectFriendConnection } from '$lib/services/meow-connect';
  import type { FriendOption } from './types';

  export let consentAccepted = false;
  export let connectedFriends = 0;
  export let pendingIncoming: MeowConnectFriendConnection[] = [];
  export let friendConnections: MeowConnectFriendConnection[] = [];
  export let sortedFriendConnections: MeowConnectFriendConnection[] = [];
  export let showFriendPopover = false;
  export let friendSearch = '';
  export let friendDiscordId = '';
  export let friendActionBusy = false;
  export let filteredFriendOptions: FriendOption[] = [];
  export let popoverElement: HTMLElement | null = null;
  export let getProfileAvatar: (discordId: string) => string | undefined = () => undefined;
  export let getInitials: (name: string) => string = () => '?';

  const dispatch = createEventDispatcher<{
    openPopover: void;
    sendFriendRequest: void;
    selectFriendOption: FriendOption;
    acceptFriendRequest: MeowConnectFriendConnection;
    removeFriend: MeowConnectFriendConnection;
  }>();
</script>

<article class="settings-panel" data-guide="meow-connect-friends">
  <div class="panel-title">
    <div>
      <h3>Friends</h3>
      <p>{connectedFriends} connected | {pendingIncoming.length} incoming request{pendingIncoming.length === 1 ? '' : 's'}</p>
    </div>
    <div class="friend-add-control">
      {#if consentAccepted && showFriendPopover}
        <div class="friend-popover" bind:this={popoverElement}>
          <button
            class="popover-close"
            type="button"
            aria-label="Close add friends"
            on:click={() => {
              showFriendPopover = false;
            }}
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
                  dispatch('sendFriendRequest');
                }
                if (event.key === 'Escape') {
                  showFriendPopover = false;
                }
              }}
            />
            <button
              type="button"
              disabled={friendActionBusy || !(friendDiscordId || friendSearch).trim()}
              on:click={() => dispatch('sendFriendRequest')}
            >
              Add
            </button>
          </div>

          <div class="friend-suggestion-list">
            {#if filteredFriendOptions.length === 0}
              <p>No whitelist name matches.</p>
            {:else}
              {#each filteredFriendOptions as friend}
                <button type="button" on:click={() => dispatch('selectFriendOption', friend)}>
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

      <button class="primary-button" type="button" on:click={() => dispatch('openPopover')}>
        Add friend
      </button>
    </div>
  </div>

  {#if pendingIncoming.length > 0}
    <div class="friend-request-notice">
      <strong>{pendingIncoming.length}</strong>
      <span>incoming friend request{pendingIncoming.length === 1 ? '' : 's'} waiting</span>
    </div>
  {/if}

  <div class="friend-list">
    {#if friendConnections.length === 0}
      <p class="column-empty">No MeowConnect friends yet.</p>
    {:else}
      {#each sortedFriendConnections as connection}
        <div class:incoming={connection.status === 'pending' && connection.direction === 'incoming'} class="friend-row">
          {#if connection.profile.avatarUrl}
            <img src={connection.profile.avatarUrl} alt="" />
          {:else}
            <span class="avatar-fallback">{getInitials(connection.profile.displayName)}</span>
          {/if}
          <div>
            <strong>{connection.profile.displayName}</strong>
            <span>{connection.status}{connection.status === 'pending' && connection.direction === 'incoming' ? ' incoming' : ''}</span>
          </div>
          <div class="friend-actions">
            {#if connection.status === 'pending' && connection.direction === 'incoming'}
              <button
                class="mini-button"
                type="button"
                disabled={friendActionBusy}
                on:click={() => dispatch('acceptFriendRequest', connection)}
              >
                Accept
              </button>
            {/if}
            <button
              class="mini-button subtle"
              type="button"
              disabled={friendActionBusy}
              on:click={() => dispatch('removeFriend', connection)}
            >
              Remove
            </button>
          </div>
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

  .panel-title,
  .friend-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    min-width: 0;
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

  .panel-title {
    align-items: flex-start;
  }

  .panel-title p,
  .column-empty,
  .friend-suggestion-list p {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.76rem;
    line-height: 1.35;
  }

  .panel-title p {
    max-width: 60rem;
  }

  .friend-add-control {
    position: relative;
    display: flex;
    justify-content: flex-end;
    align-items: center;
  }

  .primary-button,
  .mini-button,
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

  .primary-button {
    padding: 0.42rem 0.58rem;
    border-radius: 8px;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    font-size: 0.76rem;
    font-weight: 600;
    transition: background 0.18s ease, color 0.18s ease, border-color 0.18s ease;
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

  .friend-suggestion-list strong,
  .friend-row strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--md-sys-color-on-surface);
    font-size: 0.8rem;
  }

  .friend-suggestion-list strong {
    font-size: 0.78rem;
    font-weight: 600;
  }

  .friend-suggestion-list img,
  .friend-suggestion-list .avatar-fallback {
    width: 1.7rem;
    height: 1.7rem;
  }

  .friend-row img,
  .avatar-fallback {
    width: 2rem;
    height: 2rem;
  }

  .friend-suggestion-list img,
  .friend-row img,
  .avatar-fallback {
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

  .friend-list {
    display: grid;
    gap: 0.5rem;
  }

  .friend-request-notice {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    width: fit-content;
    padding: 0.38rem 0.55rem;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 52%, var(--md-sys-color-outline-variant));
    border-radius: 8px;
    background: color-mix(in srgb, var(--md-sys-color-primary) 10%, var(--md-sys-color-surface));
    color: var(--md-sys-color-on-surface);
    font-size: 0.75rem;
  }

  .friend-request-notice strong {
    display: grid;
    place-items: center;
    min-width: 1.25rem;
    height: 1.25rem;
    border-radius: 999px;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    font-size: 0.7rem;
    line-height: 1;
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

  .friend-row.incoming {
    border-color: color-mix(in srgb, var(--md-sys-color-primary) 52%, var(--md-sys-color-outline-variant));
    background: color-mix(in srgb, var(--md-sys-color-primary) 8%, var(--md-sys-color-surface));
  }

  .friend-row > div:first-of-type {
    min-width: 0;
    flex: 1;
  }

  .friend-row span {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.74rem;
  }

  .friend-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
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

  @media (max-width: 760px) {
    .panel-title {
      display: grid;
      grid-template-columns: 1fr;
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
