<script lang="ts">
  import { flip } from 'svelte/animate';
  import { dndzone } from 'svelte-dnd-action';
  import { appAsset, classAsset, iconAsset } from '$lib/assets';
  import type { Character } from '$lib/store';
  import { getClassIcon, getClassName } from './helpers';

  export let rosterName = '';
  export let dailyUpdateBadge: { label: string; state: string };
  export let dndItems: any[] = [];
  export let dndOptions: any;
  export let onDndConsider: (event: CustomEvent<any>) => void;
  export let onDndFinalize: (event: CustomEvent<any>) => void;
  export let onToggleGold: (char: Character) => void;
  export let onToggleHideFromDashboard: (char: Character) => void;
  // Temporarily disabled due to Supabase realtime message limits
  export let onToggleMeowConnect: (char: Character) => void = () => {};
  export let showMeowConnect = false;
  export let onRequestSoftRemove: (char: Character) => void;

  const visibleIcon = iconAsset('visble.png');
  const invisibleIcon = iconAsset('invisble.png');
  const meowConnectIcon = appAsset('meowconnect_tab.png');
</script>

<div class="character-section">
  <div class="section-header">
    <div class="character-section-title">
      <h4>Characters in {rosterName}</h4>
      <span
        class:ready={dailyUpdateBadge.state === 'ready'}
        class:running={dailyUpdateBadge.state === 'running'}
        class="scrape-countdown-badge"
      >
        {dailyUpdateBadge.label}
      </span>
    </div>
  </div>

  <div
    class="character-list"
    use:dndzone={dndOptions}
    on:consider={onDndConsider}
    on:finalize={onDndFinalize}
  >
    {#each dndItems as char (char.id)}
      <div class="settings-list-card character-item" animate:flip={{ duration: 200 }}>
        <div class="drag-handle" data-guide="character-drag">
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="9" cy="5" r="1"/><circle cx="9" cy="12" r="1"/><circle cx="9" cy="19" r="1"/>
            <circle cx="15" cy="5" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="15" cy="19" r="1"/>
          </svg>
        </div>

        <div class="char-info">
          <img src={classAsset(getClassIcon(char.class_id))} alt="" class="class-icon" />
          <div class="name-box">
            <span class="char-name">{char.char_name}</span>
            <span class="class-name">{getClassName(char.class_id)}</span>
          </div>
          <div class="stats">
            <div class="stat-item">iLvl: {char.item_level.toFixed(2)}</div>
            <div class="stat-item">CP: {char.combat_power.toFixed(2)}</div>
          </div>
        </div>

        <div class="actions">
          <button
            class="toggle-btn gold"
            data-guide="gold-toggle"
            class:active={char.earns_gold}
            on:click|stopPropagation={() => onToggleGold(char)}
            title={char.earns_gold ? 'Character currently earns gold' : 'Character current state is non-gold earner'}
          >
            {char.earns_gold ? 'EARNS GOLD' : 'RAT'}
          </button>
          <button
            class="toggle-btn hide"
            class:active={char.hide_from_dashboard}
            on:click|stopPropagation={() => onToggleHideFromDashboard(char)}
            title={char.hide_from_dashboard ? 'Hidden from dashboard' : 'Visible on dashboard'}
          >
            <img
              class="visibility-icon"
              src={char.hide_from_dashboard ? invisibleIcon : visibleIcon}
              alt=""
            />
            <span>{char.hide_from_dashboard ? 'HIDDEN' : 'VISIBLE'}</span>
          </button>
          <!-- Temporarily disabled due to Supabase realtime message limits -->
          {#if showMeowConnect}
          <button
            class="toggle-btn connect"
            data-guide="meow-connect-toggle"
            class:active={char.meow_connect_enabled}
            on:click|stopPropagation={() => onToggleMeowConnect(char)}
            title={char.meow_connect_enabled ? 'Connected to MeowConnect shared availability' : 'Not connected to MeowConnect shared availability'}
          >
            <img class="connect-icon" src={meowConnectIcon} alt="" />
            <span>{char.meow_connect_enabled ? 'CONNECTED' : 'OFF'}</span>
          </button>
          {/if}
          <button
            class="icon-btn remove-character"
            on:click|stopPropagation={() => onRequestSoftRemove(char)}
            title="Remove character from this roster view"
            aria-label={`Remove ${char.char_name} from roster view`}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M9 3h6l1 2h4v2H4V5h4l1-2Z" />
              <path d="M6 9h12l-1 12H7L6 9Zm4 2v8h2v-8h-2Zm4 0v8h2v-8h-2Z" />
            </svg>
          </button>
        </div>
      </div>
    {:else}
      <div class="empty-state">
        <p>No characters in this roster yet.</p>
        <p>Add a roster to scrape character data.</p>
      </div>
    {/each}
  </div>
</div>

<style>
  .character-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--md-sys-color-outline);
    padding-bottom: 0.75rem;
    background: transparent;
    margin-top: 0;
  }

  .section-header h4 {
    margin: 0;
    color: var(--md-sys-color-primary);
    font-size: 1.08rem;
    font-weight: 600;
  }

  .character-section-title {
    display: flex;
    align-items: baseline;
    gap: 0.65rem;
    min-width: 0;
    flex-wrap: wrap;
  }

  .scrape-countdown-badge {
    display: inline-flex;
    align-items: center;
    max-width: 100%;
    color: color-mix(in srgb, var(--md-sys-color-on-surface-variant) 78%, transparent);
    font-size: 0.72rem;
    font-weight: 600;
    line-height: 1.2;
    opacity: 0.82;
    white-space: nowrap;
  }

  .scrape-countdown-badge.ready {
    color: color-mix(in srgb, var(--md-sys-color-primary) 72%, var(--md-sys-color-on-surface-variant));
  }

  .scrape-countdown-badge.running {
    color: color-mix(in srgb, var(--md-sys-color-warning) 72%, var(--md-sys-color-on-surface-variant));
  }

  .character-list {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .drag-handle {
    display: flex;
    align-items: center;
    padding: 8px;
    color: var(--md-sys-color-on-surface-variant);
    cursor: grab;
  }

  .character-item {
    display: flex;
    align-items: center;
    background: var(--md-sys-color-surface);
    border: 2px solid var(--md-sys-color-outline);
    border-radius: 12px;
    padding: 0.75rem;
    gap: 0.8rem;
    transition: all 0.3s ease;
    cursor: grab;
    box-shadow: var(--app-shadow-sm);
  }

  .character-item:hover {
    background: var(--md-sys-color-surface-variant);
    border-color: var(--md-sys-color-primary);
    transform: translateY(-2px);
    box-shadow: var(--app-shadow-md);
  }

  .character-item:active {
    cursor: grabbing;
  }

  .character-item .drag-handle:hover {
    color: var(--md-sys-color-primary) !important;
  }

  .char-info {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .class-icon {
    width: 32px;
    height: 32px;
    object-fit: contain;
    border-radius: 6px;
  }

  .name-box {
    display: flex;
    flex-direction: column;
    min-width: 120px;
  }

  .char-name {
    color: var(--md-sys-color-on-surface);
    font-weight: 600;
    font-size: 0.9rem;
  }

  .class-name {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.76rem;
  }

  .stats {
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
  }

  .stat-item {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.76rem;
    background: var(--md-sys-color-surface-variant);
    padding: 0.25rem 0.5rem;
    border-radius: 6px;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .toggle-btn {
    padding: 0.42rem 0.62rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface-variant);
    cursor: pointer;
    font-size: 0.68rem;
    font-weight: 600;
    transition: all 0.3s ease;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .toggle-btn:hover {
    background: var(--md-sys-color-surface-container);
    border-color: var(--md-sys-color-primary);
    transform: translateY(-1px);
  }

  .toggle-btn.active {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border-color: var(--md-sys-color-primary);
    box-shadow: var(--app-shadow-sm);
  }

  .toggle-btn.gold.active {
    background: var(--app-color-gold);
    color: var(--app-color-on-gold);
    border-color: var(--app-color-gold);
  }

  .toggle-btn.hide.active {
    background: var(--app-color-hidden);
    color: white;
    border-color: var(--app-color-hidden);
  }

  .toggle-btn.hide,
  .toggle-btn.connect {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }

  .visibility-icon,
  .connect-icon {
    width: 16px;
    height: 16px;
    object-fit: contain;
    display: block;
  }

  .toggle-btn.connect.active {
    background: color-mix(in srgb, var(--md-sys-color-primary) 84%, var(--md-sys-color-tertiary));
    color: var(--md-sys-color-on-primary);
    border-color: color-mix(in srgb, var(--md-sys-color-primary) 70%, var(--md-sys-color-tertiary));
  }

  .icon-btn {
    width: 32px;
    height: 32px;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface-variant);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .icon-btn svg {
    width: 16px;
    height: 16px;
    fill: currentColor;
  }

  .icon-btn:hover {
    transform: translateY(-1px);
    border-color: var(--md-sys-color-primary);
    color: var(--md-sys-color-primary);
  }

  .icon-btn.remove-character:hover {
    border-color: var(--md-sys-color-error);
    color: var(--md-sys-color-error);
    background: color-mix(in srgb, var(--md-sys-color-error) 10%, transparent);
  }

  .empty-state {
    text-align: center;
    padding: 2rem;
    color: var(--md-sys-color-on-surface-variant);
    font-style: italic;
  }

  .empty-state p {
    margin: 0.5rem 0;
  }

  @media (max-width: 768px) {
    .section-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 1rem;
    }

    .character-item {
      flex-direction: column;
      align-items: stretch;
    }

    .char-info {
      flex-direction: column;
      align-items: flex-start;
    }

    .actions {
      justify-content: center;
    }
  }
</style>
