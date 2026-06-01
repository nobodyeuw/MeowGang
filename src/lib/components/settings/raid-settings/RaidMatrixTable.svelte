<script lang="ts">
  import RaidMatrixGateRow from '$lib/components/settings/raid-settings/RaidMatrixGateRow.svelte';
  import RaidMatrixHeader from '$lib/components/settings/raid-settings/RaidMatrixHeader.svelte';
  import RaidMatrixMasterRow from '$lib/components/settings/raid-settings/RaidMatrixMasterRow.svelte';
  import type { CharacterRaidConfig, RaidBulkToggleType, RaidMatrixData } from '$lib/components/settings/raid-settings/types';

  type RaidToggleType = RaidBulkToggleType | 'buy_box';

  export let raidMatrix: RaidMatrixData[] = [];
  export let visibleRaidMatrix: RaidMatrixData[] = [];
  export let hasHiddenRaidRows = false;
  export let collapseUntrackedRaidRows = false;
  export let onSetCollapseUntrackedRaidRows: (value: boolean) => void;
  export let onToggleRaidExpansion: (contentId: string) => void;
  export let onToggleAllRaidMasters: (raid: RaidMatrixData, type: RaidBulkToggleType) => void;
  export let areAllEligibleRaidMastersActive: (raid: RaidMatrixData, type: RaidBulkToggleType) => boolean;
  export let getClassIcon: (classId: string) => string;
  export let hasMixedDifficultiesReactive: (char: CharacterRaidConfig, raidId: string) => boolean;
  export let onChangeMasterDifficulty: (contentId: string, charId: number, difficulty: string) => void;
  export let isMasterActiveReactive: (char: CharacterRaidConfig, raidId: string, difficulty: string, type: RaidToggleType) => boolean;
  export let onToggleMasterRaid: (charId: number, raidId: string, difficulty: string, type: RaidToggleType) => void;
  export let onDisabledGoldClick: (char: CharacterRaidConfig) => void;
  export let onChangeGateDifficulty: (contentId: string, charId: number, gateName: string, difficulty: string) => void;
  export let onToggleRaidGate: (charId: number, raidId: string, difficulty: string, gate: string, type: 'take_gold' | 'buy_box') => void;
</script>

<div class="matrix-container" data-guide="raid-matrix">
  <div class="matrix-wrapper">
    <table class="raid-matrix">
      <RaidMatrixHeader
        characters={raidMatrix[0]?.characters || []}
        {hasHiddenRaidRows}
        {collapseUntrackedRaidRows}
        {getClassIcon}
        {onSetCollapseUntrackedRaidRows}
      />
      <tbody>
        {#each visibleRaidMatrix as raid (raid.unique_key || raid.content_id)}
          <RaidMatrixMasterRow
            {raid}
            {onToggleRaidExpansion}
            {onToggleAllRaidMasters}
            {areAllEligibleRaidMastersActive}
            {hasMixedDifficultiesReactive}
            {onChangeMasterDifficulty}
            {isMasterActiveReactive}
            {onToggleMasterRaid}
            {onDisabledGoldClick}
          />

          {#if raid.is_expanded}
            {#each Array.from(raid.gates.keys()).sort() as gateName}
              <RaidMatrixGateRow
                {raid}
                {gateName}
                {onChangeGateDifficulty}
                {onToggleRaidGate}
              />
            {/each}
          {/if}
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .raid-matrix {
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    font-size: 14px;
    min-width: 800px;
  }

  @media (max-width: 768px) {
  }
</style>
