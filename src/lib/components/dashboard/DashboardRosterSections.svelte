<script lang="ts">
  import type { Character } from '$lib/store';
  import { getGameClassDisplayName, getGameClassIconId } from '$lib/data/classes';
  import CharacterCard from '$lib/components/dashboard/CharacterCard.svelte';
  import type { DashboardCharacterData } from '$lib/components/dashboard/types';

  export let rosters: Array<{ id: string; roster_name: string }> = [];
  export let charactersByRoster: Record<string, Character[]> = {};
  export let dashboardView: 'cards' | 'compact' = 'compact';
  export let characterDataMap: Record<string, DashboardCharacterData> = {};
  export let showDashboardStaticBadges = true;

  function getRosterName(rosterId: string): string {
    return rosters.find((roster) => roster.id === rosterId)?.roster_name || rosterId;
  }
</script>

<div class="characters-grid">
  {#each Object.entries(charactersByRoster) as [rosterId, rosterCharacters]}
    <div class="roster-section roster-{rosterId}">
      <h3 class="roster-title">
        <span class="roster-title-text">
          {getRosterName(rosterId)}
          <span class="character-count">({rosterCharacters.length})</span>
        </span>
      </h3>

      <div class="characters-list" class:compact-list={dashboardView === 'compact'}>
        {#each rosterCharacters as character}
          <CharacterCard
            {character}
            viewMode={dashboardView}
            classIcon={getGameClassIconId(character.class_id)}
            className={getGameClassDisplayName(character.class_id)}
            restedValues={characterDataMap[String(character.char_id)]?.restedValues || []}
            completionStatus={characterDataMap[String(character.char_id)]?.completionStatus || []}
            raidConfigs={characterDataMap[String(character.char_id)]?.raidConfigs || []}
            trackingStatus={characterDataMap[String(character.char_id)]?.trackingStatus || []}
            showStaticBadges={showDashboardStaticBadges}
          />
        {/each}
      </div>
    </div>
  {/each}

  {#if Object.keys(charactersByRoster).length === 0}
    <div class="empty-state">
      <div class="empty-icon">Users</div>
      <h3>No Characters Found</h3>
      <p>Add a roster and characters to get started!</p>
    </div>
  {/if}
</div>

<style>
  .characters-grid {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    width: var(--dashboard-frame-width);
    box-sizing: border-box;
    align-items: center;
  }

  .characters-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, 180px);
    grid-auto-flow: dense;
    grid-auto-rows: 72px;
    gap: 1rem;
    align-items: stretch;
    width: 100%;
    box-sizing: border-box;
    overflow: visible;
    justify-content: center;
  }

  .characters-list:not(.compact-list) :global(.character-card) {
    grid-column: span 2;
    grid-row: span 2;
    min-height: 0;
  }

  .characters-list:not(.compact-list) :global(.character-card.minimal-card) {
    grid-row: span 1;
  }

  .characters-list.compact-list {
    grid-template-columns: repeat(3, minmax(0, 1fr));
    grid-auto-rows: auto;
    grid-auto-flow: row;
    gap: 0.5rem;
    align-items: stretch;
  }

  .characters-list.compact-list :global(.character-card:not(.minimal-card)) {
    grid-column: 1 / -1;
  }

  .roster-section {
    --roster-border-color: color-mix(in srgb, var(--md-sys-color-primary) 45%, transparent);
    --roster-hover-color: color-mix(in srgb, var(--app-color-highlight-text) 55%, transparent);
    box-sizing: border-box;
    background: var(--surface-variant);
    border-radius: 8px;
    padding: 0.7rem 0.75rem 0.75rem;
    box-shadow: var(--app-shadow-sm);
    transition: box-shadow 0.18s ease, border-color 0.18s ease;
    border: 1px solid var(--roster-border-color);
    position: relative;
    width: 100%;
    max-width: none;
  }

  .roster-section::after {
    content: '';
    position: absolute;
    inset: -1px;
    border-radius: inherit;
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.18s ease;
    box-shadow:
      inset 0 0 0 1px var(--roster-hover-color),
      0 0 12px color-mix(in srgb, var(--app-color-highlight-text) 16%, transparent);
  }

  .roster-section:hover {
    border-color: var(--roster-hover-color);
    box-shadow:
      var(--app-shadow-sm),
      0 0 16px color-mix(in srgb, var(--app-color-highlight-text) 12%, transparent);
  }

  .roster-section:hover::after {
    opacity: 1;
  }

  .roster-title {
    margin: 0 0 0.65rem;
    color: var(--roster-border-color);
    font-size: 0.68rem;
    line-height: 1;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    text-transform: uppercase;
    letter-spacing: 0;
  }

  .roster-title-text {
    flex: 0 1 auto;
    max-width: min(70%, 22rem);
    min-width: 0;
    display: inline-flex;
    align-items: baseline;
    justify-content: center;
    gap: 0.25rem;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .character-count {
    flex: 0 0 auto;
    background: transparent;
    color: inherit;
    padding: 0;
    border-radius: 0;
    font-size: 0.62rem;
    font-weight: 500;
  }

  .empty-state {
    text-align: center;
    padding: 4rem 2rem;
    color: var(--on-surface-variant);
  }

  .empty-icon {
    font-size: 4rem;
    margin-bottom: 1rem;
    opacity: 0.5;
  }

  .empty-state h3 {
    margin: 0 0 0.5rem 0;
    color: var(--on-surface);
  }

  .empty-state p {
    margin: 0;
    opacity: 0.8;
  }

  @media (max-width: 768px) {
    .characters-list {
      grid-template-columns: 1fr;
    }

    .characters-list:not(.compact-list) :global(.character-card) {
      grid-column: 1 / -1;
    }

    .characters-list.compact-list {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 480px) {
    .characters-list.compact-list {
      grid-template-columns: 1fr;
    }
  }
</style>
