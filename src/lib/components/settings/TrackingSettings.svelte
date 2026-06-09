<script lang="ts">
  // Tracking Settings owns persistence and backend writes; table components only render the matrix.
  import { onMount } from 'svelte';
  import { activeRosterId } from '$lib/store';
  import RosterButtonGroup from '$lib/components/common/RosterButtonGroup.svelte';
  import TrackingMatrixTable from '$lib/components/settings/tracking-settings/TrackingMatrixTable.svelte';
  import {
    buildTrackingMatrixData,
    getVisibleTrackingRows,
    isRosterEventTask,
    isTrackingRowEnabled,
    loadCollapseUntrackedRows,
    saveCollapseUntrackedRows,
    supportsLazyDaily
  } from '$lib/components/settings/tracking-settings/helpers';
  import {
    loadRosterEventProgressCommand,
    loadTrackingConfigMatrix,
    saveRestedValueCommand,
    updateLazyDailyConfigCommand,
    updateRosterEventWeeklyCountCommand,
    updateTrackingConfigCommand,
    type RosterEventProgress
  } from '$lib/services/tracking-settings';

  let selectedCharacterId: number | null = null;

  let matrixData: any | null = null;
  let isLoading = true;
  let error = '';
  let lastLoadedRosterId: string = '';
  let warningMessage = '';
  let rosterEventProgress: Record<string, RosterEventProgress> = {};
  let collapseUntrackedRows = loadCollapseUntrackedRows();
  $: visibleDailyTasks = getVisibleTrackingRows(matrixData?.daily_tasks || [], collapseUntrackedRows);
  $: visibleWeeklyTasks = getVisibleTrackingRows(matrixData?.weekly_tasks || [], collapseUntrackedRows);
  $: visibleRosterTasks = getVisibleTrackingRows(matrixData?.roster_tasks || [], collapseUntrackedRows);
  $: visibleRaids = getVisibleTrackingRows(matrixData?.raids || [], collapseUntrackedRows);
  $: hasHiddenTrackingRows = Boolean(matrixData) && [
    ...(matrixData?.daily_tasks || []),
    ...(matrixData?.weekly_tasks || []),
    ...(matrixData?.roster_tasks || []),
    ...(matrixData?.raids || [])
  ].some((row: any) => !isTrackingRowEnabled(row));

  function setCollapseUntrackedRows(value: boolean) {
    collapseUntrackedRows = value;
    saveCollapseUntrackedRows(value);
  }

  async function loadRosterEventProgress() {
    const rosterId = $activeRosterId;
    const eventTasks = matrixData?.roster_tasks?.filter((task: any) => isRosterEventTask(task.content_id)) || [];
    const nextProgress: Record<string, RosterEventProgress> = {};

    for (const task of eventTasks) {
      try {
        const progress = await loadRosterEventProgressCommand(rosterId, task.content_id);
        nextProgress[task.content_id] = progress;
      } catch (err) {
        console.warn('Failed to load roster event progress:', err);
      }
    }

    rosterEventProgress = nextProgress;
  }

  function syncRosterEventTaskState(taskId: string) {
    const progress = rosterEventProgress[taskId];
    if (!progress || !matrixData?.roster_tasks) return;

    matrixData.roster_tasks = matrixData.roster_tasks.map((task: any) => {
      if (task.content_id !== taskId) return task;
      return {
        ...task,
        character_states: task.character_states.map((state: any) => ({
          ...state,
          completed: progress.completed_this_week >= progress.weekly_limit
        }))
      };
    });
    matrixData = { ...matrixData };
  }

  async function loadMatrixData() {
    try {
      error = '';
      matrixData = null;
      isLoading = true;
      
      const baseMatrix = await loadTrackingConfigMatrix($activeRosterId);
      
      const { matrixData: nextMatrixData, lowIlvlTrackingClears } = buildTrackingMatrixData(baseMatrix);

      // Characters below a raid's min iLvl should not keep stale tracking rows enabled.
      if (lowIlvlTrackingClears.length > 0) {
        await Promise.all(lowIlvlTrackingClears.map((clear) =>
          updateTrackingConfigCommand(clear.characterId, clear.taskId, false).catch((err) => {
            console.warn('Failed to clear low ilvl tracking for character', clear.characterId, 'raid', clear.taskId, err);
          })
        ));
      }
      
      matrixData = nextMatrixData;
      
      if (!matrixData || !matrixData.characters || matrixData.characters.length === 0) {
        console.error('No characters found in matrix data!');
        error = 'No characters found for this roster';
        return;
      }

      await loadRosterEventProgress();
    } catch (err) {
      error = `Error loading matrix: ${err}`;
    } finally {
      isLoading = false;
    }
  }

  async function updateRestedValue(charId: number, contentId: string, currentValue: number) {
    try {
      await saveRestedValueCommand(charId, contentId, currentValue);
      
      // Update local data instead of reloading entire matrix
      const updateTaskState = (tasks: any[]) => {
        for (const task of tasks) {
          const state = task.character_states.find((s: any) => s.char_id === charId);
          if (state) {
            state.current_value = currentValue;
          }
        }
      };
      
      updateTaskState(matrixData.daily_tasks);
      updateTaskState(matrixData.weekly_tasks);
      updateTaskState(matrixData.roster_tasks);
      
    } catch (err) {
      console.error('Failed to update rested value:', err);
    }
  }

  function showWarning(message: string) {
    warningMessage = message;
  }

  function isValidRestedValue(value: number): boolean {
    return Number.isInteger(value) && value >= 0 && value <= 100 && value % 10 === 0;
  }

  function handleRestedWheel(event: WheelEvent) {
    event.preventDefault();
    (event.currentTarget as HTMLInputElement).blur();
  }

  function handleRestedChange(event: Event, charId: number, contentId: string) {
    const input = event.currentTarget as HTMLInputElement;
    const rawValue = input.value.trim();
    const nextValue = Number(rawValue);
    const currentState = matrixData?.daily_tasks
      ?.find((task: any) => task.content_id === contentId)
      ?.character_states
      ?.find((state: any) => state.char_id === charId);
    const previousValue = currentState?.current_value || 0;

    if (rawValue === '' || !Number.isFinite(nextValue) || !isValidRestedValue(nextValue)) {
      input.value = String(previousValue);
      showWarning('Rested values must be 0, 10, 20, ... 100.');
      return;
    }

    warningMessage = '';
    updateRestedValue(charId, contentId, nextValue);
  }

  async function handleRosterEventCountChange(event: Event, taskId: string) {
    const input = event.currentTarget as HTMLInputElement;
    const rawValue = input.value.trim();
    const nextValue = Number(rawValue);
    const progress = rosterEventProgress[taskId];
    const weeklyLimit = progress?.weekly_limit ?? 3;
    const previousValue = progress?.completed_this_week ?? 0;

    if (
      rawValue === '' ||
      !Number.isInteger(nextValue) ||
      nextValue < 0 ||
      nextValue > weeklyLimit
    ) {
      input.value = String(previousValue);
      showWarning(`Event completions must be 0, 1, 2, or 3.`);
      return;
    }

    try {
      warningMessage = '';
      await updateRosterEventWeeklyCountCommand($activeRosterId, taskId, nextValue);
      await loadRosterEventProgress();
      syncRosterEventTaskState(taskId);
      window.dispatchEvent(new CustomEvent('roster-event-progress-updated', { detail: { taskId } }));
    } catch (err) {
      input.value = String(previousValue);
      showWarning(`Failed to update event completions: ${err}`);
    }
  }

  function getCharacterTaskState(task: any, charId: number): any {
    return task.character_states.find((state: any) => state.char_id === charId);
  }

  function getCharacterRaidState(raid: any, charId: number): any {
    return raid.character_states.find((state: any) => state.char_id === charId);
  }

  async function toggleTask(charId: number, taskId: string, newState: boolean) {
    try {
      await updateTrackingConfigCommand(charId, taskId, newState);
      
      // Update local data for the specific task row only
      const updateTaskState = (tasks: any[]) => {
        const task = tasks.find((t: any) => t.content_id === taskId);
        if (task) {
          task.character_states = task.character_states.map((s: any) =>
            s.char_id === charId ? { ...s, tracked: newState } : { ...s }
          );
        }
      };
      
      updateTaskState(matrixData.daily_tasks);
      updateTaskState(matrixData.weekly_tasks);
      updateTaskState(matrixData.roster_tasks);
      matrixData = { ...matrixData };
      
    } catch (err) {
      console.error('Failed to toggle task:', err);
    }
  }

  async function toggleLazyDaily(charId: number, taskId: string, newState: boolean) {
    try {
      await updateLazyDailyConfigCommand(charId, taskId, newState);

      const task = matrixData?.daily_tasks?.find((t: any) => t.content_id === taskId);
      if (task) {
        task.character_states = task.character_states.map((s: any) =>
          s.char_id === charId ? { ...s, lazy_daily: newState } : { ...s }
        );
      }
      matrixData = { ...matrixData };
    } catch (err) {
      console.error('Failed to toggle lazy daily:', err);
    }
  }

  async function toggleRaid(charId: number, raidId: string, newState: boolean) {
    try {
      await updateTrackingConfigCommand(charId, raidId, newState);
      
      // Update only the targeted raid row for this character
      matrixData.raids = matrixData.raids.map((raid: any) => {
        if (raid.raid_id !== raidId) return raid;
        return {
          ...raid,
          character_states: raid.character_states.map((s: any) =>
            s.char_id === charId ? { ...s, tracked: newState } : { ...s }
          )
        };
      });
      matrixData = { ...matrixData };
      window.dispatchEvent(new CustomEvent('tracking-config-changed', {
        detail: { type: 'raid', characterId: charId, contentId: raidId, tracked: newState }
      }));
       
    } catch (err) {
      console.error('Failed to toggle raid:', err);
    }
  }

  async function toggleRosterTask(taskId: string, newState: boolean) {
    try {
      const characters = matrixData?.characters || [];

      const firstCharacter = characters[0];
      if (!firstCharacter) return;
      await updateTrackingConfigCommand(firstCharacter.char_id, taskId, newState);
      
      // Update local data and force reactivity
      matrixData.roster_tasks = matrixData.roster_tasks.map((t: any) => {
        if (t.content_id === taskId) {
          return { ...t, character_states: t.character_states.map((s: any) => ({ ...s, tracked: newState })) };
        }
        return { ...t };
      });
      matrixData = { ...matrixData };
      if (isRosterEventTask(taskId)) {
        await loadRosterEventProgress();
        syncRosterEventTaskState(taskId);
        window.dispatchEvent(new CustomEvent('roster-event-progress-updated', { detail: { taskId } }));
      }
      
    } catch (err) {
      console.error('Failed to toggle roster task:', err);
    }
  }

  async function toggleAllCharactersForTask(taskId: string, newState: boolean) {
    try {
      const characters = matrixData?.characters || [];
      
      for (const char of characters) {
        await updateTrackingConfigCommand(char.char_id, taskId, newState);
      }
      
      // Update local data for all task sections and force reactivity
      const updateTaskState = (tasks: any[]) => {
        const task = tasks.find((t: any) => t.content_id === taskId);
        if (task) {
          task.character_states = task.character_states.map((s: any) => ({ ...s, tracked: newState }));
        }
      };
      
      updateTaskState(matrixData.daily_tasks);
      updateTaskState(matrixData.weekly_tasks);
      matrixData = { ...matrixData };
      
    } catch (err) {
      console.error('Failed to toggle all characters for task:', err);
    }
  }

  async function toggleAllLazyDailyForTask(taskId: string, newState: boolean) {
    try {
      const characters = matrixData?.characters || [];

      for (const char of characters) {
        await updateLazyDailyConfigCommand(char.char_id, taskId, newState);
      }

      const task = matrixData?.daily_tasks?.find((t: any) => t.content_id === taskId);
      if (task) {
        task.character_states = task.character_states.map((s: any) => ({ ...s, lazy_daily: newState }));
      }
      matrixData = { ...matrixData };
    } catch (err) {
      console.error('Failed to toggle lazy daily for all characters:', err);
    }
  }

  async function toggleAllCharactersForRaid(raidId: string, newState: boolean) {
    try {
      const characters = matrixData?.characters || [];
      
      for (const char of characters) {
        const raid = matrixData.raids.find((r: any) => r.raid_id === raidId);
        if (raid) {
          const charState = raid.character_states.find((s: any) => s.char_id === char.char_id);
          // Only toggle eligible characters (ilvl high enough)
          if (raid.min_ilvl <= char.item_level) {
            await updateTrackingConfigCommand(char.char_id, raidId, newState);
          }
        }
      }
      
      // Update local data and force reactivity
      const raid = matrixData.raids.find((r: any) => r.raid_id === raidId);
      if (raid) {
        raid.character_states = raid.character_states.map((s: any, i: number) => {
          const char = matrixData.characters[i];
          if (raid.min_ilvl <= char.item_level) {
            return { ...s, tracked: newState };
          }
          return { ...s };
        });
      }
      matrixData = { ...matrixData };
      window.dispatchEvent(new CustomEvent('tracking-config-changed', {
        detail: { type: 'raid', contentId: raidId, tracked: newState }
      }));
      
    } catch (err) {
      console.error('Failed to toggle all characters for raid:', err);
    }
  }

  function areAllCharactersTrackedForTask(taskId: string): boolean {
    const checkTaskState = (tasks: any[]) => {
      const task = tasks.find((t: any) => t.content_id === taskId);
      if (!task || task.character_states.length === 0) return false;
      return task.character_states.every((state: any) => state.tracked === true);
    };
    
    return checkTaskState(matrixData.daily_tasks) || checkTaskState(matrixData.weekly_tasks);
  }

  function areAllCharactersLazyForTask(taskId: string): boolean {
    const task = matrixData?.daily_tasks?.find((t: any) => t.content_id === taskId);
    if (!task || task.character_states.length === 0) return false;
    return task.character_states.every((state: any) => state.lazy_daily === true);
  }

  function areAllEligibleCharactersTrackedForRaid(raidId: string): boolean {
    const raid = matrixData?.raids?.find((r: any) => r.raid_id === raidId);
    if (!raid || raid.character_states.length === 0) return false;
    const eligibleStates = raid.character_states.filter((_: any, i: number) => {
      const char = matrixData.characters[i];
      return raid.min_ilvl <= char.item_level;
    });
    if (eligibleStates.length === 0) return false;
    return eligibleStates.every((state: any) => state.tracked === true);
  }

  // Load data when component mounts or roster changes
  $: if ($activeRosterId && $activeRosterId !== lastLoadedRosterId) {
    lastLoadedRosterId = $activeRosterId;
    loadMatrixData();
  }

  onMount(() => {
    const handleRosterEventProgressUpdated = async (event: Event) => {
      const detail = (event as CustomEvent<{ taskId?: string }>).detail;
      await loadRosterEventProgress();
      if (detail?.taskId) {
        syncRosterEventTaskState(detail.taskId);
      }
    };

    window.addEventListener('roster-event-progress-updated', handleRosterEventProgressUpdated);
    return () => {
      window.removeEventListener('roster-event-progress-updated', handleRosterEventProgressUpdated);
    };
  });
