<script lang="ts">
  import { rosters, characters, activeRosterId } from '$lib/store';
  import { onMount } from 'svelte';
  import type { Character } from '$lib/store';
  import {
    getArgeosStatusKind as resolveArgeosStatusKind
  } from '$lib/components/dashboard/helpers';
  import DashboardRosterSections from '$lib/components/dashboard/DashboardRosterSections.svelte';
  import DashboardStatsBar from '$lib/components/dashboard/DashboardStatsBar.svelte';
  import WeeklyGoldProgressCard from '$lib/components/dashboard/WeeklyGoldProgressCard.svelte';
  import {
    getDashboardStaticBadgesPreference,
    getDashboardViewPreference,
    type DashboardViewMode
  } from '$lib/services/dashboard-preferences';
  import { buildDashboardStats } from '$lib/services/dashboard';
  import DashboardCalendarWidget from '$lib/components/dashboard/DashboardCalendarWidget.svelte';
  import {
    cleanupExpiredDashboardRaidReservations,
    getDashboardCalendarAssignments,
    getDashboardRaidReservations,
    loadDashboardCalendarAssignments,
    loadDashboardRaidReservations,
    loadUserDashboardCalendarEvents,
    type DashboardCalendarAssignment,
    type DashboardCalendarEvent,
    type DashboardRaidReservation
  } from '$lib/services/dashboard-calendar';
  import type {
    DashboardCharacterData,
    DashboardDailyDetail,
    DashboardRaidDetail,
    DashboardRosterEventDetail,
    DashboardWeeklyTaskDetail
  } from '$lib/components/dashboard/types';

  // Props for header communication
  export let setHeaderContent: (content: string) => void;
  export let discordId = '';

  // State
  let visibleCharacters: Character[] = [];
  let loading = true;
  let totalRaidsCompleted = 0;
  let totalAdditionalRaidsCompleted = 0;
  let totalDailiesCompleted = 0;
  let totalWeekliesCompleted = 0;
  let totalRaidsPossible = 0;
  let totalAdditionalRaidsPossible = 0;
  let totalDailiesTracked = 0;
  let totalDailiesPossible = 0;
  let totalWeekliesPossible = 0;
  let totalCalendarEventsCompleted = 0;
  let totalCalendarEventsPossible = 0;
  let totalArgeosTracked = 0;
  let totalArgeosAvailableToday = 0;
  let totalArgeosDoneToday = 0;
  let totalArgeosFullyDone = 0;
  let progressPercentage = 0;
  let earnedGoldPercentage = 0;
  let actualGoldDisplay = 0;
  let actualBoundGoldDisplay = 0;
  let actualTradableGoldDisplay = 0;
  let estimatedGoldDisplay = 0;
  let remainingGoldDisplay = 0;
  let dashboardView: DashboardViewMode = 'compact';
  let showDashboardStaticBadges = true;
  let mismatchGoldLost = 0;
  let mismatchGoldBonus = 0;
  let raidDetails: DashboardRaidDetail[] = [];
  let additionalRaidDetails: DashboardRaidDetail[] = [];
  let dailyDetails: DashboardDailyDetail[] = [];
  let weeklyTaskDetails: DashboardWeeklyTaskDetail[] = [];
  let calendarEventDetails: DashboardRosterEventDetail[] = [];
  let argeosDetails: DashboardRosterEventDetail[] = [];
  let calendarEvents: DashboardCalendarEvent[] = [];
  let calendarAssignments: DashboardCalendarAssignment[] = [];
  let raidReservations: DashboardRaidReservation[] = [];
  let calendarLoading = false;
  let loadedCalendarDiscordId = '';
  $: argeosStatusKind = resolveArgeosStatusKind(
    totalArgeosTracked,
    totalArgeosAvailableToday,
    totalArgeosDoneToday,
    totalArgeosFullyDone
  );

  // Load characters for ALL rosters
  async function loadAllCharacters() {
    visibleCharacters = $characters.filter(char => !char.hide_from_dashboard);
    loading = false;
    
    // Calculate stats from all characters so hidden dashboard entries still count for daily/weekly progress.
    await calculateGlobalStats($characters);
    
    // Update header
    if (setHeaderContent) {
      setHeaderContent('');
    }
  }

  async function loadDashboardCalendar() {
    cleanupExpiredDashboardRaidReservations();
    calendarAssignments = await loadDashboardCalendarAssignments();
    raidReservations = await loadDashboardRaidReservations();

    if (!discordId) {
      calendarEvents = [];
      return;
    }

    calendarLoading = true;
    try {
      calendarEvents = await loadUserDashboardCalendarEvents(discordId);
      calendarAssignments = await loadDashboardCalendarAssignments();
      raidReservations = await loadDashboardRaidReservations();
    } catch (error) {
      console.warn('Failed to load dashboard calendar events:', error);
      calendarEvents = [];
    } finally {
      calendarLoading = false;
    }
  }

  // Calculate global statistics using reactive data
  async function calculateGlobalStats(characters: Character[]) {
    try {
      const stats = await buildDashboardStats($rosters, characters, visibleCharacters);
      totalRaidsCompleted = stats.totalRaidsCompleted;
      totalAdditionalRaidsCompleted = stats.totalAdditionalRaidsCompleted;
      totalDailiesCompleted = stats.totalDailiesCompleted;
      totalWeekliesCompleted = stats.totalWeekliesCompleted;
      totalRaidsPossible = stats.totalRaidsPossible;
      totalAdditionalRaidsPossible = stats.totalAdditionalRaidsPossible;
      totalDailiesTracked = stats.totalDailiesTracked;
      totalDailiesPossible = stats.totalDailiesPossible;
      totalWeekliesPossible = stats.totalWeekliesPossible;
      totalCalendarEventsCompleted = stats.totalCalendarEventsCompleted;
      totalCalendarEventsPossible = stats.totalCalendarEventsPossible;
      totalArgeosTracked = stats.totalArgeosTracked;
      totalArgeosAvailableToday = stats.totalArgeosAvailableToday;
      totalArgeosDoneToday = stats.totalArgeosDoneToday;
      totalArgeosFullyDone = stats.totalArgeosFullyDone;
      progressPercentage = stats.progressPercentage;
      earnedGoldPercentage = stats.earnedGoldPercentage;
      actualGoldDisplay = stats.actualGoldDisplay;
      actualBoundGoldDisplay = stats.actualBoundGoldDisplay;
      actualTradableGoldDisplay = stats.actualTradableGoldDisplay;
      estimatedGoldDisplay = stats.estimatedGoldDisplay;
      remainingGoldDisplay = stats.remainingGoldDisplay;
      mismatchGoldLost = stats.mismatchGoldLost;
      mismatchGoldBonus = stats.mismatchGoldBonus;
      characterDataMap = stats.characterDataMap;
      raidDetails = stats.raidDetails;
      additionalRaidDetails = stats.additionalRaidDetails;
      dailyDetails = stats.dailyDetails;
      weeklyTaskDetails = stats.weeklyTaskDetails;
      calendarEventDetails = stats.calendarEventDetails;
      argeosDetails = stats.argeosDetails;
    } catch (error) {
      console.error('Failed to calculate global stats:', error);
    }
  }
  // Initialize app and load all data
  onMount(() => {
    dashboardView = getDashboardViewPreference();
    showDashboardStaticBadges = getDashboardStaticBadgesPreference();

    (async () => {
      await loadAllCharacters();
      await loadDashboardCalendar();
    })();
    
    // Listen for raid settings updates
    const handleRaidSettingsUpdate = async () => {
      // Add small delay to ensure database updates are committed
      await new Promise(resolve => setTimeout(resolve, 100));
      await calculateGlobalStats($characters);
    };
    
    // Listen for raid completions
    const handleRaidCompleted = async () => {
      await calculateGlobalStats($characters);
    };

    const handleCharacterDataComplete = async () => {
      await calculateGlobalStats($characters);
    };

    const handleRosterEventProgressUpdated = async () => {
      await calculateGlobalStats($characters);
    };

    const handleDashboardViewChanged = (event: Event) => {
      const nextView = (event as CustomEvent<DashboardViewMode>).detail;
      dashboardView = nextView === 'cards' ? 'cards' : 'compact';
    };

    const handleCalendarChanged = async () => {
      try {
        calendarAssignments = await loadDashboardCalendarAssignments();
        raidReservations = await loadDashboardRaidReservations();
      } catch {
        calendarAssignments = getDashboardCalendarAssignments();
        raidReservations = getDashboardRaidReservations();
      }
    };

    const handleStaticBadgesChanged = (event: Event) => {
      showDashboardStaticBadges = (event as CustomEvent<boolean>).detail;
    };

    window.addEventListener('raid-settings-updated', handleRaidSettingsUpdate);
    window.addEventListener('raid-completed', handleRaidCompleted);
    window.addEventListener('character-data-complete', handleCharacterDataComplete);
    window.addEventListener('roster-event-progress-updated', handleRosterEventProgressUpdated);
    window.addEventListener('dashboard-view:changed', handleDashboardViewChanged);
    window.addEventListener('dashboard-static-badges:changed', handleStaticBadgesChanged);
    window.addEventListener('dashboard-calendar:changed', handleCalendarChanged);
    
    // Cleanup on unmount
    return () => {
      window.removeEventListener('raid-settings-updated', handleRaidSettingsUpdate);
      window.removeEventListener('raid-completed', handleRaidCompleted);
      window.removeEventListener('character-data-complete', handleCharacterDataComplete);
      window.removeEventListener('roster-event-progress-updated', handleRosterEventProgressUpdated);
      window.removeEventListener('dashboard-view:changed', handleDashboardViewChanged);
      window.removeEventListener('dashboard-static-badges:changed', handleStaticBadgesChanged);
      window.removeEventListener('dashboard-calendar:changed', handleCalendarChanged);
    };
  });

  $: if (!loading) {
    const normalizedDiscordId = String(discordId || '').trim();
    if (normalizedDiscordId !== loadedCalendarDiscordId) {
      loadedCalendarDiscordId = normalizedDiscordId;
      void loadDashboardCalendar();
    }
  }

  $: if (!loading && $characters) {
    visibleCharacters = $characters.filter(char => !char.hide_from_dashboard);
  }

  // Reload data when active roster changes
  $: if ($activeRosterId && !loading) {
    calculateGlobalStats($characters);
  }

  // Group characters by roster
  $: charactersByRoster = (() => {
    const grouped: { [key: string]: Character[] } = {};
    const allRosters = $rosters;
    
    // Initialize groups for all rosters
    allRosters.forEach(roster => {
      grouped[roster.id] = [];
    });
    
    // Group characters by roster
    visibleCharacters.forEach(character => {
      const rosterId = character.roster_id;
      if (grouped[rosterId]) {
        grouped[rosterId].push(character);
      }
    });
    
    // Sort each group by display_order
    Object.keys(grouped).forEach(rosterId => {
      grouped[rosterId].sort((a, b) => a.display_order - b.display_order);
    });
    
    return grouped;
  })();

  // Create reactive data map for character cards
  let characterDataMap: Record<string, DashboardCharacterData> = {};

