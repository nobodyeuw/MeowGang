<script lang="ts">
  // Pure To Do matrix shell: renders sections and forwards user actions to Todo.svelte.
  import RosterButtonGroup from '$lib/components/common/RosterButtonGroup.svelte';
  import TodoMatrixHeader from '$lib/components/todo/TodoMatrixHeader.svelte';
  import TodoRaidRow from '$lib/components/todo/TodoRaidRow.svelte';
  import TodoRosterTaskRow from '$lib/components/todo/TodoRosterTaskRow.svelte';
  import TodoTaskRow from '$lib/components/todo/TodoTaskRow.svelte';
  import { RAIDS } from '$lib/data/raids';

  export let matrixData: any;
  export let effectiveTodoRosterId = '';
  export let todoRosterOptions: Array<{ id: string; roster_name: string }> = [];
  export let highlightCharId: number | null = null;
  export let splitRatTodoView = false;
  export let virtualRatRosterId = '';
  export let rosterTaskStates: Record<string, boolean> = {};
  export let rosterEventProgress: Record<string, any> = {};
  export let raidConfigMap: Map<string, Map<number, string>> = new Map();

  export let onSelectTodoRoster: (event: CustomEvent<string>) => void;
  export let getCharacterTaskState: (task: any, character: any) => any;
  export let isRosterEventTask: (taskId: string) => boolean;
  export let onTaskToggle: (characterId: number, taskId: string, state: any) => void;
  export let onRosterTaskToggle: (taskId: string) => void;
  export let onRosterEventToggle: (taskId: string) => void;
  export let onRaidGateToggle: (characterId: number, raidId: string, gateId: string) => void;

  function getTaskIcon(taskId: string): string {
    if (taskId.startsWith('event_')) {
      return '/images/event_quest.webp';
    }

    const raidIds = RAIDS.map((raid) => raid.id);
    if (raidIds.includes(taskId)) {
      return '/images/kazeros-raid.webp';
    }

    const iconMap: Record<string, string> = {
      chaos: '/images/chaos-dungeon.webp',
      guardian: '/images/guardian.png',
      gate: '/images/chaos_gate.png',
      boss: '/images/boss.png',
      guild: '/images/guild.webp',
      cube: '/images/ebony1720.png',
      paradise: '/images/paradise.webp',
      shop: '/images/daily.webp'
    };

    return iconMap[taskId] || '/images/daily.webp';
  }

</script>

<div class="matrix-container">
  <div class="matrix-wrapper todo-matrix-wrapper">
    <RosterButtonGroup
      selectedRosterId={effectiveTodoRosterId}
      extraRosters={todoRosterOptions}
      on:select={onSelectTodoRoster}
    />

    {#if matrixData.characters.length > 0}
      <table class="todo-matrix">
        <TodoMatrixHeader characters={matrixData.characters} {highlightCharId} />
        <tbody>
          {#if matrixData.daily_tasks.length > 0}
            <tr class="section-separator">
              <td class="section-title-cell sticky-col first-col">
                <div class="section-title">DAILY</div>
              </td>
              <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
            </tr>
            {#each matrixData.daily_tasks as task}
              <TodoTaskRow
                {task}
                characters={matrixData.characters}
                showRested={true}
                {getTaskIcon}
                {getCharacterTaskState}
                {onTaskToggle}
              />
            {/each}
          {/if}

          {#if matrixData.roster_tasks.length > 0}
            <tr class="section-separator">
              <td class="section-title-cell sticky-col first-col">
                <div class="section-title">ROSTER WIDE</div>
              </td>
              <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
            </tr>
            {#each matrixData.roster_tasks as task}
              <TodoRosterTaskRow
                {task}
                characterCount={matrixData.characters.length}
                {rosterTaskStates}
                {rosterEventProgress}
                {getTaskIcon}
                {isRosterEventTask}
                {onRosterTaskToggle}
                {onRosterEventToggle}
              />
            {/each}
          {/if}

          {#if matrixData.weekly_tasks.length > 0}
            <tr class="section-separator">
              <td class="section-title-cell sticky-col first-col">
                <div class="section-title">WEEKLY</div>
              </td>
              <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
            </tr>
            {#each matrixData.weekly_tasks as task}
              <TodoTaskRow
                {task}
                characters={matrixData.characters}
                {getTaskIcon}
                {getCharacterTaskState}
                {onTaskToggle}
              />
            {/each}
          {/if}

          {#if matrixData.raids.length > 0}
            <tr class="section-separator">
              <td class="section-title-cell sticky-col first-col">
                <div class="section-title">RAIDS</div>
              </td>
              <td class="section-fill-cell" colspan={matrixData.characters.length}></td>
            </tr>
            {#each matrixData.raids as raid}
              <TodoRaidRow
                {raid}
                characters={matrixData.characters}
                {raidConfigMap}
                {getTaskIcon}
                {onRaidGateToggle}
              />
            {/each}
          {/if}
        </tbody>
      </table>
    {:else}
      <div class="empty-state todo-empty-view">
        <div class="empty-icon">Users</div>
        <h3>No Characters In This View</h3>
        {#if splitRatTodoView && effectiveTodoRosterId !== virtualRatRosterId}
          <p>This roster currently has no gold earners because RAT characters are separated into the RAT To Do view.</p>
        {:else if splitRatTodoView && effectiveTodoRosterId === virtualRatRosterId}
          <p>No RAT characters found across your rosters.</p>
        {:else}
          <p>Add characters to this roster to get started with your daily tasks.</p>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .todo-matrix-wrapper {
    container-type: inline-size;
  }

  .todo-matrix {
    --task-column-width: 200px;
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    background: var(--md-sys-color-surface);
  }

  .sticky-col {
    position: sticky;
    left: 0;
    background: var(--md-sys-color-surface);
  }

  .first-col {
    z-index: 11;
    min-width: var(--task-column-width);
    background: var(--md-sys-color-surface-variant);
  }

  .section-separator td {
    background: var(--app-color-accent-row);
    border-bottom: 1px solid var(--app-color-accent-divider);
    padding: 0.5rem;
    text-align: left;
    font-weight: 600;
    color: var(--app-color-accent-muted);
  }

  .section-separator .section-title-cell {
    background: var(--app-color-accent-soft);
    min-width: 200px;
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
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    text-align: center;
  }

  .empty-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }
</style>
