<script lang="ts">
  import SegmentedControl from '$lib/components/common/SegmentedControl.svelte';
  import { getGateData } from '$lib/components/settings/raid-settings/helpers';
  import type { RaidMatrixData } from '$lib/components/settings/raid-settings/types';

  export let raid: RaidMatrixData;
  export let gateName: string;
  export let onChangeGateDifficulty: (contentId: string, charId: number, gateName: string, difficulty: string) => void;
  export let onToggleRaidGate: (charId: number, raidId: string, difficulty: string, gate: string, type: 'take_gold' | 'buy_box') => void;
</script>

<tr class="gate-row">
  <td class="gate-name-cell settings-matrix-label-cell settings-matrix-sticky-col settings-matrix-first-col">
    <div class="gate-info">
      <span class="gate-name">{gateName}</span>
    </div>
  </td>

  {#each raid.characters as char}
    <td class="gate-controls-cell settings-matrix-value-cell">
      {#if !char.is_locked}
        {@const gateConfig = char.raid_configs.find((entry) => entry.content_id === raid.content_id)?.gates.find((gate) => gate.gate === gateName)}
        {@const currentGateBuyBox = gateConfig?.buy_box || false}
        {@const currentGateDifficulty = gateConfig?.difficulty || ''}
        {@const gateData = getGateData(raid.content_id, currentGateDifficulty || 'Solo', gateName)}
        {@const gateBoxPrice = gateData?.boxPrice || 0}
        <div class="cell-content">
          <div class="difficulty-selector">
            <SegmentedControl
              options={char.available_difficulties.map((difficulty) => ({ value: difficulty, label: difficulty }))}
              value={currentGateDifficulty}
              ariaLabel={`${char.char_name} ${raid.raid_name} ${gateName} difficulty`}
              density="compact"
              on:change={(event) => onChangeGateDifficulty(raid.content_id, char.char_id, gateName, event.detail)}
            />
          </div>

          <div class="options-row">
            <label class="settings-matrix-option-toggle">
              <input
                type="checkbox"
                checked={currentGateBuyBox}
                on:click={() => onToggleRaidGate(char.char_id, raid.content_id, currentGateDifficulty || 'Solo', gateName, 'buy_box')}
              />
              <span class="option-label">Buy Box ({gateBoxPrice}g)</span>
            </label>
          </div>
        </div>
      {/if}
    </td>
  {/each}
</tr>

<style>
  .gate-row {
    background: var(--md-sys-color-surface);
  }

  .gate-name-cell {
    padding: 8px 8px 8px 32px;
    font-size: 12px;
    min-width: 200px;
  }

  .gate-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .gate-name {
    font-weight: 500;
    color: var(--md-sys-color-on-surface);
  }

  .gate-controls-cell {
    min-width: 120px;
  }

  .cell-content {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: center;
  }

  .difficulty-selector {
    display: flex;
    gap: 4px;
    align-items: center;
    justify-content: center;
    flex-wrap: wrap;
  }

  .options-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: center;
  }

  @media (max-width: 768px) {
    .gate-name-cell {
      min-width: 150px;
    }
  }
</style>