</script>

<div class="dashboard-container" data-guide="dashboard">
  {#if loading}
    <div class="loading-state">
      <div class="loading-spinner"></div>
      <p>Loading characters...</p>
    </div>
  {:else}
    <WeeklyGoldProgressCard
      {progressPercentage}
      {earnedGoldPercentage}
      {actualGoldDisplay}
      {estimatedGoldDisplay}
      {remainingGoldDisplay}
      {actualBoundGoldDisplay}
      {actualTradableGoldDisplay}
      {mismatchGoldLost}
      {mismatchGoldBonus}
    />

<DashboardStatsBar
      {totalRaidsCompleted}
      {totalRaidsPossible}
      {totalAdditionalRaidsCompleted}
      {totalAdditionalRaidsPossible}
      {totalDailiesCompleted}
      {totalDailiesPossible}
      {totalDailiesTracked}
      {totalWeekliesCompleted}
      {totalWeekliesPossible}
      {totalCalendarEventsCompleted}
      {totalCalendarEventsPossible}
{raidDetails}
      {additionalRaidDetails}
      {dailyDetails}
      {weeklyTaskDetails}
      {calendarEventDetails}
{calendarEvents}      {calendarAssignments}      {raidReservations}      calendarCharacters={visibleCharacters}      calendarLoading={calendarLoading}      calendarCharacterDataMap={characterDataMap}      goldEarnerCount={visibleCharacters.filter(c => c.earns_gold).length}
      visibleCharacterCount={visibleCharacters.length}
    />

    <DashboardRosterSections
      rosters={$rosters}
      {charactersByRoster}
      {dashboardView}
      {characterDataMap}
      showDashboardStaticBadges={showDashboardStaticBadges}
      {calendarEvents}
      {calendarAssignments}
      {raidReservations}
    />
  {/if}
</div>

<style>
  .dashboard-container {
    --app-control-accent: var(--app-dashboard-accent);
    --app-control-on-accent: var(--md-sys-color-on-primary);
    --app-control-accent-container: var(--app-dashboard-accent-soft);
    --app-control-hover-border: var(--app-dashboard-accent);
    box-sizing: border-box;
    padding: 0.65rem;
    width: min(100%, 1920px);
    max-width: 100%;
    margin: 0 auto;
    background: var(--background);
    --dashboard-frame-width: 100%;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    text-align: center;
  }

  .loading-spinner {
    width: 40px;
    height: 40px;
    border: 4px solid var(--surface-variant);
    border-top: 4px solid var(--primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  @media (max-width: 768px) {
    .dashboard-container {
      padding: 0.5rem;
    }
  }
</style>



