<script lang="ts">
  import { activeRosterId, rosters } from '$lib/store';

  export let label = 'Active Roster';

  $: orderedRosters = [...$rosters].sort(
    (a, b) =>
      (a.roster_display_order ?? 0) - (b.roster_display_order ?? 0) ||
      a.roster_name.localeCompare(b.roster_name)
  );

  function selectRoster(rosterId: string) {
    activeRosterId.set(rosterId);
  }
</script>

<div class="roster-button-group" aria-label={label}>
  <span class="roster-button-label">{label}:</span>
  <div class="roster-buttons">
    {#each orderedRosters as roster}
      <button
        type="button"
        class:active={$activeRosterId === roster.id}
        on:click={() => selectRoster(roster.id)}
      >
        {roster.roster_name}
      </button>
    {/each}
  </div>
</div>

<style>
  .roster-button-group {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.75rem 1rem;
    background: var(--md-sys-color-surface);
    border-bottom: 1px solid var(--md-sys-color-outline);
    position: sticky;
    left: 0;
    z-index: 40;
    box-sizing: border-box;
  }

  .roster-button-label {
    flex: 0 0 auto;
    color: var(--md-sys-color-on-surface);
    font-size: 0.875rem;
    font-weight: 700;
  }

  .roster-buttons {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }

  button {
    padding: 0.5rem 1rem;
    background: var(--md-sys-color-surface-container);
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 0.5rem;
    cursor: pointer;
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--md-sys-color-on-surface-variant);
    transition: all 0.2s ease;
    white-space: nowrap;
  }

  button:hover {
    background: var(--md-sys-color-surface-container-highest);
    color: var(--md-sys-color-on-surface);
    border-color: var(--md-sys-color-primary);
  }

  button.active {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
    border-color: var(--md-sys-color-primary);
  }

  @media (max-width: 720px) {
    .roster-button-group {
      align-items: flex-start;
      flex-direction: column;
      gap: 0.55rem;
    }
  }
</style>
