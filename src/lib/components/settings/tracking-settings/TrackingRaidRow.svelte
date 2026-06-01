<script lang="ts">
  export let raid: any;
  export let characters: any[] = [];
  export let areAllEligibleCharactersTrackedForRaid: (raidId: string) => boolean;
  export let onToggleAllCharactersForRaid: (raidId: string, tracked: boolean) => void;
  export let getCharacterRaidState: (raid: any, charId: number) => any;
  export let onToggleRaid: (charId: number, raidId: string, tracked: boolean) => void;
</script>

<tr>
  <td class="task-name-cell settings-matrix-label-cell settings-matrix-sticky-col settings-matrix-first-col">
    <div class="raid-info">
      <span class="raid-name">{raid.raid_name}</span>
      <div class="raid-meta-row">
        <span class="settings-matrix-meta-pill">iLvl: {raid.min_ilvl}</span>
        <label
          class="ui-inline-checkbox"
          title="Toggle all characters"
        >
          <input
            type="checkbox"
            checked={areAllEligibleCharactersTrackedForRaid(raid.raid_id)}
            aria-label={`Toggle all characters for ${raid.raid_name}`}
            on:change={(event) => onToggleAllCharactersForRaid(raid.raid_id, event.currentTarget.checked)}
          />
          <span>All</span>
        </label>
      </div>
    </div>
  </td>
  {#each characters as char}
    <td class="toggle-cell">
      <div class="cell-content">
        {#if raid.min_ilvl <= char.item_level}
          <input
            type="checkbox"
            class="settings-matrix-checkbox"
            checked={getCharacterRaidState(raid, char.char_id)?.tracked || false}
            aria-label={`Toggle ${raid.raid_name} for ${char.name ?? char.character_name ?? 'character'}`}
            on:change={(event) => onToggleRaid(char.char_id, raid.raid_id, event.currentTarget.checked)}
          />
        {:else}
          <div class="settings-matrix-muted-pill">iLvl too low</div>
        {/if}
      </div>
    </td>
  {/each}
</tr>

<style>
  .raid-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .raid-meta-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .raid-name {
    font-weight: 500;
    color: var(--md-sys-color-on-surface);
  }

  .toggle-cell {
    padding: 7px;
    text-align: center;
    border-bottom: 1px solid var(--md-sys-color-outline);
    border-left: 1px solid var(--md-sys-color-outline);
    min-width: 80px;
  }

  .cell-content {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 8px;
    min-height: 32px;
  }

  .task-name-cell.settings-matrix-sticky-col {
    z-index: 15;
  }

  @media (max-width: 768px) {
    .task-name-cell {
      min-width: 150px;
    }
  }
</style>
