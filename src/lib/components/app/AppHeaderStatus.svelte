<script lang="ts">
  import type { MeowConnectFriendConnection } from '$lib/services/meow-connect';
  import type { MeowConnectHeaderState } from '$lib/types/app-shell';

  export let activeTab = 'dashboard';
  export let meowConnectFeatureEnabled = true;
  export let showHeaderCountdown = true;
  export let resetCountdown = '';
  export let meowConnectHeaderState: MeowConnectHeaderState = 'inactive';
  export let meowConnectHeaderMessage = '';
  export let meowConnectHeaderLabel = 'Inactive';
  export let pendingMeowConnectRequests = 0;
  export let pendingMeowConnectFriendRequests: MeowConnectFriendConnection[] = [];
  export let showSetupGuideButton = true;
  export let switchTab: (tab: string) => void;
  export let openMeowConnectRequests: () => void;
  export let startSetupGuide: () => void;

  function getInitials(name: string): string {
    const parts = name.trim().split(/\s+/).filter(Boolean);
    return (parts[0]?.[0] || '?').toUpperCase() + (parts[1]?.[0] || '').toUpperCase();
  }
</script>

<div class="title-row">
  <button
    type="button"
    class="app-title-button"
    on:click={() => activeTab !== 'dashboard' && switchTab('dashboard')}
    aria-label="Go to dashboard"
  >
    <img src="/images/LOAtracker_header.png" alt="LOA Tracker" class="app-title-logo" />
  </button>

  {#if showHeaderCountdown && resetCountdown}
    <div class="reset-countdown">{resetCountdown}</div>
  {/if}

  {#if meowConnectFeatureEnabled}
    <div
      class="meowconnect-header-status"
      class:active={meowConnectHeaderState === 'active'}
      class:connecting={meowConnectHeaderState === 'connecting' || meowConnectHeaderState === 'sleeping'}
      class:inactive={meowConnectHeaderState === 'inactive'}
      class:offline={meowConnectHeaderState === 'offline' || meowConnectHeaderState === 'login_required'}
      title={meowConnectHeaderMessage}
    >
      <img src="/images/meowconnect_tab.png" alt="" />
      <span>{meowConnectHeaderLabel}</span>
    </div>
  {/if}

  {#if pendingMeowConnectRequests > 0}
    <button
      type="button"
      class="meowconnect-request-alert"
      title={`${pendingMeowConnectRequests} incoming MeowConnect request${pendingMeowConnectRequests === 1 ? '' : 's'}`}
      on:click={openMeowConnectRequests}
    >
      <span class="request-avatar-stack">
        {#if pendingMeowConnectFriendRequests.length > 0}
          {#each pendingMeowConnectFriendRequests.slice(0, 3) as request, requestIndex}
            {#if request.profile.avatarUrl}
              <img
                src={request.profile.avatarUrl}
                alt=""
                title={request.profile.displayName}
                style={`--request-avatar-index: ${requestIndex}`}
              />
            {:else}
              <span
                class="request-avatar-fallback"
                title={request.profile.displayName}
                style={`--request-avatar-index: ${requestIndex}`}
              >
                {getInitials(request.profile.displayName)}
              </span>
            {/if}
          {/each}
        {:else}
          <img src="/images/meowconnect_tab.png" alt="" style="--request-avatar-index: 0" />
        {/if}
        {#if pendingMeowConnectRequests > 3}
          <span class="request-avatar-overflow">+{pendingMeowConnectRequests - 3}</span>
        {/if}
      </span>
      <span>{pendingMeowConnectRequests} request{pendingMeowConnectRequests === 1 ? '' : 's'}</span>
    </button>
  {/if}

  {#if showSetupGuideButton}
    <button class="setup-guide-button" type="button" on:click={startSetupGuide}>Set-Up Guide</button>
  {/if}
</div>

<style>
  .title-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 0.12rem;
  }

  .app-title-button {
    border: none;
    background: transparent;
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font: inherit;
    font-size: 1.5rem;
    font-weight: 600;
    margin: 0;
    padding: 0;
  }

  .app-title-button:hover {
    color: var(--md-sys-color-primary);
  }

  .app-title-logo {
    width: 178px;
    height: 38px;
    display: block;
    object-fit: contain;
    object-position: center;
  }

  .reset-countdown {
    font-size: 0.75rem;
    color: var(--md-sys-color-primary);
    font-weight: 600;
    margin: 0;
    letter-spacing: 0.3px;
    text-transform: uppercase;
  }

  .meowconnect-header-status {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    margin-left: 0.65rem;
    font-size: 0.75rem;
    font-weight: 700;
    line-height: 1;
    letter-spacing: 0.3px;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .meowconnect-header-status img {
    width: 20px;
    height: 20px;
    object-fit: contain;
    display: block;
  }

  .meowconnect-header-status.active {
    color: var(--md-sys-color-primary);
  }

  .meowconnect-header-status.connecting {
    color: color-mix(in srgb, var(--md-sys-color-primary) 70%, var(--md-sys-color-on-surface-variant));
    opacity: 0.86;
  }

  .meowconnect-header-status.inactive,
  .meowconnect-header-status.offline {
    color: color-mix(in srgb, var(--md-sys-color-error) 55%, var(--md-sys-color-on-surface-variant));
    opacity: 0.78;
  }

  .meowconnect-header-status.inactive img,
  .meowconnect-header-status.offline img {
    filter: grayscale(1);
    opacity: 0.46;
  }

  .meowconnect-request-alert {
    display: inline-flex;
    align-items: center;
    gap: 0.38rem;
    min-height: 1.9rem;
    padding: 0.22rem 0.52rem 0.22rem 0.3rem;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 52%, var(--md-sys-color-outline));
    border-radius: 999px;
    background: color-mix(in srgb, var(--md-sys-color-primary) 10%, var(--md-sys-color-surface-container));
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font-size: 0.72rem;
    font-weight: 800;
    line-height: 1;
    white-space: nowrap;
  }

  .meowconnect-request-alert:hover {
    border-color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 16%, var(--md-sys-color-surface-container));
  }

  .request-avatar-stack {
    position: relative;
    display: block;
    width: 2.95rem;
    height: 1.45rem;
  }

  .request-avatar-stack img,
  .request-avatar-fallback {
    position: absolute;
    top: 0;
    left: calc(var(--request-avatar-index, 0) * 0.78rem);
    width: 1.45rem;
    height: 1.45rem;
    border: 2px solid var(--md-sys-color-surface-container);
    border-radius: 50%;
    box-sizing: border-box;
  }

  .request-avatar-stack img {
    object-fit: cover;
  }

  .request-avatar-fallback,
  .request-avatar-overflow {
    display: grid;
    place-items: center;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    font-size: 0.58rem;
    font-weight: 900;
  }

  .request-avatar-overflow {
    position: absolute;
    right: -0.05rem;
    bottom: -0.08rem;
    min-width: 0.95rem;
    height: 0.95rem;
    padding: 0 0.12rem;
    border: 2px solid var(--md-sys-color-surface-container);
    border-radius: 999px;
    box-sizing: border-box;
  }

  .setup-guide-button {
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    padding: 0.45rem 0.7rem;
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface);
    cursor: pointer;
    font-size: 0.75rem;
    font-weight: 700;
    white-space: nowrap;
  }

  .setup-guide-button:hover {
    border-color: var(--md-sys-color-primary);
    color: var(--md-sys-color-primary);
  }

  @media (max-width: 768px) {
    .app-title-logo {
      width: 148px;
      height: 32px;
    }
  }
</style>
