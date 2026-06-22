<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { iconAsset } from '$lib/assets';
  import type { Character } from '$lib/store';
  import { getGameClassInfo } from '$lib/data';
  import { activeFilterCharId, activeRosterId } from '$lib/store';
  import { updateTodoRaidGateStatus, updateTodoTaskStatus } from '$lib/services/todo';
  import {
    buildDisplayRaids,
    buildGroupedRaids,
    buildTrackedRaidIds,
    buildTrackedWeeklyTasks,
    formatCombatPower,
    formatItemLevel,
    getClassIconUrl,
    getDailyIconTitle,
    getLastCompletedGate,
    getNextOpenGate,
    getRaidDisplayName,
    getRaidName,
    getTaskIcon,
    normalizeDifficulty,
    type CharacterCardCompletionEntry,
    type CharacterCardRaidConfig,
    type CharacterCardTrackingEntry
  } from '$lib/components/dashboard/character-card-helpers';
  import {
    clearDashboardCalendarAssignment,
    clearDashboardRaidReservation,
    clearDashboardRaidReservationNoDispatch,
    dispatchCalendarChanged,
    getReservationForRaid,
    saveDashboardCalendarAssignment,
    saveDashboardRaidReservation,
    type DashboardCalendarAssignment,
    type DashboardCalendarEvent,
    type DashboardRaidReservation
  } from '$lib/services/dashboard-calendar';
  
  export let character: Character;
  export let classIcon: string = '';
  export let className: string = '';
  export let viewMode: 'cards' | 'compact' = 'cards';
  export let restedValues: Array<{ content_id: string; current_value: number }> = [];
  export let completionStatus: CharacterCardCompletionEntry[] = [];
  export let raidConfigs: CharacterCardRaidConfig[] = [];
  export let trackingStatus: CharacterCardTrackingEntry[] = [];
  export let showStaticBadges = true;
  export let calendarEvents: DashboardCalendarEvent[] = [];
  export let calendarAssignments: DashboardCalendarAssignment[] = [];
  export let raidReservations: DashboardRaidReservation[] = [];

  const goldIcon = iconAsset('gold.png');
  const raidIcon = iconAsset('kazeros-raid.webp');
  let raidActionMenu: { raid: any; x: number; y: number } | null = null;
  let reservationPickerOpen = false;
  let reservationDate = '';
  let reservationTime = '';
  let clearReservationDialogOpen = false;
  let reservationsToClear = new Set<string>();

  // Reactive values
  $: classInfo = getGameClassInfo(character.class_id);
  $: displayName = className || (classInfo ? classInfo.displayName : "Unknown Class");
  $: iconId = classIcon || (classInfo ? classInfo.iconId : "0");

  $: trackedRaidIds = buildTrackedRaidIds(trackingStatus);
  $: groupedRaids = buildGroupedRaids(raidConfigs, trackedRaidIds);
  $: displayRaids = buildDisplayRaids({
    groupedRaids,
    completionStatus,
    characterEarnsGold: character.earns_gold,
    showStaticBadges,
    raidConfigs
  });
  $: trackedWeeklyTasks = buildTrackedWeeklyTasks(trackingStatus, completionStatus);
  $: displayWeeklyTasks = character.earns_gold || displayRaids.length > 0
    ? []
    : trackedWeeklyTasks.slice(0, 4);
  $: cardTopWeeklyTasks = displayRaids.length > 0
    ? trackedWeeklyTasks
    : displayWeeklyTasks.filter(task => ['shop', 'guild'].includes(task.content_id) || displayWeeklyTasks.length === 1);
  // Card view keeps weekly icons in fixed slots so dailies stay aligned on the right.
  $: cardTopWeeklySlotIds = displayRaids.length > 0
    ? ['cube', 'paradise', 'shop', 'guild']
    : ['shop', 'guild'];
  $: cardTopWeeklySlots = cardTopWeeklySlotIds.map((taskId) =>
    cardTopWeeklyTasks.find((task) => task.content_id === taskId) || null
  );
  $: hasFixedCardTopWeeklySlots = cardTopWeeklySlots.some(Boolean);
  $: looseCardTopWeeklyTasks = hasFixedCardTopWeeklySlots ? [] : cardTopWeeklyTasks;
  $: cardBodyWeeklyTasks = displayRaids.length === 0
    ? displayWeeklyTasks.filter(task => !['shop', 'guild'].includes(task.content_id) && displayWeeklyTasks.length !== 1)
    : displayWeeklyTasks;
  $: compactWeeklyTasks = !isMinimalCard && displayRaids.length > 0 ? trackedWeeklyTasks : [];
  $: trackedWeeklyTaskCount = trackedWeeklyTasks.length;
  $: hasCompactLabels = displayRaids.length > 0 || displayWeeklyTasks.length > 0;
  $: characterReservations = raidReservations.filter((reservation) => reservation.charId === character.char_id);
  $: characterAssignments = calendarAssignments.filter((assignment) => assignment.charId === character.char_id);

type CombinedReservation = DashboardRaidReservation | {
  isAssignment: true;
  eventKey: string;
  contentId: string;
  difficulty: string;
  scheduledAt: null;
  recurringWeekly: boolean;
  charId: number;
  charName: string;
};

