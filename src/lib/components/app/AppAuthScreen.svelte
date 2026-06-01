<script lang="ts">
  import type { DiscordAuthState } from '$lib/types/app-shell';

  export let discordAuthState: DiscordAuthState;
  export let discordAuthUser = '';
  export let discordAuthMessage = '';
  export let loginWithDiscord: () => void | Promise<void>;
  export let proceedFromWelcome: () => void | Promise<void>;
  export let retryDiscordLogin: () => void;
</script>

<svelte:head>
  <script type="text/javascript" async src="https://tenor.com/embed.js"></script>
</svelte:head>

<div class="auth-screen">
  <div class="auth-card">
    <div class="auth-topline">
      <div class="auth-brand">
        <img src="/images/LOAtracker_icon.png" alt="" class="auth-icon" />
        <span>LOA Tracker</span>
      </div>
      <span class="auth-badge">Private Access</span>
    </div>
    <h1>
      {#if discordAuthState === 'welcome'}
        Welcome, {discordAuthUser}
      {:else}
        Only for MeowGang members
      {/if}
    </h1>
    <p class="auth-message">
      {#if discordAuthState === 'welcome'}
        Discord access verified.
      {:else if discordAuthState === 'denied'}
        Not approved by our Meowtator
      {:else}
        {discordAuthMessage}
      {/if}
    </p>
    {#if discordAuthState === 'welcome'}
      <div class="welcome-gif-frame">
        <div
          class="tenor-gif-embed"
          data-postid="16242995"
          data-share-method="host"
          data-aspect-ratio="1"
          data-width="100%"
        >
          <a href="https://tenor.com/view/hello-cute-cat-hi-greetings-gif-16242995">Hello Cute GIF</a>
          from <a href="https://tenor.com/search/hello-gifs">Hello GIFs</a>
        </div>
      </div>
    {/if}
    {#if discordAuthState === 'denied'}
      <div class="denied-gif-frame">
        <div
          class="tenor-gif-embed"
          data-postid="17205935"
          data-share-method="host"
          data-aspect-ratio="1"
          data-width="100%"
        >
          <a href="https://tenor.com/view/cat-animation-slap-gif-17205935">Cat Animation Sticker</a>
          from <a href="https://tenor.com/search/cat-stickers">Cat Stickers</a>
        </div>
        <div class="cat-slap-gif" aria-label="Cat slapping animation">
          <span class="cat-face">:3</span>
          <span class="cat-paw"></span>
        </div>
      </div>
    {/if}

    {#if discordAuthState === 'checking'}
      <button class="auth-button" type="button" disabled>Checking...</button>
    {:else if discordAuthState === 'authorizing'}
      <button class="auth-button" type="button" disabled>Waiting for Discord...</button>
    {:else if discordAuthState === 'welcome'}
      <button class="auth-button" type="button" on:click={proceedFromWelcome}>Proceed</button>
    {:else if discordAuthState === 'denied'}
      <button class="auth-button" type="button" on:click={retryDiscordLogin}>Try another account</button>
    {:else}
      <button class="auth-button" type="button" on:click={loginWithDiscord}>Login with Discord</button>
    {/if}
  </div>
</div>

<style>
  .auth-screen {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1.5rem;
    box-sizing: border-box;
    background:
      linear-gradient(135deg, color-mix(in srgb, var(--md-sys-color-primary) 10%, transparent), transparent 32%),
      linear-gradient(315deg, color-mix(in srgb, var(--app-color-gold) 5%, transparent), transparent 38%),
      var(--md-sys-color-background);
  }

  .auth-card {
    width: min(100%, 390px);
    box-sizing: border-box;
    background: color-mix(in srgb, var(--surface-variant) 94%, black);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 25%, transparent);
    border-radius: 8px;
    padding: 1.15rem;
    text-align: left;
  }

  .auth-topline {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .auth-brand {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.55rem;
    color: var(--on-surface);
    font-size: 0.95rem;
    font-weight: 800;
    line-height: 1;
  }

  .auth-icon {
    width: 34px;
    height: 34px;
    border-radius: 8px;
    object-fit: contain;
    flex: 0 0 34px;
  }

  .auth-badge {
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 25%, transparent);
    border-radius: 999px;
    padding: 0.28rem 0.55rem;
    color: var(--on-surface-variant);
    font-size: 0.68rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0;
  }

  .auth-card h1 {
    margin: 0 0 0.45rem;
    color: var(--on-surface);
    font-size: 1.25rem;
    line-height: 1.15;
  }

  .auth-message {
    margin: 0;
    color: var(--on-surface-variant);
    line-height: 1.4;
    font-size: 0.85rem;
  }

  .denied-gif-frame,
  .welcome-gif-frame {
    position: relative;
    min-height: 150px;
    margin-top: 1rem;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 18%, transparent);
    border-radius: 8px;
    background: color-mix(in srgb, var(--surface-variant) 90%, black);
    overflow: hidden;
  }

  .tenor-gif-embed {
    position: relative;
    z-index: 2;
    width: 100%;
    min-height: 150px;
  }

  .tenor-gif-embed a {
    color: transparent;
    font-size: 0;
  }

  .cat-slap-gif {
    position: absolute;
    inset: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.35rem;
  }

  .cat-face {
    width: 44px;
    height: 44px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--primary);
    color: var(--on-primary);
    font-size: 1rem;
    font-weight: 900;
  }

  .cat-paw {
    width: 34px;
    height: 16px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 92%, var(--app-color-gold));
    transform-origin: left center;
    animation: slap 0.72s ease-in-out infinite;
  }

  .cat-paw::after {
    content: '';
    display: block;
    width: 10px;
    height: 10px;
    margin-left: 24px;
    margin-top: 3px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--md-sys-color-primary) 52%, white);
  }

  @keyframes slap {
    0%, 100% { transform: translateX(8px) rotate(18deg); }
    48% { transform: translateX(-18px) rotate(-14deg); }
    58% { transform: translateX(-18px) rotate(-14deg); }
  }

  .auth-button {
    width: 100%;
    margin-top: 1rem;
    border: 0;
    border-radius: 8px;
    padding: 0.68rem 1rem;
    background: var(--primary);
    color: var(--on-primary);
    font-size: 0.9rem;
    font-weight: 800;
    cursor: pointer;
  }

  .auth-button:disabled {
    cursor: default;
    opacity: 0.72;
  }
</style>