</script>

<div class="tracking-settings">
  <RosterButtonGroup />

  {#if warningMessage}
    <div class="status-indicator warning">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <line x1="12" y1="8" x2="12" y2="12"/>
        <line x1="12" y1="16" x2="12.01" y2="16"/>
      </svg>
      {warningMessage}
    </div>
  {/if}

  {#if isLoading}
    <div class="loading">Loading tracking data...</div>
  {:else if error}
    <div class="error">
      <p>{error}</p>
      <button on:click={loadMatrixData}>Retry</button>
    </div>
  {:else if matrixData}
    <TrackingMatrixTable
      {matrixData}
      {visibleDailyTasks}
      {visibleWeeklyTasks}
      {visibleRosterTasks}
      {visibleRaids}
      {hasHiddenTrackingRows}
      {collapseUntrackedRows}
      {rosterEventProgress}
      onSetCollapseUntrackedRows={setCollapseUntrackedRows}
      {areAllCharactersTrackedForTask}
      onToggleAllCharactersForTask={toggleAllCharactersForTask}
      {areAllCharactersLazyForTask}
      onToggleAllLazyDailyForTask={toggleAllLazyDailyForTask}
      {getCharacterTaskState}
      onToggleTask={toggleTask}
      onToggleLazyDaily={toggleLazyDaily}
      onRestedWheel={handleRestedWheel}
      onRestedChange={handleRestedChange}
      onToggleRosterTask={toggleRosterTask}
      onRosterEventCountChange={handleRosterEventCountChange}
      {areAllEligibleCharactersTrackedForRaid}
      onToggleAllCharactersForRaid={toggleAllCharactersForRaid}
      {getCharacterRaidState}
      onToggleRaid={toggleRaid}
    />
  {:else}
    <div class="no-data">
      <p>No tracking data loaded. Please select an active roster above.</p>
    </div>
  {/if}
</div>

<style>
  .tracking-settings {
    display: flex;
    flex-direction: column;
    padding: 0;
    flex: 1 1 0;
    min-height: 0;
    height: 100%;
    overflow: hidden;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 500;
    border-radius: 6px;
    margin: 8px 16px 0;
    color: var(--md-sys-color-on-surface);
    background: color-mix(in srgb, var(--md-sys-color-surface-variant) 50%, transparent);
  }

  .status-indicator.warning svg {
    color: var(--md-sys-color-error);
    flex: 0 0 auto;
  }

  .loading, .error {
    text-align: center;
    padding: 40px;
    color: var(--md-sys-color-on-surface);
  }

  .error {
    color: var(--md-sys-color-error);
  }

  .error button {
    margin-top: 10px;
    padding: 8px 16px;
    background: var(--md-sys-color-error);
    color: var(--md-sys-color-on-error);
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .no-data {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: var(--md-sys-color-on-surface-variant);
    padding: 1rem;
    text-align: center;
  }

</style>
