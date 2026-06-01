<script lang="ts">
  import SegmentedControl from '$lib/components/common/SegmentedControl.svelte';
  import {
    getPotentialBoxPrice,
    getPotentialGoldAmount,
    getRaidGoldValues,
    hasReachedGoldLimit,
    isRaidAlreadyActive
  } from '$lib/components/settings/raid-settings/helpers';
  import type { CharacterRaidConfig, RaidBulkToggleType, RaidMatrixData } from '$lib/components/settings/raid-settings/types';

  type RaidToggleType = RaidBulkToggleType | 'buy_box';

  export let raid: RaidMatrixData;
  export let onToggleRaidExpansion: (contentId: string) => void;
  export let onToggleAllRaidMasters: (raid: RaidMatrixData, type: RaidBulkToggleType) => void;
  export let areAllEligibleRaidMastersActive: (raid: RaidMatrixData, type: RaidBulkToggleType) => boolean;
  export let hasMixedDifficultiesReactive: (char: CharacterRaidConfig, raidId: string) => boolean;
  export let onChangeMasterDifficulty: (contentId: string, charId: number, difficulty: string) => void;
  export let isMasterActiveReactive: (char: CharacterRaidConfig, raidId: string, difficulty: string, type: RaidToggleType) => boolean;
  export let onToggleMasterRaid: (charId: number, raidId: string, difficulty: string, type: RaidToggleType) => void;
  export let onDisabledGoldClick: (char: CharacterRaidConfig) => void;
</script>