$: allReservations = [
    ...characterReservations,
    ...characterAssignments.map((a) => ({
      isAssignment: true,
      eventKey: a.eventKey,
      contentId: a.eventKey.split('-')[0] || 'unknown',
      difficulty: '',
      scheduledAt: null,
      recurringWeekly: false,
      charId: a.charId,
      charName: a.charName
    }))
  ];

  // Chaos and Guardian status
  $: chaosRested = restedValues.find(r => r.content_id === 'chaos')?.current_value || 0;
  $: guardianRested = restedValues.find(r => r.content_id === 'guardian')?.current_value || 0;
  $: chaosCompleted = completionStatus.some(c => c.content_id === 'chaos' && Number(c.is_completed) === 1);
  $: guardianCompleted = completionStatus.some(c => c.content_id === 'guardian' && Number(c.is_completed) === 1);

  // Check if chaos/guardian are tracked for this character
  $: chaosTracking = trackingStatus.find(t => t.content_id === 'chaos');
  $: guardianTracking = trackingStatus.find(t => t.content_id === 'guardian');
  $: chaosConfigured = chaosTracking ? Number(chaosTracking.is_tracked) === 1 : true;
  $: guardianConfigured = guardianTracking ? Number(guardianTracking.is_tracked) === 1 : true;
  $: chaosLazyWaiting = Number(chaosTracking?.lazy_daily ?? 0) === 1 && chaosRested < 20;
  $: guardianLazyWaiting = Number(guardianTracking?.lazy_daily ?? 0) === 1 && guardianRested < 20;
  $: chaosAvailable = chaosConfigured && !chaosCompleted && !chaosLazyWaiting;
  $: guardianAvailable = guardianConfigured && !guardianCompleted && !guardianLazyWaiting;
  $: chaosIconTitle = getDailyIconTitle('chaos', chaosCompleted, chaosLazyWaiting);
  $: guardianIconTitle = getDailyIconTitle('guardian', guardianCompleted, guardianLazyWaiting);
  $: isMinimalCard =
    displayRaids.length === 0 &&
    trackedWeeklyTaskCount <= 1 &&
    (chaosConfigured || guardianConfigured);
  $: isDailyOnlyMinimalCard = isMinimalCard && trackedWeeklyTaskCount === 0;

  function handleCharacterClick() {
    // Set active filter character in global store
    activeFilterCharId.set(character.char_id);
    
    // Set active roster to this character's roster
    activeRosterId.set(character.roster_id);
    
    // Navigate to ToDo tab
    goto(`/?tab=todo&char=${character.char_id}`);
  }

  async function completeDashboardTask(contentId: string, completed: boolean, event?: MouseEvent) {
    event?.preventDefault();
    await updateTodoTaskStatus(character.char_id, contentId, !completed);
    dispatchDashboardCompletionUpdate();
  }

  async function undoDashboardTask(contentId: string, event: MouseEvent) {
    event.preventDefault();
    await updateTodoTaskStatus(character.char_id, contentId, false);
    dispatchDashboardCompletionUpdate();
  }

  async function completeDashboardRaidGate(raid: any, event?: MouseEvent) {
    event?.preventDefault();
    if (raid.completed) {
      await undoDashboardRaidGate(raid, event as MouseEvent);
      return;
    }
    const nextGate = getNextOpenGate(completionStatus, raid.content_id, raid.difficulty);
    if (!nextGate) return;

    await updateTodoRaidGateStatus(character.char_id, raid.content_id, nextGate, raid.content_id, true);
    window.dispatchEvent(new CustomEvent('raid-completed'));
    dispatchDashboardCompletionUpdate();
  }

  function openRaidActionMenu(raid: any, event: MouseEvent) {
    event.preventDefault();
    const now = new Date();
    reservationDate = now.toISOString().slice(0, 10);
    reservationTime = now.toTimeString().slice(0, 5);
    reservationPickerOpen = false;
    const menuWidth = 260;
    const menuHeight = 390;
    const padding = 10;
    raidActionMenu = {
      raid,
      x: Math.min(Math.max(padding, event.clientX), window.innerWidth - menuWidth - padding),
      y: Math.min(Math.max(padding, event.clientY), window.innerHeight - menuHeight - padding)
    };
  }

  function closeRaidActionMenu() {
    raidActionMenu = null;
    reservationPickerOpen = false;
  }

  async function assignRaidToEvent(event: DashboardCalendarEvent) {
    if (!raidActionMenu) return;
    await saveDashboardCalendarAssignment(event, character, raidActionMenu.raid.content_id);
    closeRaidActionMenu();
    // Ensure the component reactivity updates
    dispatchCalendarChanged();
  }

  function openReserveRaidOncePicker() {
    if (!raidActionMenu) return;
    reservationPickerOpen = true;
  }

  async function reserveRaidOnce() {
    if (!raidActionMenu) return;
    const parsed = Date.parse(`${reservationDate}T${reservationTime}`);
    if (!Number.isFinite(parsed)) {
      window.alert('Could not read that date/time.');
      return;
    }
    await saveDashboardRaidReservation({
      charId: character.char_id,
      contentId: raidActionMenu.raid.content_id,
      difficulty: raidActionMenu.raid.difficulty,
      label: new Intl.DateTimeFormat(undefined, {
        weekday: 'short',
        day: '2-digit',
        month: '2-digit',
        hour: '2-digit',
        minute: '2-digit'
      }).format(new Date(parsed)),
      scheduledAt: parsed,
      recurringWeekly: false
    });
    closeRaidActionMenu();
  }

  async function reserveRaidWeekly() {
    if (!raidActionMenu) return;
    const label = window.prompt('Static weekly reservation label', 'Static')?.trim();
    if (!label) return;
    await saveDashboardRaidReservation({
      charId: character.char_id,
      contentId: raidActionMenu.raid.content_id,
      difficulty: raidActionMenu.raid.difficulty,
      label,
      recurringWeekly: true
    });
    closeRaidActionMenu();
  }

  function clearRaidReservation() {
    if (!raidActionMenu) return;
    clearReservationDialogOpen = true;
    reservationsToClear.clear();
    // Pre-select the current reservation
    const reservationKey = `${raidActionMenu.raid.content_id}-${raidActionMenu.raid.difficulty}`;
    reservationsToClear.add(reservationKey);
  }

  function clearSelectedReservations() {
    for (const reservationKey of reservationsToClear) {
      if (reservationKey.startsWith('assignment-')) {
        const eventKey = reservationKey.replace('assignment-', '');
        clearDashboardCalendarAssignment(eventKey);
      } else {
        const [contentId, difficulty] = reservationKey.split('-');
        clearDashboardRaidReservationNoDispatch(character.char_id, contentId, difficulty);
      }
    }
    clearReservationDialogOpen = false;
    reservationsToClear.clear();
    closeRaidActionMenu();
    dispatchCalendarChanged();
  }

  function toggleReservationToClear(reservationKey: string) {
    if (reservationsToClear.has(reservationKey)) {
      reservationsToClear.delete(reservationKey);
    } else {
      reservationsToClear.add(reservationKey);
    }
  }

  function getReservationKey(reservation: DashboardRaidReservation): string {
    return `${reservation.contentId}-${reservation.difficulty}`;
  }

  function getReservationKeyForCombined(reservation: CombinedReservation): string {
    if ('isAssignment' in reservation && reservation.isAssignment) {
      return `assignment-${reservation.eventKey}`;
    }
    if ('contentId' in reservation && 'difficulty' in reservation) {
      return `${reservation.contentId}-${reservation.difficulty}`;
    }
    return '';
  }

  function getRaidAssignment(raid: any): DashboardCalendarAssignment | undefined {
    return characterAssignments.find((assignment) =>
      assignment.raidContentId === raid.content_id
    );
  }

  function getRaidReservation(raid: any): DashboardRaidReservation | undefined {
    return getReservationForRaid(character.char_id, raid.content_id, raid.difficulty)
      || raidReservations.find((reservation) =>
        reservation.charId === character.char_id &&
        reservation.contentId === raid.content_id &&
        reservation.difficulty === raid.difficulty
      );
  }

  onMount(() => {
    const handleWindowClick = () => closeRaidActionMenu();
    window.addEventListener('click', handleWindowClick);
    return () => window.removeEventListener('click', handleWindowClick);
  });

  async function undoDashboardRaidGate(raid: any, event?: MouseEvent) {
    event?.preventDefault();
    const gateToUndo = getLastCompletedGate(completionStatus, raid.content_id, raid.difficulty);
    if (!gateToUndo) return;

    await updateTodoRaidGateStatus(character.char_id, raid.content_id, gateToUndo, raid.content_id, false);
    window.dispatchEvent(new CustomEvent('raid-completed'));
    dispatchDashboardCompletionUpdate();
  }

  function dispatchDashboardCompletionUpdate() {
    window.dispatchEvent(new CustomEvent('character-data-complete'));
  }

  
</script>

