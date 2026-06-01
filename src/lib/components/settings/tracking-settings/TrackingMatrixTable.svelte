<script lang="ts">
  import TrackingMatrixHeader from '$lib/components/settings/tracking-settings/TrackingMatrixHeader.svelte';
  import TrackingRaidRow from '$lib/components/settings/tracking-settings/TrackingRaidRow.svelte';
  import TrackingRosterTaskRow from '$lib/components/settings/tracking-settings/TrackingRosterTaskRow.svelte';
  import TrackingTaskRow from '$lib/components/settings/tracking-settings/TrackingTaskRow.svelte';

  export let matrixData: any;
  export let visibleDailyTasks: any[] = [];
  export let visibleWeeklyTasks: any[] = [];
  export let visibleRosterTasks: any[] = [];
  export let visibleRaids: any[] = [];
  export let hasHiddenTrackingRows = false;
  export let collapseUntrackedRows = false;
  export let rosterEventProgress: Record<string, any> = {};

  export let onSetCollapseUntrackedRows: (value: boolean) => void;
  export let areAllCharactersTrackedForTask: (taskId: string) => boolean;
  export let onToggleAllCharactersForTask: (taskId: string, tracked: boolean) => void;
  export let areAllCharactersLazyForTask: (taskId: string) => boolean;
  export let onToggleAllLazyDailyForTask: (taskId: string, lazy: boolean) => void;
  export let getCharacterTaskState: (task: any, charId: number) => any;
  export let onToggleTask: (charId: number, taskId: string, tracked: boolean) => void;
  export let onToggleLazyDaily: (charId: number, taskId: string, lazy: boolean) => void;
  export let onRestedWheel: (event: WheelEvent) => void;
  export let onRestedChange: (event: Event, charId: number, contentId: string) => void;
  export let onToggleRosterTask: (taskId: string, tracked: boolean) => void;
  export let onRosterEventCountChange: (event: Event, taskId: string) => void;
  export let areAllEligibleCharactersTrackedForRaid: (raidId: string) => boolean;
  export let onToggleAllCharactersForRaid: (raidId: string, tracked: boolean) => void;
  export let getCharacterRaidState: (raid: any, charId: number) => any;
  export let onToggleRaid: (charId: number, raidId: string, tracked: boolean) => void;
</script>

<div class="matrix-container" data-guide="tracking-matrix">
  <div class="tracking-matrix-wrapper">
    <table class="tracking-matrix">
      <TrackingMatrixHeader
        characters={matrixData.characters}
        {hasHiddenTrackingRows}
        {collapseUntrackedRows}
        {onSetCollapseUntrackedRows}
      />
      <tbody>
        <tr class="section-separator">
          <td class="section-title-cell sticky-col first-col">
            <div class="section-title">DAILY</div>
          </td>
          <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
        </tr>
        {#each visibleDailyTasks as task}
          <TrackingTaskRow
            {task}
            characters={matrixData.characters}
            showRestedControls={true}
            {areAllCharactersTrackedForTask}
            {onToggleAllCharactersForTask}
            {areAllCharactersLazyForTask}
            {onToggleAllLazyDailyForTask}
            {getCharacterTaskState}
            {onToggleTask}
            {onToggleLazyDaily}
            {onRestedWheel}
            {onRestedChange}
          />
        {/each}

        <tr class="section-separator">
          <td class="section-title-cell sticky-col first-col">
            <div class="section-title">WEEKLY</div>
          </td>
          <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
        </tr>
        {#each visibleWeeklyTasks as task}
          <TrackingTaskRow
            {task}
            characters={matrixData.characters}
            {areAllCharactersTrackedForTask}
            {onToggleAllCharactersForTask}
            {areAllCharactersLazyForTask}
            {onToggleAllLazyDailyForTask}
            {getCharacterTaskState}
            {onToggleTask}
            {onToggleLazyDaily}
            {onRestedWheel}
            {onRestedChange}
          />
        {/each}

        <tr class="section-separator">
          <td class="section-title-cell sticky-col first-col">
            <div class="section-title">ROSTER WIDE</div>
          </td>
          <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
        </tr>
        {#each visibleRosterTasks as task}
          <TrackingRosterTaskRow
            {task}
            characterCount={matrixData.characters.length}
            {rosterEventProgress}
            {onToggleRosterTask}
            {onRosterEventCountChange}
            {onRestedWheel}
          />
        {/each}

        <tr class="section-separator">
          <td class="section-title-cell sticky-col first-col">
            <div class="section-title">RAIDS</div>
          </td>
          <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
        </tr>
        {#each visibleRaids as raid}
          <TrackingRaidRow
            {raid}
            characters={matrixData.characters}
            {areAllEligibleCharactersTrackedForRaid}
            {onToggleAllCharactersForRaid}
            {getCharacterRaidState}
            {onToggleRaid}
          />
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .tracking-matrix-wrapper {
    container-type: inline-size;
  }

  .tracking-matrix {
    --task-column-width: 200px;
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    font-size: 14px;
    min-width: 800px;
  }

  .section-separator td {
    background: var(--app-color-accent-row);
    border-bottom: 1px solid var(--app-color-accent-divider);
    padding: 8px 12px;
    font-weight: 600;
    color: var(--app-color-accent-muted);
    text-align: left;
  }

  .section-separator .section-title-cell {
    background: var(--app-color-accent-soft);
    min-width: var(--task-column-width);
    position: sticky;
    left: 0;
    z-index: 18;
  }

  .section-fill-cell {
    min-width: 0;
  }

  .section-title {
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    padding: 0;
    font-size: 16px;
    font-weight: 700;
  }

  .sticky-col {
    position: sticky;
    left: 0;
    z-index: 10;
    background: var(--md-sys-color-surface);
    box-shadow: 2px 0 0 0 var(--md-sys-color-outline);
  }

  .first-col {
    z-index: 20;
    min-width: var(--task-column-width);
    background: var(--md-sys-color-surface-variant);
    box-shadow: 2px 0 0 0 var(--md-sys-color-outline);
  }

  @media (max-width: 768px) {
  }
</style>
