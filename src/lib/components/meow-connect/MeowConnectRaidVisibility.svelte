<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { Raid } from '$lib/data/raids';

  export let raidOptions: Raid[] = [];
  export let visibleRaidIds: string[] = [];

  const dispatch = createEventDispatcher<{
    selectAll: void;
    clear: void;
    toggle: string;
  }>();
</script>

<article class="settings-panel compact-panel raid-visibility-panel">
  <div class="panel-title">
    <div>
      <h3>Raid Visibility</h3>
      <p>Select the raids shown on the Raid Together board.</p>
    </div>
    <div class="panel-actions">
      <button type="button" on:click={() => dispatch('selectAll')}>All</button>
      <button type="button" on:click={() => dispatch('clear')}>None</button>
    </div>
  </div>

  <div class="raid-toggle-grid">
    {#each raidOptions as raid}
      <label class:active={visibleRaidIds.includes(raid.id)}>
        <input
          type="checkbox"
          checked={visibleRaidIds.includes(raid.id)}
          on:change={() => dispatch('toggle', raid.id)}
        />
        <span>{raid.name}</span>
        <small>{raid.gates[0].minIlvl}+</small>
      </label>
    {/each}
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

  p {
    max-width: 60rem;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.76rem;
    line-height: 1.35;
  }

  .panel-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .panel-actions button {
    padding: 0.38rem 0.58rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface-variant);
    font: inherit;
    font-size: 0.76rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.18s ease, color 0.18s ease, border-color 0.18s ease;
  }

  .panel-actions button:hover {
    border-color: var(--md-sys-color-primary);
    color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 8%, transparent);
  }

  .raid-toggle-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(112px, 1fr));
    gap: 0.35rem;
  }

  .raid-toggle-grid label {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 0.34rem;
    align-items: center;
    padding: 0.38rem 0.42rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 8px;
    color: var(--md-sys-color-on-surface);
    background: var(--md-sys-color-surface);
    cursor: pointer;
    transition: border-color 0.18s ease, background 0.18s ease;
  }

  .raid-toggle-grid label:hover {
    border-color: var(--md-sys-color-primary);
  }

  .raid-toggle-grid label.active {
    border-color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 12%, transparent);
  }

  .raid-toggle-grid input {
    accent-color: var(--md-sys-color-primary);
  }

  .raid-toggle-grid span,
  .raid-toggle-grid small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .raid-toggle-grid span {
    font-size: 0.74rem;
  }

  .raid-toggle-grid small {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.68rem;
  }
</style>