<div class="character-card"
     class:compact={viewMode === 'compact'}
     class:minimal-card={isMinimalCard}
     class:daily-only-minimal={isDailyOnlyMinimalCard}
     class:gold-earner={character.earns_gold}
     class:non-gold-earner={!character.earns_gold}
     data-dashboard-character-id={character.char_id}
     aria-label={`Select character ${character.char_name}`}>

  {#if viewMode === 'compact'}
    <div
      class="compact-main-row"
      class:no-raids={!hasCompactLabels}
      class:has-dailies={chaosConfigured || guardianConfigured}
      class:has-labels={hasCompactLabels}
      class:has-weeklies={compactWeeklyTasks.length > 0}
    >
      <button
        type="button"
        class="compact-identity identity-link"
        on:click={handleCharacterClick}
        title={`Open ${character.char_name} in To Do`}
      >
        <img
          src={getClassIconUrl(iconId)}
          alt={displayName}
          class="class-icon compact-class-icon"
          on:error={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
        />
        <span class="character-name compact-name">
          {character.char_name}
        </span>
      </button>

      <button
        type="button"
        class="compact-stats stats-link"
        on:click={handleCharacterClick}
        title={`Open ${character.char_name} in To Do`}
      >
        <span class="item-level">iLvl {formatItemLevel(character.item_level)}</span>
        <span class="combat-power">CP {formatCombatPower(character.combat_power)}</span>
      </button>

      {#if hasCompactLabels || chaosConfigured || guardianConfigured}
        <div class="compact-daily-icons" aria-label="Daily task status">
          {#if chaosConfigured}
            <button
              type="button"
              class="compact-daily-state"
              class:available={chaosAvailable}
              class:inactive={!chaosAvailable}
              title={chaosIconTitle}
              on:click={(event) => completeDashboardTask('chaos', chaosCompleted, event)}
              on:contextmenu={(event) => event.preventDefault()}
            >
              <span class="compact-daily-icon">
                <img src={getTaskIcon('chaos')} alt="Chaos" />
              </span>
              <span class="compact-daily-progress" aria-hidden="true">
                <span style="width: {chaosRested}%"></span>
              </span>
            </button>
          {:else}
            <span class="compact-daily-state placeholder" aria-hidden="true">
              <span class="compact-daily-icon"></span>
              <span class="compact-daily-progress"><span></span></span>
            </span>
          {/if}
          {#if guardianConfigured}
            <button
              type="button"
              class="compact-daily-state"
              class:available={guardianAvailable}
              class:inactive={!guardianAvailable}
              title={guardianIconTitle}
              on:click={(event) => completeDashboardTask('guardian', guardianCompleted, event)}
              on:contextmenu={(event) => event.preventDefault()}
            >
              <span class="compact-daily-icon">
                <img src={getTaskIcon('guardian')} alt="Guardian" />
              </span>
              <span class="compact-daily-progress" aria-hidden="true">
                <span style="width: {guardianRested}%"></span>
              </span>
            </button>
          {:else}
            <span class="compact-daily-state placeholder" aria-hidden="true">
              <span class="compact-daily-icon"></span>
              <span class="compact-daily-progress"><span></span></span>
            </span>
          {/if}
          {#if isMinimalCard && displayWeeklyTasks.length === 1}
            {@const weeklyTask = displayWeeklyTasks[0]}
            <button
              type="button"
              class="compact-daily-state weekly"
              class:inactive={weeklyTask.completed}
              title={`${weeklyTask.name}: ${weeklyTask.completed ? 'done' : 'open'}`}
              on:click={(event) => completeDashboardTask(weeklyTask.content_id, weeklyTask.completed, event)}
              on:contextmenu={(event) => event.preventDefault()}
            >
              <span class="compact-daily-icon">
                <img src={getTaskIcon(weeklyTask.content_id)} alt={weeklyTask.name} />
              </span>
            </button>
          {/if}
        </div>
      {/if}

      {#if compactWeeklyTasks.length > 0}
        <div class="compact-weekly-icons" aria-label="Weekly task status">
          {#each compactWeeklyTasks as task}
            <button
              type="button"
              class="compact-weekly-state"
              class:inactive={task.completed}
              title={`${task.name}: ${task.completed ? 'done' : 'open'}`}
              on:click={(event) => completeDashboardTask(task.content_id, task.completed, event)}
              on:contextmenu={(event) => event.preventDefault()}
            >
              <img src={getTaskIcon(task.content_id)} alt={task.name} />
            </button>
          {/each}
        </div>
      {/if}

      {#if !isMinimalCard && (displayRaids.length > 0 || displayWeeklyTasks.length > 0)}
        <div class="compact-raid-row" class:weekly-only={displayRaids.length === 0 && displayWeeklyTasks.length > 0}>
          {#each displayRaids as raid}
            <div
              class="raid-item compact-raid"
              class:completed={raid.completed}
              class:gold-raid={raid.isGoldRaid}
              class:tracked-raid={raid.isTrackedRaid}
              class:static-reserved={raid.isStaticReserved}
              class:mismatch={raid.completionMismatch}
              title={raid.completionTooltip ?? (raid.isStaticReserved ? `Reserved for ${raid.staticBadgeText}` : '')}
              role="button"
              tabindex="0"
              on:click={(event) => completeDashboardRaidGate(raid, event)}
              on:contextmenu={(event) => openRaidActionMenu(raid, event)}
              on:keydown={(event) => event.key === 'Enter' && completeDashboardRaidGate(raid)}
            >
              <div class="raid-content">
                <img src={raidIcon} alt="Raid" class="raid-icon">
                <span class="raid-name compact-raid-name">
                  <span>{getRaidName(raid.content_id, raid.difficulty)}</span>
                  <span class="compact-raid-difficulty">{normalizeDifficulty(raid.difficulty)}</span>
                </span>
                {#if raid.gateProgress.total > 0}
                  <span
                    class="gate-progress"
                    class:gate-progress-done={raid.completed}
                    class:gate-progress-partial={!raid.completed && raid.gateProgress.completed > 0}
                  >
                    {raid.gateProgress.completed}/{raid.gateProgress.total}
                  </span>
                {/if}
                {#if raid.isGoldRaid}
                  <img src={goldIcon} alt="Gold" class="gold-icon">
                {/if}
                {#if raid.isStaticReserved}
                  <span class="static-badge">{raid.staticBadgeText}</span>
                {/if}
                {#if getRaidAssignment(raid)}
                  <span class="planned-badge">Plan</span>
                {/if}
                {#if getRaidReservation(raid)}
                  <span class="calendar-reservation-badge">
                    {getRaidReservation(raid)?.recurringWeekly ? getRaidReservation(raid)?.label : 'Reserved'}
                  </span>
                {/if}
              </div>
            </div>
          {/each}
          {#each displayWeeklyTasks as task}
            <div
              class="raid-item compact-raid weekly-task"
              class:completed={task.completed}
              title={`${task.name}: ${task.completed ? 'done' : 'open'}`}
              role="button"
              tabindex="0"
              on:click={(event) => completeDashboardTask(task.content_id, task.completed, event)}
              on:contextmenu={(event) => event.preventDefault()}
              on:keydown={(event) => event.key === 'Enter' && completeDashboardTask(task.content_id, task.completed)}
            >
              <div class="raid-content">
                <img src={getTaskIcon(task.content_id)} alt={task.name} class="raid-icon">
                <span class="raid-name compact-raid-name">
                  <span>{task.name}</span>
                  <span class="compact-raid-difficulty">Weekly</span>
                </span>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {:else}
    <!-- Interactive Header -->
    <div class="card-header">
      <div class="character-info">
        <button
          type="button"
          class="class-section identity-link"
          on:click={handleCharacterClick}
          title={`Open ${character.char_name} in To Do`}
        >
          <img
            src={getClassIconUrl(iconId)}
            alt={displayName}
            class="class-icon"
            on:error={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
          />
          <div class="character-details">
            <span class="character-name">
              {character.char_name}
            </span>
            <div class="character-stats">
              <span class="item-level">iLvl {formatItemLevel(character.item_level)}</span>
              <span class="combat-power">CP {formatCombatPower(character.combat_power)}</span>
            </div>
          </div>
        </button>
            </div>
    </div>

    <!-- Activity Section (Dailies & Events) -->
    <div class="activity-section" class:fixed-weekly-slots={hasFixedCardTopWeeklySlots}>
      {#if hasFixedCardTopWeeklySlots}
        {#each cardTopWeeklySlots as weeklyTask}
          {#if weeklyTask}
            <div
              class="activity-item weekly-inline"
              class:inactive={weeklyTask.completed}
              title={`${weeklyTask.name}: ${weeklyTask.completed ? 'done' : 'open'}`}
              role="button"
              tabindex="0"
              on:click={(event) => completeDashboardTask(weeklyTask.content_id, weeklyTask.completed, event)}
              on:contextmenu={(event) => event.preventDefault()}
              on:keydown={(event) => event.key === 'Enter' && completeDashboardTask(weeklyTask.content_id, weeklyTask.completed)}
            >
              <div class="activity-icon">
                <img src={getTaskIcon(weeklyTask.content_id)} alt={weeklyTask.name} class="task-icon" />
              </div>
            </div>
          {:else}
            <span class="activity-item weekly-inline placeholder" aria-hidden="true">
              <span class="activity-icon"></span>
            </span>
          {/if}
        {/each}
      {/if}
      {#each looseCardTopWeeklyTasks as weeklyTask}
        <div
          class="activity-item weekly-inline"
          class:inactive={weeklyTask.completed}
          title={`${weeklyTask.name}: ${weeklyTask.completed ? 'done' : 'open'}`}
          role="button"
          tabindex="0"
          on:click={(event) => completeDashboardTask(weeklyTask.content_id, weeklyTask.completed, event)}
          on:contextmenu={(event) => event.preventDefault()}
          on:keydown={(event) => event.key === 'Enter' && completeDashboardTask(weeklyTask.content_id, weeklyTask.completed)}
        >
          <div class="activity-icon">
            <img src={getTaskIcon(weeklyTask.content_id)} alt={weeklyTask.name} class="task-icon" />
          </div>
        </div>
      {/each}
      {#if chaosConfigured}
        <div
          class="activity-item"
          class:inactive={!chaosAvailable}
          title={chaosIconTitle}
          role="button"
          tabindex="0"
          on:click={(event) => completeDashboardTask('chaos', chaosCompleted, event)}
          on:contextmenu={(event) => event.preventDefault()}
          on:keydown={(event) => event.key === 'Enter' && completeDashboardTask('chaos', chaosCompleted)}
        >
          <div class="activity-icon">
            <img src={getTaskIcon('chaos')} alt="Chaos" class="task-icon" />
          </div>
          <div class="rested-progress">
            <div class="rested-bar">
              <div class="rested-fill" style="width: {chaosRested}%"></div>
            </div>
            <span class="rested-value">{chaosRested}%</span>
          </div>
        </div>
      {/if}
      {#if guardianConfigured}
        <div
          class="activity-item"
          class:inactive={!guardianAvailable}
          title={guardianIconTitle}
          role="button"
          tabindex="0"
          on:click={(event) => completeDashboardTask('guardian', guardianCompleted, event)}
          on:contextmenu={(event) => event.preventDefault()}
          on:keydown={(event) => event.key === 'Enter' && completeDashboardTask('guardian', guardianCompleted)}
        >
          <div class="activity-icon">
            <img src={getTaskIcon('guardian')} alt="Guardian" class="task-icon" />
          </div>
          <div class="rested-progress">
            <div class="rested-bar">
              <div class="rested-fill" style="width: {guardianRested}%"></div>
            </div>
            <span class="rested-value">{guardianRested}%</span>
          </div>
        </div>
      {/if}
    </div>

    <!-- Gold Raid Section -->
    {#if !isMinimalCard && (displayRaids.length > 0 || cardBodyWeeklyTasks.length > 0)}
      <div class="raid-section">
        <div class="raid-list">
          {#each displayRaids as raid}
            <div
              class="raid-item"
              class:completed={raid.completed}
              class:gold-raid={raid.isGoldRaid}
              class:tracked-raid={raid.isTrackedRaid}
              class:static-reserved={raid.isStaticReserved}
              class:mismatch={raid.completionMismatch}
              title={raid.completionTooltip ?? (raid.isStaticReserved ? `Reserved for ${raid.staticBadgeText}` : '')}
              role="button"
              tabindex="0"
              on:click={(event) => completeDashboardRaidGate(raid, event)}
              on:contextmenu={(event) => openRaidActionMenu(raid, event)}
              on:keydown={(event) => event.key === 'Enter' && completeDashboardRaidGate(raid)}
            >
              <div class="raid-content">
                <img src={raidIcon} alt="Raid" class="raid-icon">
                <span class="raid-name">{getRaidDisplayName(raid.content_id, raid.difficulty)}</span>
                {#if raid.gateProgress.total > 0}
                  <span
                    class="gate-progress"
                    class:gate-progress-done={raid.completed}
                    class:gate-progress-partial={!raid.completed && raid.gateProgress.completed > 0}
                  >
                    {raid.gateProgress.completed}/{raid.gateProgress.total}
                  </span>
                {/if}
                {#if raid.isGoldRaid}
                  <img src={goldIcon} alt="Gold" class="gold-icon">
                {/if}
                {#if raid.isStaticReserved}
                  <span class="static-badge">{raid.staticBadgeText}</span>
                {/if}
                {#if getRaidAssignment(raid)}
                  <span class="planned-badge">Plan</span>
                {/if}
                {#if getRaidReservation(raid)}
                  <span class="calendar-reservation-badge">
                    {getRaidReservation(raid)?.recurringWeekly ? getRaidReservation(raid)?.label : 'Reserved'}
                  </span>
                {/if}
              </div>
            </div>
          {/each}
          {#each cardBodyWeeklyTasks as task}
            <div
              class="raid-item weekly-task"
              class:completed={task.completed}
              title={`${task.name}: ${task.completed ? 'done' : 'open'}`}
              role="button"
              tabindex="0"
              on:click={(event) => completeDashboardTask(task.content_id, task.completed, event)}
              on:contextmenu={(event) => event.preventDefault()}
              on:keydown={(event) => event.key === 'Enter' && completeDashboardTask(task.content_id, task.completed)}
            >
              <div class="raid-content">
                <img src={getTaskIcon(task.content_id)} alt={task.name} class="raid-icon">
                <span class="raid-name">{task.name}</span>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}

  {#if raidActionMenu}
    <div
      class="raid-action-menu"
      style={`left: ${raidActionMenu.x}px; top: ${raidActionMenu.y}px;`}
      role="menu"
      tabindex="-1"
      on:click|stopPropagation
      on:keydown|stopPropagation
    >
      <strong>{getRaidDisplayName(raidActionMenu.raid.content_id, raidActionMenu.raid.difficulty)}</strong>
      <span class="raid-action-hint">Assign or reserve {character.char_name}</span>

      {#if calendarEvents.length > 0}
        <div class="raid-action-group">
          <small>Existing planned MeowGang raid</small>
          {#each calendarEvents as event}
            <button type="button" on:click={() => assignRaidToEvent(event)}>
              <span>{event.sectionLabel || event.raidName}</span>
              <small>{event.startsAtLabel}</small>
            </button>
          {/each}
        </div>
      {:else}
        <p>No active planned raids found for your Discord account.</p>
      {/if}

      <div class="raid-action-group">
        <small>Local reservation</small>
        <button type="button" on:click={openReserveRaidOncePicker}>Reserve date/time</button>
        {#if reservationPickerOpen}
          <div class="reservation-picker">
            <label>
              <span>Day</span>
              <input type="date" bind:value={reservationDate} />
            </label>
            <label>
              <span>Server time</span>
              <input type="time" bind:value={reservationTime} step="300" />
            </label>
            <button type="button" on:click={reserveRaidOnce}>Save reservation</button>
          </div>
        {/if}
        <button type="button" on:click={reserveRaidWeekly}>Reserve weekly/static</button>
        <button type="button" class="danger" on:click={clearRaidReservation}>Clear reservation</button>
      </div>
    </div>
  {/if}

  {#if clearReservationDialogOpen}
    <div class="clear-reservation-dialog-overlay" on:click={() => clearReservationDialogOpen = false} on:keydown={(e) => e.key === 'Escape' && clearReservationDialogOpen = false}>
      <div class="clear-reservation-dialog" on:click|stopPropagation role="dialog" aria-modal="true" aria-labelledby="dialog-title" tabindex="-1">
        <header>
          <strong id="dialog-title">Clear Reservations</strong>
          <button type="button" on:click={() => clearReservationDialogOpen = false} aria-label="Close dialog">✕</button>
        </header>
        <p>Select reservations to clear for {character.char_name}:</p>
        <div class="reservations-list">
          {#if allReservations.length === 0}
            <p class="empty">No reservations found.</p>
          {:else}
            {#each allReservations as reservation}
              <label class="reservation-item">
                <input
                  type="checkbox"
                  checked={reservationsToClear.has(getReservationKeyForCombined(reservation))}
                  on:change={() => toggleReservationToClear(getReservationKeyForCombined(reservation))}
                />
                <span>
                  <strong>{reservation.isAssignment ? 'Planned signup' : getRaidName(reservation.contentId, reservation.difficulty)}</strong>
                  {#if !reservation.isAssignment}
                    <small>{reservation.difficulty}{reservation.scheduledAt ? ` - ${new Date(reservation.scheduledAt).toLocaleDateString()}` : ''}{reservation.recurringWeekly ? ' (weekly)' : ''}</small>
                  {:else}
                    <small>{reservation.charName} - {reservation.eventKey}</small>
                  {/if}
                </span>
              </label>
            {/each}
          {/if}
        </div>
        <footer>
          <button type="button" on:click={() => clearReservationDialogOpen = false}>Cancel</button>
          <button
            type="button"
            class="danger"
            on:click={clearSelectedReservations}
            disabled={reservationsToClear.size === 0}
          >
            Clear {reservationsToClear.size} reservation{reservationsToClear.size === 1 ? '' : 's'}
          </button>
        </footer>
      </div>
    </div>
  {/if}
</div>

  
  
<style>
  .character-card {
    box-sizing: border-box;
    background: var(--surface-variant);
    border-radius: 12px;
    padding: 0.8rem;
    box-shadow: var(--app-shadow-sm);
    position: relative;
    overflow: hidden;
    border: 2px solid transparent;
    min-height: 124px;
    display: flex;
    flex-direction: column;
  }

  .character-card:global(.dashboard-focus-highlight) {
    outline: 2px solid var(--app-color-guide-highlight, #00e5ff);
    box-shadow:
      var(--app-shadow-highlight),
      0 0 0 4px color-mix(in srgb, var(--app-color-guide-highlight, #00e5ff) 20%, transparent) !important;
  }

  .character-card.gold-earner {
    border-color: var(--app-dashboard-card-gold-border);
    box-shadow: var(--app-dashboard-card-gold-shadow);
  }

  .character-card.non-gold-earner {
    border-color: var(--app-dashboard-card-tracked-border);
    box-shadow: var(--app-dashboard-card-tracked-shadow);
  }

  .character-card.compact {
    min-height: 0;
    padding: 0.48rem 0.65rem;
    gap: 0.35rem;
    border-width: 1px;
    container-type: inline-size;
  }

  .character-card.compact:not(.minimal-card) {
    grid-column: 1 / -1;
  }

  .character-card.compact.minimal-card {
    min-width: 0;
    padding: 0.42rem 0.55rem;
  }

  .character-card.minimal-card:not(.compact) {
    min-height: 54px;
    padding: 0.34rem 0.55rem;
  }

  .compact-main-row {
    display: grid;
    grid-template-columns: minmax(8.5rem, 0.45fr) minmax(8.6rem, max-content) 7rem minmax(0, 2.75fr);
    gap: clamp(0.2rem, 0.55vw, 0.45rem);
    align-items: center;
    min-width: 0;
  }

  .character-card.compact.minimal-card .compact-main-row,
  .character-card.compact.minimal-card .compact-main-row.has-dailies,
  .character-card.compact.minimal-card .compact-main-row.no-raids,
  .character-card.compact.minimal-card .compact-main-row.no-raids.has-dailies {
    grid-template-columns: minmax(0, 1fr) max-content;
    justify-content: stretch;
  }

  .character-card.compact.minimal-card.non-gold-earner .compact-main-row,
  .character-card.compact.minimal-card.non-gold-earner .compact-main-row.has-dailies,
  .character-card.compact.minimal-card.non-gold-earner .compact-main-row.no-raids,
  .character-card.compact.minimal-card.non-gold-earner .compact-main-row.no-raids.has-dailies {
    grid-template-columns: minmax(0, 1fr) max-content max-content;
  }

  .character-card.compact.minimal-card .compact-stats {
    display: none;
  }

  .character-card.compact.minimal-card.non-gold-earner .compact-stats {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    justify-self: end;
    gap: 0.05rem;
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }

  .character-card.compact.minimal-card.non-gold-earner .compact-stats .item-level,
  .character-card.compact.minimal-card.non-gold-earner .compact-stats .combat-power {
    font-size: 0.58rem;
    line-height: 1.05;
  }

  .character-card.compact.minimal-card .compact-daily-icons {
    width: auto;
  }

  .character-card.compact.daily-only-minimal .compact-daily-icons {
    gap: 0.4rem;
  }

  .character-card.compact.minimal-card .compact-raid-row {
    grid-column: 1 / -1;
    grid-template-columns: 1fr;
  }

  .compact-main-row.has-dailies {
    grid-template-columns: minmax(8.5rem, 0.45fr) minmax(8.6rem, max-content) 7rem minmax(0, 2.75fr);
  }

  .compact-main-row.has-weeklies,
  .compact-main-row.has-weeklies.has-dailies {
    grid-template-columns: minmax(8.5rem, 0.45fr) minmax(8.6rem, max-content) 7rem max-content minmax(0, 2.35fr);
  }

  .compact-main-row.no-raids {
    grid-template-columns: minmax(32px, max-content) max-content;
    justify-content: start;
  }

  .compact-main-row.no-raids.has-dailies {
    grid-template-columns: minmax(32px, max-content) max-content 3.25rem;
    justify-content: start;
  }

  .compact-identity,
  .compact-stats,
  .compact-raid-row {
    min-width: 0;
  }

  .compact-identity {
    display: flex;
    align-items: center;
    gap: clamp(0.3rem, 0.65vw, 0.55rem);
  }

  .compact-class-icon {
    flex-shrink: 0;
  }

  .compact-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .compact-stats {
    display: flex;
    gap: clamp(0.3rem, 0.65vw, 0.6rem);
    align-items: center;
    justify-self: start;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .identity-link,
  .stats-link {
    appearance: none;
    border: none;
    background: transparent;
    color: inherit;
    cursor: pointer;
    font-family: inherit;
    padding: 0;
    text-align: left;
  }

  .identity-link:hover .character-name {
    color: var(--primary);
  }

  .character-card.compact:not(.minimal-card) .compact-stats .item-level {
    width: 4.9rem;
  }

  .character-card.compact:not(.minimal-card) .compact-stats .combat-power {
    width: 4.75rem;
  }

  .compact-daily-icons {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.28rem;
    flex-shrink: 0;
    width: 3.25rem;
  }

  .character-card.compact:not(.minimal-card) .compact-daily-icons {
    justify-content: flex-start;
    width: 7rem;
  }

  .compact-daily-state {
    appearance: none;
    border: none;
    background: transparent;
    color: inherit;
    cursor: pointer;
    font-family: inherit;
    padding: 0;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    min-width: 0;
  }

  .compact-daily-state.inactive {
    opacity: 0.34;
    filter: grayscale(0.95);
  }

  .compact-daily-state.available {
    opacity: 1;
    filter: none;
  }

  .compact-daily-state.weekly .compact-daily-icon {
    border-color: color-mix(in srgb, var(--app-color-muted-state) 28%, transparent);
  }

  .compact-daily-state.placeholder {
    visibility: hidden;
  }

  .character-card.compact.daily-only-minimal .compact-daily-state:not(.placeholder) {
    min-width: 58px;
  }

  .character-card.compact:not(.minimal-card) .compact-daily-state:not(.placeholder) {
    min-width: 3.25rem;
  }

  .compact-daily-icon {
    width: 22px;
    height: 22px;
    border-radius: 5px;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--surface) 86%, black);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-on-surface) 8%, transparent);
    overflow: hidden;
  }

  .compact-daily-icon img {
    width: 16px;
    height: 16px;
    object-fit: contain;
    border-radius: 2px;
  }

  .compact-daily-progress {
    display: none;
    width: 32px;
    height: 4px;
    overflow: hidden;
    border-radius: 999px;
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 10%, transparent);
  }

  .compact-daily-progress span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, var(--app-color-tracked), var(--app-color-tracked-alt));
  }

  .character-card.compact.daily-only-minimal .compact-daily-progress {
    display: block;
  }

  .character-card.compact:not(.minimal-card) .compact-daily-progress {
    display: block;
  }

  .compact-weekly-icons {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.22rem;
    min-width: 0;
    white-space: nowrap;
  }

  .compact-weekly-state {
    appearance: none;
    color: inherit;
    cursor: pointer;
    padding: 0;
    width: 22px;
    height: 22px;
    border-radius: 5px;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--surface) 86%, black);
    border: 1px solid color-mix(in srgb, var(--app-color-muted-state) 22%, transparent);
    overflow: hidden;
  }

  .compact-weekly-state.inactive {
    opacity: 0.34;
    filter: grayscale(0.95);
  }

  .compact-weekly-state img {
    width: 16px;
    height: 16px;
    object-fit: contain;
    border-radius: 2px;
  }

  .compact-raid-row {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: clamp(0.25rem, 0.6vw, 0.4rem);
    min-width: 0;
  }

  .compact-raid-row.weekly-only {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .compact-raid {
    min-width: 0;
  }

  .compact-raid-name {
    display: flex;
    gap: 0.25rem;
    align-items: center;
    min-width: 0;
  }

  .compact-raid-difficulty {
    color: currentColor;
    opacity: 0.72;
    flex-shrink: 0;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 0.5rem;
    padding-right: 8rem;
  }

  .character-card.minimal-card:not(.compact) .card-header {
    min-width: 0;
    margin-bottom: 0;
    padding-right: 8rem;
  }

  .character-info {
    flex: 1;
    min-width: 0;
  }

  .class-section {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
    width: 100%;
  }

  .class-icon {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    box-shadow: var(--app-shadow-sm);
  }

  .character-card.minimal-card:not(.compact) .class-icon {
    width: 28px;
    height: 28px;
    border-radius: 7px;
  }

  .character-details {
    flex: 1;
    min-width: 0;
  }

  .character-name {
    display: block;
    margin: 0 0 0.25rem 0;
    color: var(--on-surface);
    font-size: 0.9rem;
    font-weight: 600;
    line-height: 1.2;
  }

  .character-card.minimal-card:not(.compact) .character-name {
    margin-bottom: 0.1rem;
  }

  .character-stats {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }

  .item-level {
    font-weight: 500;
    color: color-mix(in srgb, var(--md-sys-color-on-surface) 70%, transparent);
    font-size: 0.75rem;
  }

  .combat-power {
    font-weight: 500;
    color: var(--app-color-accent-muted);
    font-size: 0.75rem;
  }

  .activity-section {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }

  .character-card:not(.compact) .activity-section {
    position: absolute;
    top: 0.5rem;
    right: 0.55rem;
    z-index: 2;
    display: grid;
    grid-auto-flow: column;
    grid-template-rows: repeat(2, auto);
    align-items: center;
    justify-items: end;
    gap: 0.22rem 0.3rem;
  }

  .character-card.minimal-card:not(.compact) .activity-section {
    top: 0.34rem;
    right: 0.45rem;
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.24rem;
  }

  .character-card.minimal-card:not(.compact) .activity-section.fixed-weekly-slots {
    top: 0.5rem;
    right: 0.55rem;
    display: grid;
    grid-auto-flow: column;
    grid-template-rows: repeat(2, auto);
    align-items: center;
    justify-items: end;
    gap: 0.22rem 0.3rem;
  }

  .activity-item {
    border: none;
    background: transparent;
    color: inherit;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.25rem;
    flex: 1;
  }

  .character-card:not(.compact) .activity-item {
    flex: 0 0 auto;
    gap: 0.18rem;
  }

  .character-card:not(.compact) .rested-progress {
    display: flex;
    flex: 0 0 42px;
    width: 42px;
    gap: 0;
  }

  .character-card:not(.compact) .rested-bar {
    height: 3px;
  }

  .character-card:not(.compact) .rested-value {
    display: none;
  }

  .character-card.minimal-card:not(.compact) .activity-item {
    flex: 0 0 auto;
    gap: 0.18rem;
  }

  .character-card.minimal-card:not(.compact) .rested-progress {
    flex-basis: 28px;
    width: 28px;
  }

  .activity-item.weekly-inline .activity-icon {
    border: 1px solid color-mix(in srgb, var(--app-color-muted-state) 28%, transparent);
  }

  .activity-item.placeholder {
    visibility: hidden;
    pointer-events: none;
  }

  .activity-item.inactive {
    opacity: 0.4;
    filter: grayscale(0.8);
  }

  .activity-icon {
    width: 24px;
    height: 24px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--surface);
    transition: all 0.2s ease;
  }

  .character-card:not(.compact) .activity-icon {
    width: 22px;
    height: 22px;
  }

  .character-card.minimal-card:not(.compact) .activity-icon {
    width: 22px;
    height: 22px;
  }

  .rested-progress {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    flex: 1;
  }

  .rested-bar {
    flex: 1;
    height: 4px;
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 10%, transparent);
    border-radius: 2px;
    overflow: hidden;
  }

  .rested-fill {
    height: 100%;
    background: var(--app-color-success-gradient);
    border-radius: 2px;
    transition: width 0.3s ease;
  }

  .rested-value {
    font-size: 0.625rem;
    color: var(--on-surface-variant);
    font-weight: 500;
    min-width: 28px;
    text-align: right;
  }

  .task-icon {
    width: 16px;
    height: 16px;
    border-radius: 2px;
  }

  .raid-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .raid-list {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }

  .raid-item {
    padding: 0.2rem 0.45rem;
    background: var(--surface);
    border-radius: 4px;
    font-size: 0.75rem;
    color: var(--on-surface-variant);
    font-weight: 500;
    transition: all 0.2s ease;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .raid-item.gold-raid {
    background: var(--app-color-raid-gold-surface);
    border: 1px solid color-mix(in srgb, var(--app-color-gold) 30%, transparent);
    color: var(--app-color-gold);
  }

  .raid-item.tracked-raid {
    background: var(--app-color-raid-tracked-surface);
    border: 1px solid color-mix(in srgb, var(--app-color-tracked) 28%, transparent);
    color: var(--app-color-on-tracked);
  }

  .raid-item.static-reserved {
    border-color: color-mix(in srgb, var(--app-color-static) 45%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--app-color-static) 16%, transparent);
  }

  .raid-item.weekly-task {
    background: var(--app-color-raid-weekly-surface);
    border: 1px solid color-mix(in srgb, var(--app-color-muted-state) 24%, transparent);
    color: color-mix(in srgb, var(--app-color-muted-state) 64%, var(--md-sys-color-on-surface));
  }

  .raid-item.completed {
    opacity: 0.5;
    text-decoration: line-through;
    cursor: default;
  }

  .raid-item.gold-raid.completed {
    opacity: 0.4;
    text-decoration: line-through;
    color: color-mix(in srgb, var(--app-color-gold) 60%, transparent);
  }

  .raid-item.tracked-raid.completed {
    opacity: 0.42;
    text-decoration: line-through;
    color: color-mix(in srgb, var(--app-color-on-tracked) 60%, transparent);
  }

  .raid-item.weekly-task.completed {
    opacity: 0.42;
    text-decoration: line-through;
    color: color-mix(in srgb, var(--app-color-muted-state) 62%, var(--md-sys-color-on-surface));
  }

  .raid-item.mismatch {
    opacity: 0.9;
    background: color-mix(in srgb, var(--md-sys-color-error) 12%, transparent);
    color: var(--md-sys-color-error);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-error) 35%, transparent);
    text-decoration: line-through;
    text-decoration-color: var(--md-sys-color-error);
  }

  .raid-item.mismatch .raid-name {
    color: inherit;
  }

  .raid-content {
    display: flex;   /* was missing! */
    align-items: center;
    gap: 0.25rem;
    flex: 1;
    min-width: 0;
  }

  .gate-progress {
    font-size: 0.65rem;
    font-weight: 700;
    padding: 0 0.25rem;
    border-radius: 3px;
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, transparent);
    color: color-mix(in srgb, var(--md-sys-color-on-surface) 55%, transparent);
    white-space: nowrap;
    flex-shrink: 0;
  }
  .gate-progress-partial { background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, transparent); color: color-mix(in srgb, var(--md-sys-color-on-surface) 55%, transparent); }
  .gate-progress-done    { background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, transparent); color: color-mix(in srgb, var(--md-sys-color-on-surface) 55%, transparent); }

  .raid-icon {
    width: 14px;
    height: 14px;
    border-radius: 2px;
    flex-shrink: 0;
  }

  .raid-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .gold-icon {
    width: 12px;
    height: 12px;
    border-radius: 2px;
    flex-shrink: 0;
  }

  .static-badge {
    flex-shrink: 0;
    border-radius: 3px;
    padding: 0.05rem 0.25rem;
    background: color-mix(in srgb, var(--app-color-static) 18%, transparent);
    color: var(--app-color-on-static);
    font-size: 0.58rem;
    font-weight: 800;
    text-transform: uppercase;
  }

  .planned-badge,
  .calendar-reservation-badge {
    flex-shrink: 0;
    border-radius: 3px;
    padding: 0.05rem 0.25rem;
    font-size: 0.56rem;
    font-weight: 800;
    line-height: 1.15;
    text-transform: uppercase;
    text-decoration: none;
  }

  .planned-badge {
    background: color-mix(in srgb, var(--md-sys-color-primary) 18%, transparent);
    color: var(--md-sys-color-primary);
  }

  .calendar-reservation-badge {
    background: color-mix(in srgb, var(--app-color-static) 16%, transparent);
    color: var(--app-color-on-static);
  }

  .raid-action-menu {
    position: fixed;
    z-index: 80;
    width: min(260px, calc(100vw - 1rem));
    max-height: min(390px, calc(100vh - 1rem));
    overflow: auto;
    border-radius: 8px;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-primary) 34%, transparent);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
    box-shadow: var(--app-shadow-md);
    padding: 0.6rem;
    display: grid;
    gap: 0.45rem;
    text-decoration: none;
  }

  .raid-action-menu > strong {
    font-size: 0.74rem;
    line-height: 1.2;
  }

  .raid-action-hint,
  .raid-action-menu p,
  .raid-action-group small,
  .raid-action-group button small {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.64rem;
    line-height: 1.25;
  }

  .raid-action-menu p {
    margin: 0;
  }

  .raid-action-group {
    display: grid;
    gap: 0.25rem;
  }

  .raid-action-group button {
    appearance: none;
    width: 100%;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-outline) 65%, transparent);
    background: color-mix(in srgb, var(--md-sys-color-surface) 82%, transparent);
    color: inherit;
    border-radius: 5px;
    padding: 0.35rem 0.45rem;
    cursor: pointer;
    font: inherit;
    font-size: 0.68rem;
    text-align: left;
    display: grid;
    gap: 0.08rem;
  }

  .raid-action-group button:hover {
    border-color: var(--md-sys-color-primary);
  }

  .raid-action-group button.danger {
    color: var(--md-sys-color-error);
  }

  .reservation-picker {
    display: grid;
    grid-template-columns: 1fr 0.8fr;
    gap: 0.35rem;
    padding: 0.4rem;
    border-radius: 6px;
    background: color-mix(in srgb, var(--md-sys-color-surface) 70%, transparent);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-outline) 42%, transparent);
  }

  .reservation-picker label {
    display: grid;
    gap: 0.18rem;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.62rem;
  }

  .reservation-picker input {
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
    border-radius: 5px;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-outline) 65%, transparent);
    background: var(--md-sys-color-surface-container);
    color: var(--md-sys-color-on-surface);
    font: inherit;
    font-size: 0.68rem;
    padding: 0.25rem;
  }

  .reservation-picker button {
    grid-column: 1 / -1;
  }

  .compact-raid-name > span:first-child {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 1360px) {
    .compact-main-row {
      grid-template-columns: minmax(7.75rem, 0.32fr) minmax(8.25rem, max-content) 7rem minmax(0, 2.85fr);
    }

    .compact-main-row.has-dailies {
      grid-template-columns: minmax(7.75rem, 0.32fr) minmax(8.25rem, max-content) 7rem minmax(0, 2.85fr);
    }

    .compact-main-row.has-weeklies,
    .compact-main-row.has-weeklies.has-dailies {
      grid-template-columns: minmax(7.75rem, 0.32fr) minmax(8.25rem, max-content) 7rem max-content minmax(0, 2.45fr);
    }

    .compact-main-row.no-raids {
      grid-template-columns: minmax(32px, max-content) max-content;
      justify-content: start;
    }

    .compact-main-row.no-raids.has-dailies {
      grid-template-columns: minmax(32px, max-content) max-content 3.25rem;
      justify-content: start;
    }

    .compact-raid-row {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }

  @media (max-width: 1120px) {
    .compact-main-row {
      grid-template-columns: minmax(7rem, 0.22fr) minmax(7.8rem, max-content) 3.25rem minmax(0, 2.8fr);
    }

    .compact-main-row.has-dailies {
      grid-template-columns: minmax(7rem, 0.22fr) minmax(7.8rem, max-content) 3.25rem minmax(0, 2.8fr);
    }

    .compact-main-row.has-weeklies,
    .compact-main-row.has-weeklies.has-dailies {
      grid-template-columns: minmax(7rem, 0.22fr) minmax(7.8rem, max-content) 3.25rem max-content minmax(0, 2.35fr);
    }

    .character-card.compact:not(.minimal-card) .compact-daily-icons {
      justify-content: center;
      width: 3.25rem;
    }

    .character-card.compact:not(.minimal-card) .compact-daily-state:not(.placeholder) {
      min-width: 0;
    }

    .character-card.compact:not(.minimal-card) .compact-daily-progress {
      display: none;
    }

    .compact-main-row.no-raids {
      grid-template-columns: minmax(32px, max-content) max-content;
      justify-content: start;
    }

    .compact-main-row.no-raids.has-dailies {
      grid-template-columns: minmax(32px, max-content) max-content 3.25rem;
      justify-content: start;
    }

    .compact-raid-row {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .character-card.compact {
      padding: 0.45rem 0.6rem;
    }

    .compact .raid-item {
      font-size: 0.7rem;
      padding-inline: 0.4rem;
    }

    .compact .character-name {
      font-size: 0.85rem;
    }
  }

  @media (max-width: 980px) {
    .compact-main-row,
    .compact-main-row.no-raids {
      grid-template-columns: minmax(6.5rem, 0.18fr) minmax(7.5rem, max-content) 3.25rem minmax(0, 2.6fr);
    }

    .compact-main-row.has-dailies,
    .compact-main-row.no-raids.has-dailies {
      grid-template-columns: minmax(6.5rem, 0.18fr) minmax(7.5rem, max-content) 3.25rem minmax(0, 2.6fr);
    }

    .compact-main-row.has-weeklies,
    .compact-main-row.has-weeklies.has-dailies {
      grid-template-columns: minmax(6.5rem, 0.18fr) minmax(7.5rem, max-content) 3.25rem minmax(0, 2.6fr);
    }

    .compact-weekly-icons {
      display: none;
    }

    .compact-main-row.no-raids {
      grid-template-columns: minmax(32px, max-content) max-content;
      justify-content: start;
    }

    .compact-main-row.no-raids.has-dailies {
      grid-template-columns: minmax(32px, max-content) max-content 3.25rem;
      justify-content: start;
    }
  }

  @media (max-width: 900px) {
    .compact-raid-difficulty {
      display: none;
    }
  }

  @container (max-width: 560px) {
    .compact-main-row,
    .compact-main-row.no-raids {
      grid-template-columns: minmax(0, 1fr) minmax(7.75rem, max-content) 3.25rem;
    }

    .compact-main-row.has-dailies,
    .compact-main-row.no-raids.has-dailies {
      grid-template-columns: minmax(0, 1fr) minmax(7.75rem, max-content) 3.25rem;
    }

    .compact-main-row.no-raids:not(.has-dailies) {
      grid-template-columns: minmax(0, max-content) max-content;
      justify-content: start;
    }

    .compact-main-row.has-labels,
    .compact-main-row.has-labels.has-dailies {
      grid-template-columns: minmax(0, 1fr) minmax(7.75rem, max-content) 3.25rem;
    }

    .compact-raid-row {
      grid-column: 1 / -1;
    }

    .compact-raid-row.weekly-only {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @container (max-width: 460px) {
    .compact-main-row,
    .compact-main-row.no-raids {
      grid-template-columns: 1fr;
    }

    .compact-stats {
      justify-self: start;
    }

    .compact-raid-row {
      grid-template-columns: 1fr;
    }

    .compact-raid-row.weekly-only {
      grid-template-columns: 1fr;
    }
  }

  @container (max-width: 230px) {
    .character-card.compact.daily-only-minimal .compact-daily-state:not(.placeholder) {
      min-width: auto;
    }

    .character-card.compact.daily-only-minimal .compact-daily-progress {
      display: none;
    }
  }

  @media (max-width: 768px) {
    .character-card {
      padding: 0.65rem;
      min-height: 108px;
    }

    .class-icon {
      width: 28px;
      height: 28px;
    }

    .character-name {
      font-size: 0.85rem;
    }

    .raid-item {
      font-size: 0.7rem;
      padding: 0.2rem 0.4rem;
    }

    .compact-raid-row {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }

  .clear-reservation-dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 1rem;
  }

  .clear-reservation-dialog {
    background: #1e1e1e;
    border-radius: 12px;
    padding: 1.5rem;
    max-width: 500px;
    width: 100%;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    box-shadow: var(--app-shadow-lg);
  }

  .clear-reservation-dialog header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .clear-reservation-dialog header button {
    background: transparent;
    border: none;
    color: var(--md-sys-color-on-surface);
    font-size: 1.5rem;
    cursor: pointer;
    padding: 0.25rem;
    line-height: 1;
  }

  .clear-reservation-dialog p {
    margin: 0;
    font-size: 0.9rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .reservations-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 300px;
    overflow-y: auto;
    padding-right: 0.5rem;
  }

  .reservations-list .empty {
    color: var(--md-sys-color-on-surface-variant);
    font-style: italic;
    text-align: center;
    padding: 1rem;
  }

  .reservation-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem;
    border-radius: 8px;
    background: color-mix(in srgb, var(--md-sys-color-surface-container-high) 50%, transparent);
    border: 1px solid color-mix(in srgb, var(--md-sys-color-outline) 30%, transparent);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .reservation-item:hover {
    background: color-mix(in srgb, var(--md-sys-color-surface-container-high) 70%, transparent);
  }

  .reservation-item input[type="checkbox"] {
    width: 1.2rem;
    height: 1.2rem;
    cursor: pointer;
  }

  .reservation-item span {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    flex: 1;
  }

  .reservation-item span strong {
    font-size: 0.9rem;
  }

  .reservation-item span small {
    font-size: 0.75rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .clear-reservation-dialog footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    padding-top: 0.5rem;
    border-top: 1px solid color-mix(in srgb, var(--md-sys-color-outline) 30%, transparent);
  }

  .clear-reservation-dialog footer button {
    padding: 0.5rem 1rem;
    border-radius: 6px;
    font-size: 0.85rem;
    cursor: pointer;
    border: 1px solid color-mix(in srgb, var(--md-sys-color-outline) 50%, transparent);
    background: var(--md-sys-color-surface-container);
    color: var(--md-sys-color-on-surface);
  }

  .clear-reservation-dialog footer button.danger {
    background: var(--md-sys-color-error);
    color: var(--md-sys-color-on-error);
    border-color: var(--md-sys-color-error);
  }

  .clear-reservation-dialog footer button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