<tr class="master-row">
  <td class="raid-name-cell settings-matrix-label-cell settings-matrix-sticky-col settings-matrix-first-col">
    <div class="raid-master-info">
      <button
        class="expand-button"
        class:expanded={raid.is_expanded}
        on:click={() => onToggleRaidExpansion(raid.content_id)}
      >
        <span class="expand-icon">{raid.is_expanded ? 'v' : '>'}</span>
      </button>
      <span class="raid-name">{raid.raid_name}</span>
      <div class="raid-bulk-actions" data-guide="raid-bulk-actions">
        <label
          class="ui-inline-checkbox static-all-control"
          title="Toggle Static/Friends for all eligible characters"
        >
          <input
            type="checkbox"
            checked={areAllEligibleRaidMastersActive(raid, 'reserved_for_static')}
            aria-label={`Toggle Static/Friends for all eligible characters in ${raid.raid_name}`}
            on:change={() => onToggleAllRaidMasters(raid, 'reserved_for_static')}
          />
          <span>All Static</span>
        </label>
        <label
          class="ui-inline-checkbox accent-warning gold-all-control"
          title="Toggle Take Gold for all eligible gold earners"
        >
          <input
            type="checkbox"
            checked={areAllEligibleRaidMastersActive(raid, 'take_gold')}
            aria-label={`Toggle Take Gold for all eligible gold earners in ${raid.raid_name}`}
            on:change={() => onToggleAllRaidMasters(raid, 'take_gold')}
          />
          <span>All Gold</span>
        </label>
      </div>
    </div>
  </td>

  {#each raid.characters as char}
    <td class="difficulty-cell settings-matrix-value-cell">
      {#if char.is_locked}
        <div class="locked-indicator">
          <span class="settings-matrix-muted-pill">iLvl too low</span>
        </div>
      {:else}
        {@const isMixedDifficulty = hasMixedDifficultiesReactive(char, raid.content_id) || char.master_difficulty === 'Mixed'}
        <div class="cell-content">
          <div class="difficulty-selector">
            <SegmentedControl
              options={char.available_difficulties.map((difficulty) => ({ value: difficulty, label: difficulty }))}
              value={isMixedDifficulty ? '' : char.master_difficulty}
              ariaLabel={`${char.char_name} ${raid.raid_name} difficulty`}
              density="compact"
              on:change={(event) => onChangeMasterDifficulty(raid.content_id, char.char_id, event.detail)}
            />

            {#if isMixedDifficulty}
              <div class="mixed-indicator">Mixed</div>
            {/if}
          </div>

          <div class="options-row">
            <label class="settings-matrix-option-toggle static-option" title="Mark this character as reserved for a static or friend run for this raid">
              <input
                type="checkbox"
                checked={isMasterActiveReactive(char, raid.content_id, char.master_difficulty, 'reserved_for_static')}
                on:change={() => onToggleMasterRaid(char.char_id, raid.content_id, char.master_difficulty, 'reserved_for_static')}
              />
              <span class="option-label">Static/Friends</span>
            </label>

            {#if char.earns_gold}
              {@const raidGoldValues = getRaidGoldValues(char, raid.content_id)}
              {@const masterTakeGold = isMasterActiveReactive(char, raid.content_id, char.master_difficulty, 'take_gold')}
              {@const masterBuyBox = isMasterActiveReactive(char, raid.content_id, char.master_difficulty, 'buy_box')}
              {@const potentialBoxPrice = getPotentialBoxPrice(char, raid.content_id)}
              {@const potentialGoldAmount = getPotentialGoldAmount(char, raid.content_id)}
              {@const hasGoldLimit = hasReachedGoldLimit(char)}
              {@const isRaidActive = isRaidAlreadyActive(char, raid.content_id)}
              {@const shouldDisableGold = hasGoldLimit && !isRaidActive}

              <label class="settings-matrix-option-toggle gold-option" data-guide="raid-gold-toggle" title={`Tradable: ${raidGoldValues.tradableGold}g | Bound: ${raidGoldValues.boundGold}g (Total: ${raidGoldValues.totalGold}g)`}>
                <input
                  type="checkbox"
                  checked={masterTakeGold}
                  disabled={shouldDisableGold}
                  on:change={() => {
                    if (shouldDisableGold) {
                      onDisabledGoldClick(char);
                      return;
                    }

                    onToggleMasterRaid(char.char_id, raid.content_id, char.master_difficulty, 'take_gold');
                  }}
                />
                <span class="option-label">Take Gold ({potentialGoldAmount}g)</span>
              </label>
              <label class="settings-matrix-option-toggle">
                <input
                  type="checkbox"
                  checked={masterBuyBox}
                  on:click={() => onToggleMasterRaid(char.char_id, raid.content_id, char.master_difficulty, 'buy_box')}
                />
                <span class="option-label">Buy Box ({potentialBoxPrice}g)</span>
              </label>
            {/if}

            {#if !char.earns_gold}
              {@const nonGoldMasterBuyBox = isMasterActiveReactive(char, raid.content_id, char.master_difficulty, 'buy_box')}
              {@const nonGoldPotentialBoxPrice = getPotentialBoxPrice(char, raid.content_id)}
              <label class="settings-matrix-option-toggle">
                <input
                  type="checkbox"
                  checked={nonGoldMasterBuyBox}
                  on:click={() => onToggleMasterRaid(char.char_id, raid.content_id, char.master_difficulty, 'buy_box')}
                />
                <span class="option-label">Buy Box ({nonGoldPotentialBoxPrice}g)</span>
              </label>
            {/if}
          </div>
        </div>
      {/if}
    </td>
  {/each}
</tr>

<style>
  .master-row {
    background: var(--md-sys-color-surface-container);
  }

  .raid-name-cell {
    min-width: 200px;
  }

  .raid-master-info {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 4px 8px;
    align-items: center;
  }

  .raid-bulk-actions {
    grid-column: 2;
    display: inline-flex;
    gap: 4px;
    align-items: center;
    justify-self: start;
  }

  .expand-button {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 12px;
    color: var(--md-sys-color-on-surface);
    padding: 4px;
    border-radius: 4px;
    transition: all 0.2s ease;
  }

  .expand-button:hover {
    background: var(--md-sys-color-surface-container-highest);
  }

  .raid-name {
    font-weight: 500;
    color: var(--md-sys-color-on-surface);
  }

  .difficulty-cell {
    min-width: 120px;
  }

  .locked-indicator {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    color: var(--md-sys-color-on-surface-variant);
  }

  .difficulty-selector {
    display: flex;
    gap: 4px;
    align-items: center;
    justify-content: center;
    flex-wrap: wrap;
  }

  .mixed-indicator {
    padding: 4px 8px;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 38%, var(--md-sys-color-outline));
    background: color-mix(in srgb, var(--md-sys-color-primary) 12%, var(--md-sys-color-surface));
    color: var(--md-sys-color-primary);
    border-radius: 4px;
    font-size: 10px;
    font-weight: 800;
    text-transform: uppercase;
  }

  .cell-content {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: center;
  }

  .options-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: center;
  }

  @media (max-width: 768px) {
    .raid-name-cell {
      min-width: 150px;
    }

    .difficulty-cell {
      min-width: 100px;
    }
  }
</style>
