<script lang="ts">
  import { rosters, characters, activeRosterId } from '$lib/store';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import type { Character } from '$lib/store';
  import { GAME_CLASSES } from '$lib/data/classes';
  import { RAIDS } from '$lib/data/raids';
  import { getCurrentAvailabilityStatus } from '$lib/utils/availability';
  import CharacterCard from './dashboard/CharacterCard.svelte';

  // Props for header communication
  export let setHeaderContent: (content: string) => void;

  // State
  let visibleCharacters: Character[] = [];
  let loading = true;
  let totalRaidsCompleted = 0;
  let totalDailiesCompleted = 0;
  let totalWeekliesCompleted = 0;
  let totalRaidsPossible = 0;
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
  let actualGoldDisplay = 0;
  let actualBoundGoldDisplay = 0;
  let actualTradableGoldDisplay = 0;
  let estimatedGoldDisplay = 0;
  let dashboardView: 'cards' | 'compact' = 'cards';

  let mismatchGoldLost = 0;

  interface CompletionStatusEntry {
    content_id: string;
    is_completed: number;
    details?: string | null;
    session_id?: string | null;
  }

  interface RestedValueEntry {
    content_id: string;
    current_value: number;
  }

  interface TrackingStatusEntry {
    content_id: string;
    is_tracked: number;
    lazy_daily?: number;
  }

  interface RaidConfigEntry {
    content_id: string;
    gate: string;
    difficulty: string;
    take_gold: number;
    buy_box: number;
  }

  interface DashboardSnapshot {
    characters: Character[];
    rested_by_character: Record<string, RestedValueEntry[]>;
    completion_by_character: Record<string, CompletionStatusEntry[]>;
    tracking_by_character: Record<string, TrackingStatusEntry[]>;
    raid_configs_by_character: Record<string, RaidConfigEntry[]>;
  }

  interface RosterEventProgress {
    task_id: string;
    completed_this_week: number;
    weekly_limit: number;
    completed_today: boolean;
    available: boolean;
  }

  let currentDashboardSnapshot: DashboardSnapshot | null = null;

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

  // Calculate global statistics using reactive data
  async function calculateGlobalStats(characters: Character[]) {
    try {
      // Calculate completion stats
      let raidsCompleted = 0;
      let dailiesCompleted = 0;
      let weekliesCompleted = 0;
      let raidsPossible = 0;
      let dailiesTracked = 0;
      let dailiesPossible = 0;
      let weekliesPossible = 0;
      let calendarEventsCompleted = 0;
      let calendarEventsPossible = 0;
      let argeosTracked = 0;
      let argeosAvailableToday = 0;
      let argeosDoneToday = 0;
      let argeosFullyDone = 0;
      const currentCalendarEventIds = getCurrentCalendarEventIds();
      const nextCharacterDataMap: typeof characterDataMap = {};
      
      // Collect raid configs for all rosters
      let allRaidConfigsByCharacter: Record<string, RaidConfigEntry[]> = {};
      
      // Load data for each roster once
      const rosterDataMap: Record<string, DashboardSnapshot> = {};
      for (const roster of $rosters) {
        const snapshot = await invoke<DashboardSnapshot>('get_dashboard_snapshot', {
          rosterId: roster.id
        });
        rosterDataMap[roster.id] = snapshot;

        if (currentCalendarEventIds.length > 0) {
          if (snapshot.characters?.length > 0) {
            const rosterCompletionStatus = Object.values(snapshot.completion_by_character || {}).flat();

            for (const eventId of currentCalendarEventIds) {
              calendarEventsPossible++;

              if (rosterCompletionStatus.some((c: any) => c.content_id === eventId && c.is_completed === 1)) {
                calendarEventsCompleted++;
              }
            }
          }
        }

        if (isRosterTaskTracked(snapshot, 'event_argeos_winter')) {
          argeosTracked++;
          const progress = await invoke<RosterEventProgress>('get_roster_event_progress', {
            rosterId: roster.id,
            taskId: 'event_argeos_winter'
          });

          if (progress.available) argeosAvailableToday++;
          if (progress.completed_today) argeosDoneToday++;
          if (progress.completed_this_week >= progress.weekly_limit) argeosFullyDone++;
        }
      }
      
      for (const character of characters) {
        try {
          const key = String(character.char_id);
          const rosterSnapshot = rosterDataMap[character.roster_id];
          
          if (!rosterSnapshot) continue;
          
          const completionStatus = rosterSnapshot.completion_by_character?.[key] || [];
          const trackingStatus = rosterSnapshot.tracking_by_character?.[key] || [];
          const restedValues = rosterSnapshot.rested_by_character?.[key] || [];
          const characterRaidConfigs = rosterSnapshot.raid_configs_by_character?.[key] || [];
          
          // Populate character data map for CharacterCard
          nextCharacterDataMap[key] = {
            restedValues,
            completionStatus,
            raidConfigs: characterRaidConfigs,
            trackingStatus
          };
          
          // Collect raid configs for visible characters only. Hidden characters still count for daily/weekly progress,
          // but they stay out of dashboard raid/gold presentation.
          if (!character.hide_from_dashboard && characterRaidConfigs.length > 0) {
            allRaidConfigsByCharacter[key] = characterRaidConfigs;
          }
          
          // Count dailies (chaos + guardian). Lazy dailies only count from 20+ rested.
          const chaosConfigured = trackingStatus.some((t: any) => t.content_id === 'chaos' && Number(t.is_tracked) === 1);
          const guardianConfigured = trackingStatus.some((t: any) => t.content_id === 'guardian' && Number(t.is_tracked) === 1);
          const chaosTracked = shouldCountDaily(trackingStatus, restedValues, 'chaos');
          const guardianTracked = shouldCountDaily(trackingStatus, restedValues, 'guardian');

          if (chaosConfigured) dailiesTracked++;
          if (guardianConfigured) dailiesTracked++;
          
          if (chaosTracked) {
            dailiesPossible++;
            const chaosCompleted = completionStatus.some((c: any) => c.content_id === 'chaos' && c.is_completed === 1);
            if (chaosCompleted) dailiesCompleted++;
          }
          
          if (guardianTracked) {
            dailiesPossible++;
            const guardianCompleted = completionStatus.some((c: any) => c.content_id === 'guardian' && c.is_completed === 1);
            if (guardianCompleted) dailiesCompleted++;
          }
          
          // Count weeklies (cube, paradise, shop, guild, etc.)
          const weeklyTasks = ['cube', 'paradise', 'shop', 'guild'];
          for (const weeklyTask of weeklyTasks) {
            const weeklyTracked = trackingStatus.some((t: any) => t.content_id === weeklyTask && Number(t.is_tracked) === 1);
            if (weeklyTracked) {
              weekliesPossible++;
              const weeklyCompleted = completionStatus.some((c: any) => c.content_id === weeklyTask && c.is_completed === 1);
              if (weeklyCompleted) weekliesCompleted++;
            }
          }
          
          if (!character.hide_from_dashboard) {
            // Count gold raids (unique raids only, not gates)
            const raidConfigs = rosterSnapshot.raid_configs_by_character?.[key] || [];
            const goldRaids = raidConfigs.filter((r: any) => r.take_gold === 1);
            
            // Get unique raid content_ids (not gates)
            const uniqueRaidIds = [...new Set(goldRaids.map((r: any) => r.content_id))];
            raidsPossible += uniqueRaidIds.length;
            
            for (const raidId of uniqueRaidIds) {
              const isCompleted = completionStatus.some((c: any) => c.content_id === raidId && c.is_completed === 1);
              if (isCompleted) raidsCompleted++;
            }
          }
        } catch (error) {
          console.error(`Failed to load stats for character ${character.char_id}:`, error);
        }
      }
      
      totalRaidsCompleted = raidsCompleted;
      totalDailiesCompleted = dailiesCompleted;
      totalWeekliesCompleted = weekliesCompleted;
      totalRaidsPossible = raidsPossible;
      totalDailiesTracked = dailiesTracked;
      totalDailiesPossible = dailiesPossible;
      totalWeekliesPossible = weekliesPossible;
      totalCalendarEventsCompleted = calendarEventsCompleted;
      totalCalendarEventsPossible = calendarEventsPossible;
      totalArgeosTracked = argeosTracked;
      totalArgeosAvailableToday = argeosAvailableToday;
      totalArgeosDoneToday = argeosDoneToday;
      totalArgeosFullyDone = argeosFullyDone;
      characterDataMap = nextCharacterDataMap;
      
      // Update progress percentage from completed raid clears.
      const goldProgress = calculateGoldProgress(visibleCharacters, allRaidConfigsByCharacter);
      actualGoldDisplay = goldProgress.actualGold;
      actualBoundGoldDisplay = goldProgress.actualBoundGold;
      actualTradableGoldDisplay = goldProgress.actualTradableGold;
      estimatedGoldDisplay = goldProgress.plannedGold;
      mismatchGoldLost = goldProgress.lostGold;

      if (goldProgress.plannedGold > 0) {
        progressPercentage = Math.min((goldProgress.actualGold / goldProgress.plannedGold) * 100, 100);
        console.log(
          'Progress calculation - Actual gold:',
          goldProgress.actualGold,
          'Bound gold:',
          goldProgress.actualBoundGold,
          'Tradable gold:',
          goldProgress.actualTradableGold,
          'Max gold:',
          goldProgress.plannedGold,
          'Lost gold:',
          goldProgress.lostGold,
          'Progress %:',
          progressPercentage
        );
      } else {
        progressPercentage = 0;
        console.log('Progress calculation failed - planned gold:', goldProgress.plannedGold);
      }
      
    } catch (error) {
      console.error('Failed to calculate global stats:', error);
    }
  }

  // Initialize app and load all data
  onMount(() => {
    const savedDashboardView = localStorage.getItem('dashboardView');
    if (savedDashboardView === 'cards' || savedDashboardView === 'compact') {
      dashboardView = savedDashboardView;
    }

    (async () => {
      await loadAllCharacters();
    })();
    
    // Listen for raid settings updates
    const handleRaidSettingsUpdate = async () => {
      console.log('DEBUG: Dashboard received raid-settings-updated event');
      console.log('Raid settings updated, refreshing dashboard...');
      // Add small delay to ensure database updates are committed
      await new Promise(resolve => setTimeout(resolve, 100));
      await calculateGlobalStats($characters);
    };
    
    // Listen for raid completions
    const handleRaidCompleted = async () => {
      console.log('Raid completed, refreshing dashboard...');
      await calculateGlobalStats($characters);
    };

    const handleCharacterDataComplete = async () => {
      console.log('Character data completed, refreshing dashboard...');
      await calculateGlobalStats($characters);
    };

    const handleRosterEventProgressUpdated = async () => {
      console.log('Roster event progress updated, refreshing dashboard...');
      await calculateGlobalStats($characters);
    };

    window.addEventListener('raid-settings-updated', handleRaidSettingsUpdate);
    window.addEventListener('raid-completed', handleRaidCompleted);
    window.addEventListener('character-data-complete', handleCharacterDataComplete);
    window.addEventListener('roster-event-progress-updated', handleRosterEventProgressUpdated);
    
    // Cleanup on unmount
    return () => {
      window.removeEventListener('raid-settings-updated', handleRaidSettingsUpdate);
      window.removeEventListener('raid-completed', handleRaidCompleted);
      window.removeEventListener('character-data-complete', handleCharacterDataComplete);
      window.removeEventListener('roster-event-progress-updated', handleRosterEventProgressUpdated);
    };
  });

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
  let characterDataMap: Record<string, {
    restedValues: Array<{ content_id: string; current_value: number }>;
    completionStatus: CompletionStatusEntry[];
    raidConfigs: RaidConfigEntry[];
    trackingStatus: Array<{ content_id: string; is_tracked: number; lazy_daily?: number }>;
  }> = {};

  // Pre-build a lookup map for RAIDS by id+difficulty to avoid O(n) scans
  const raidLookup: Record<string, typeof RAIDS[0]> = {};
  for (const raid of RAIDS) {
    raidLookup[`${raid.id}-${raid.difficulty}`] = raid;
  }

  function getGateIdFromSession(sessionId?: string | null): string | null {
    if (!sessionId) return null;
    const parts = sessionId.split('_');
    return parts.length > 1 ? parts[parts.length - 1] : null;
  }

  function findRaid(contentId: string, difficulty?: string | null): typeof RAIDS[0] | undefined {
    const normalizedDifficulty = (difficulty ?? '').trim().toLowerCase();
    return Object.values(raidLookup).find(
      raid => raid.id === contentId && raid.difficulty.trim().toLowerCase() === normalizedDifficulty
    );
  }

  function getGateGoldBreakdown(
    contentId: string,
    difficulty: string | null | undefined,
    gateId: string,
    buyBox: number
  ): { boundGold: number; tradableGold: number; totalGold: number } {
    const raid = findRaid(contentId, difficulty);
    const gate = raid?.gates.find(g => g.gate === gateId);
    if (!gate) return { boundGold: 0, tradableGold: 0, totalGold: 0 };

    const boundGold = gate.boundGold || 0;
    const boxPrice = buyBox === 1 ? (gate.boxPrice || 0) : 0;
    const tradableGold = (gate.tradableGold || 0) - boxPrice;
    return {
      boundGold,
      tradableGold,
      totalGold: boundGold + tradableGold
    };
  }

  function calculateGoldProgress(
    characters: Character[],
    raidConfigsByCharacter: Record<string, RaidConfigEntry[]>
  ): { plannedGold: number; actualGold: number; actualBoundGold: number; actualTradableGold: number; lostGold: number } {
    let plannedGold = 0;
    let actualGold = 0;
    let actualBoundGold = 0;
    let actualTradableGold = 0;
    let lostGold = 0;

    for (const character of characters) {
      if (!character.earns_gold) continue;

      try {
        const charKey = String(character.char_id);
        const raidConfigs = raidConfigsByCharacter[charKey] || [];
        const goldRaids = raidConfigs.filter(config => config.take_gold === 1);
        const completionData = characterDataMap[charKey]?.completionStatus ?? [];
        const countedGates = new Set<string>();

        for (const config of goldRaids) {
          const gateId = config.gate;
          const gateKey = `${config.content_id}-${config.difficulty}-${gateId}`;
          if (countedGates.has(gateKey)) continue;
          countedGates.add(gateKey);

          const plannedGateGold = getGateGoldBreakdown(config.content_id, config.difficulty, gateId, config.buy_box);
          plannedGold += plannedGateGold.totalGold;

          const clearedEntry = completionData.find(entry =>
            entry.content_id === config.content_id &&
            entry.is_completed === 1 &&
            entry.details &&
            getGateIdFromSession(entry.session_id) === gateId
          );
          if (!clearedEntry) continue;

          const actualGateGold = getGateGoldBreakdown(config.content_id, clearedEntry.details, gateId, config.buy_box);
          actualGold += actualGateGold.totalGold;
          actualBoundGold += actualGateGold.boundGold;
          actualTradableGold += actualGateGold.tradableGold;

          const diff = plannedGateGold.totalGold - actualGateGold.totalGold;
          if (diff > 0) {
            lostGold += diff;
          }
        }
      } catch (error) {
        console.error(`Failed to calculate gold for character ${character.char_id}:`, error);
      }
    }

    return { plannedGold, actualGold, actualBoundGold, actualTradableGold, lostGold };
  }

  function getClassIcon(classId: string): string {
    const classInfo = GAME_CLASSES[classId];
    return classInfo ? classInfo.iconId : '0';
  }

  function getClassName(classId: string): string {
    const classInfo = GAME_CLASSES[classId];
    return classInfo ? classInfo.displayName : classId;
  }

  function getRestedValue(restedValues: RestedValueEntry[], contentId: string): number {
    return restedValues.find((value) => value.content_id === contentId)?.current_value || 0;
  }

  function shouldCountDaily(
    trackingStatus: TrackingStatusEntry[],
    restedValues: RestedValueEntry[],
    contentId: string
  ): boolean {
    const tracking = trackingStatus.find((status) => status.content_id === contentId && Number(status.is_tracked) === 1);
    if (!tracking) return false;
    if (Number(tracking.lazy_daily) === 1) {
      return getRestedValue(restedValues, contentId) >= 20;
    }
    return true;
  }

  function setDashboardView(view: 'cards' | 'compact') {
    dashboardView = view;
    localStorage.setItem('dashboardView', view);
  }

  function getOpenCount(completed: number, possible: number): number {
    return Math.max(possible - completed, 0);
  }

  function getOpenStatusKind(completed: number, possible: number, configured = possible): 'empty' | 'idle' | 'done' | 'open' {
    if (possible <= 0) return configured > 0 ? 'idle' : 'empty';
    return getOpenCount(completed, possible) === 0 ? 'done' : 'open';
  }

  function isRosterTaskTracked(snapshot: DashboardSnapshot, taskId: string): boolean {
    return Object.values(snapshot.tracking_by_character || {})
      .flat()
      .some((entry) => entry.content_id === taskId && Number(entry.is_tracked) === 1);
  }

  function getArgeosStatusKind(): 'empty' | 'done' | 'today' | 'open' {
    if (totalArgeosTracked <= 0) return 'empty';
    if (totalArgeosFullyDone >= totalArgeosTracked) return 'done';
    if (totalArgeosAvailableToday > 0) return 'open';
    if (totalArgeosDoneToday > 0) return 'today';
    return 'empty';
  }

  function getCurrentCalendarEventLabel(): string {
    const availability = getCurrentAvailabilityStatus();

    if (availability.gate && availability.boss) {
      return 'Chaos Gate + Field Boss';
    }

    if (availability.gate) {
      return 'Chaos Gate';
    }

    if (availability.boss) {
      return 'Field Boss';
    }

    return 'No Event';
  }

  function getCurrentCalendarEventIds(): string[] {
    const availability = getCurrentAvailabilityStatus();
    const eventIds: string[] = [];

    if (availability.gate) {
      eventIds.push('gate');
    }

    if (availability.boss) {
      eventIds.push('boss');
    }

    return eventIds;
  }

  function getCurrentCalendarEventIcon(): string {
    const availability = getCurrentAvailabilityStatus();

    if (availability.gate) {
      return '/images/chaos_gate.png';
    }

    if (availability.boss) {
      return '/images/boss.png';
    }

    return 'images/calendar_7743808.png';
  }

</script>

<div class="dashboard-container" data-guide="dashboard">
  {#if loading}
    <div class="loading-state">
      <div class="loading-spinner"></div>
      <p>Loading characters...</p>
    </div>
  {:else}
    <!-- Progress Banner -->
    <div class="gold-card-modern">
      <div class="card-glass-overlay"></div>
      
      <div class="card-content">
        <div class="gold-info-main">
          <div class="title-group">
            <img src="/images/gold.png" alt="Gold" class="gold-icon-large" />
            <h3>Weekly Gold Progress</h3>
          </div>
          
          <div class="gold-values">
            <span class="current" style={`--gold-progress: ${Math.min(progressPercentage, 100)}%`}>{actualGoldDisplay.toLocaleString()}</span>
            <span class="divider">/</span>
            <span class="target">{estimatedGoldDisplay.toLocaleString()}</span>
            <span class="unit">Gold</span>
          </div>
        </div>

        <div class="progress-container-modern">
          <div class="progress-track">
            <div class="progress-fill-glow" style="width: {Math.min(progressPercentage, 100)}%">
              <div class="shimmer"></div>
            </div>
            {#if mismatchGoldLost > 0 && estimatedGoldDisplay > 0}
              <div
                class="progress-fill-lost"
                style="left: {Math.min(progressPercentage, 100)}%; width: {Math.min(mismatchGoldLost / estimatedGoldDisplay * 100, 100 - Math.min(progressPercentage, 100))}%"
              ></div>
            {/if}
          </div>
          <div class="progress-labels">
            <span class="pct-text">{Math.round(progressPercentage)}% complete</span>
            {#if mismatchGoldLost > 0}
              <span class="remaining-text mismatch-loss">⚠ {mismatchGoldLost.toLocaleString()} lost to difficulty mismatch</span>
            {:else}
              <span class="remaining-text">{Math.max(estimatedGoldDisplay - actualGoldDisplay, 0).toLocaleString()} gold remaining</span>
            {/if}
          </div>
        </div>

        <div class="gold-details-minimal">
          <div class="detail-item">
            <span class="dot bound"></span>
            <span class="label">Bound:</span>
            <span class="val">{actualBoundGoldDisplay.toLocaleString()}</span>
          </div>
          <div class="detail-item">
            <span class="dot tradable"></span>
            <span class="label">Tradable:</span>
            <span class="val">{actualTradableGoldDisplay.toLocaleString()}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Header Stats -->
    {#if totalRaidsPossible > 0 || totalDailiesTracked > 0 || totalWeekliesPossible > 0 || totalArgeosTracked > 0 || visibleCharacters.some(c => c.earns_gold) || visibleCharacters.length > 0}
    <div class="header-stats">
      {#if totalRaidsPossible > 0}
      <div class="stat-card">
        <div class="stat-icon">
          <img src="/images/kazeros-raid.webp" alt="Raids" />
        </div>
        <div class="stat-content">
          <div class="stat-status" class:done={getOpenStatusKind(totalRaidsCompleted, totalRaidsPossible) === 'done'}>
            {#if getOpenStatusKind(totalRaidsCompleted, totalRaidsPossible) === 'done'}
              <span class="stat-status-text">All done</span>
            {:else}
              <span class="stat-open-count">{getOpenCount(totalRaidsCompleted, totalRaidsPossible)}</span>
              <span class="stat-open-label">open</span>
            {/if}
          </div>
          <div class="stat-label">Raids</div>
        </div>
      </div>
      {/if}
      {#if totalDailiesTracked > 0}
      <div class="stat-card">
        <div class="stat-icon">
          <img src="/images/icons8-last-24-hours-80.png" alt="Dailies" />
        </div>
        <div class="stat-content">
          <div
            class="stat-status"
            class:done={getOpenStatusKind(totalDailiesCompleted, totalDailiesPossible, totalDailiesTracked) === 'done'}
            class:idle={getOpenStatusKind(totalDailiesCompleted, totalDailiesPossible, totalDailiesTracked) === 'idle'}
          >
            {#if getOpenStatusKind(totalDailiesCompleted, totalDailiesPossible, totalDailiesTracked) === 'idle'}
              <span class="stat-status-text">Resting</span>
            {:else if getOpenStatusKind(totalDailiesCompleted, totalDailiesPossible, totalDailiesTracked) === 'done'}
              <span class="stat-status-text">All done</span>
            {:else}
              <span class="stat-open-count">{getOpenCount(totalDailiesCompleted, totalDailiesPossible)}</span>
              <span class="stat-open-label">open</span>
            {/if}
          </div>
          <div class="stat-label">Dailies</div>
        </div>
      </div>
      {/if}
      {#if totalWeekliesPossible > 0}
      <div class="stat-card">
        <div class="stat-icon">
          <img src="images/calendar_7743808.png" alt="Weeklies" />
        </div>
        <div class="stat-content">
          <div class="stat-status" class:done={getOpenStatusKind(totalWeekliesCompleted, totalWeekliesPossible) === 'done'}>
            {#if getOpenStatusKind(totalWeekliesCompleted, totalWeekliesPossible) === 'done'}
              <span class="stat-status-text">All done</span>
            {:else}
              <span class="stat-open-count">{getOpenCount(totalWeekliesCompleted, totalWeekliesPossible)}</span>
              <span class="stat-open-label">open</span>
            {/if}
          </div>
          <div class="stat-label">Weeklies</div>
        </div>
      </div>
      {/if}
      <div class="stat-card">
        <div class="stat-icon">
          <img src={getCurrentCalendarEventIcon()} alt="Calendar Event" />
        </div>
        <div class="stat-content">
          <div
            class="stat-status"
            class:done={getOpenStatusKind(totalCalendarEventsCompleted, totalCalendarEventsPossible) === 'done'}
            class:empty={getOpenStatusKind(totalCalendarEventsCompleted, totalCalendarEventsPossible) === 'empty'}
          >
            {#if getOpenStatusKind(totalCalendarEventsCompleted, totalCalendarEventsPossible) === 'empty'}
              <span class="stat-status-text">No event</span>
            {:else if getOpenStatusKind(totalCalendarEventsCompleted, totalCalendarEventsPossible) === 'done'}
              <span class="stat-status-text">All done</span>
            {:else}
              <span class="stat-open-count">{getOpenCount(totalCalendarEventsCompleted, totalCalendarEventsPossible)}</span>
              <span class="stat-open-label">open</span>
            {/if}
          </div>
          <div class="stat-label event-name">{getCurrentCalendarEventLabel()}</div>
        </div>
      </div>
      {#if totalArgeosTracked > 0}
      <div class="stat-card">
        <div class="stat-icon">
          <img src="/images/event_quest.webp" alt="Stoopid Argeos" />
        </div>
        <div class="stat-content">
          <div
            class="stat-status"
            class:done={getArgeosStatusKind() === 'done'}
            class:idle={getArgeosStatusKind() === 'today'}
          >
            {#if getArgeosStatusKind() === 'done'}
              <span class="stat-status-text">Fully done</span>
            {:else if getArgeosStatusKind() === 'today'}
              <span class="stat-status-text">Done today</span>
            {:else if getArgeosStatusKind() === 'open'}
              <span class="stat-open-count">{totalArgeosAvailableToday}</span>
              <span class="stat-open-label">open</span>
            {:else}
              <span class="stat-status-text">Not tracked</span>
            {/if}
          </div>
          <div class="stat-label event-name">Stoopid Argeos</div>
        </div>
      </div>
      {/if}
      {#if visibleCharacters.some(c => c.earns_gold)}
      <div class="stat-card">
        <div class="stat-icon">
          <img src="/images/gold.png" alt="Gold Earners" />
        </div>
        <div class="stat-content">
          <div class="stat-value">{visibleCharacters.filter(c => c.earns_gold).length}</div>
          <div class="stat-label">Gold Earners</div>
        </div>
      </div>
      {/if}
    </div>
    {/if}

    <div class="dashboard-view-toolbar">
      <div>
        <h3>Roster View</h3>
      </div>
      <div class="view-switch" aria-label="Dashboard view mode">
        <button
          type="button"
          class:active={dashboardView === 'cards'}
          on:click={() => setDashboardView('cards')}
        >
          Cards
        </button>
        <button
          type="button"
          class:active={dashboardView === 'compact'}
          on:click={() => setDashboardView('compact')}
        >
          List
        </button>
      </div>
    </div>

    <!-- Character Cards Grid -->
    <div class="characters-grid">
      {#each Object.entries(charactersByRoster) as [rosterId, rosterCharacters], index}
        <div class="roster-section roster-{rosterId}">
          <h3 class="roster-title">
            <span class="roster-title-text">
              {#each $rosters as roster}
                {#if roster.id === rosterId}
                  {roster.roster_name}
                {/if}
              {/each}
              <span class="character-count">({rosterCharacters.length})</span>
            </span>
          </h3>
          
          <div class="characters-list" class:compact-list={dashboardView === 'compact'}>
            {#each rosterCharacters as character}
              <CharacterCard 
                {character}
                viewMode={dashboardView}
                classIcon={getClassIcon(character.class_id)}
                className={getClassName(character.class_id)}
                restedValues={characterDataMap[String(character.char_id)]?.restedValues || []}
                completionStatus={characterDataMap[String(character.char_id)]?.completionStatus || []}
                raidConfigs={characterDataMap[String(character.char_id)]?.raidConfigs || []}
                trackingStatus={characterDataMap[String(character.char_id)]?.trackingStatus || []}
              />
            {/each}
          </div>
        </div>
      {/each}
      
      {#if Object.keys(charactersByRoster).length === 0}
        <div class="empty-state">
          <div class="empty-icon">👥</div>
          <h3>No Characters Found</h3>
          <p>Add a roster and characters to get started!</p>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .dashboard-container {
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

  /* Progress Banner */
  .gold-card-modern {
    position: relative;
    width: var(--dashboard-frame-width);
    box-sizing: border-box;
    background: #1a1a1d;
    border: 1px solid rgba(255, 215, 0, 0.15);
    border-radius: 14px;
    padding: 1rem;
    margin-bottom: 0.75rem;
    overflow: hidden;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
  }

  .card-glass-overlay {
    position: absolute;
    top: -50%;
    left: -20%;
    width: 140%;
    height: 200%;
    background: radial-gradient(circle at center, rgba(255, 215, 0, 0.05) 0%, transparent 70%);
    pointer-events: none;
  }

  .card-content {
    position: relative;
    z-index: 2;
  }

  .gold-info-main {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .title-group {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .gold-icon-large {
    width: 28px;
    height: 28px;
    filter: drop-shadow(0 0 8px rgba(255, 215, 0, 0.4));
  }

  .gold-values {
    font-size: 1.55rem;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
  }

  .gold-values .current {
    color: transparent;
    background: linear-gradient(90deg, #b8860b 0%, #ffd700 var(--gold-progress), #ffec8b 100%);
    background-clip: text;
    -webkit-background-clip: text;
    text-shadow: 0 0 15px rgba(255, 215, 0, 0.25);
  }
  .gold-values .divider { color: #444; margin: 0 0.25rem; }
  .gold-values .target { color: #888; }
  .gold-values .unit { font-size: 0.875rem; color: #555; margin-left: 0.5rem; text-transform: uppercase; }

  .progress-container-modern {
    margin-bottom: 0.65rem;
  }

  .progress-track {
    height: 10px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 5px;
    overflow: hidden;
    position: relative;
    border: 1px solid rgba(255, 255, 255, 0.03);
  }

  .progress-fill-glow {
    height: 100%;
    background: linear-gradient(90deg, #b8860b, #ffd700, #ffec8b);
    border-radius: 5px;
    position: relative;
    box-shadow: 0 0 15px rgba(255, 215, 0, 0.4);
    transition: width 1s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .shimmer {
    position: absolute;
    top: 0; left: 0; right: 0; bottom: 0;
    background: linear-gradient(
      90deg,
      transparent,
      rgba(255, 255, 255, 0.2),
      transparent
    );
    animation: shimmer 2s infinite;
  }

  @keyframes shimmer {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(100%); }
  }

  .progress-labels {
    display: flex;
    justify-content: space-between;
    margin-top: 0.5rem;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .pct-text { color: #ffd700; }
  .remaining-text { color: #666; }
  .remaining-text.mismatch-loss { color: #f87171; }

  .progress-fill-lost {
    position: absolute;
    top: 0;
    height: 100%;
    background: repeating-linear-gradient(
      45deg,
      rgba(120, 120, 120, 0.45),
      rgba(120, 120, 120, 0.45) 4px,
      rgba(60, 60, 60, 0.2) 4px,
      rgba(60, 60, 60, 0.2) 8px
    );
    border-radius: 0 3px 3px 0;
    transition: width 1s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .gold-details-minimal {
    display: flex;
    gap: 1rem;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    padding-top: 0.65rem;
  }

  .detail-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }

  .dot { width: 8px; height: 8px; border-radius: 50%; }
  .dot.bound { background: #ff6b6b; box-shadow: 0 0 6px #ff6b6b; }
  .dot.tradable { background: #ffd700; box-shadow: 0 0 6px #ffd700; }
  .detail-item .label { color: #777; }
  .detail-item .val { color: #eee; font-weight: 600; }

  .progress-info h3 {
    margin: 0 0 0.5rem 0;
    color: white;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .progress-stats {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    color: rgba(255, 255, 255, 0.9);
    font-size: 1.5rem;
    font-weight: 700;
  }

  .current-gold {
    color: #ffd700;
  }

  .separator {
    color: rgba(255, 255, 255, 0.6);
  }

  .estimated-gold {
    color: rgba(255, 255, 255, 0.8);
  }

  .gold-label {
    font-size: 0.875rem;
    opacity: 0.8;
    margin-left: 0.25rem;
  }

  .gold-breakdown {
    display: flex;
    gap: 1rem;
    margin-top: 0.5rem;
    flex-wrap: wrap;
  }

  .gold-type {
    display: flex;
    flex-direction: column;
    align-items: center;
    font-size: 0.75rem;
    opacity: 0.9;
  }

  .gold-type-label {
    font-size: 0.625rem;
    opacity: 0.7;
    margin-top: 0.125rem;
  }

  .bound-gold {
    color: #ff6b6b;
    font-weight: 600;
  }

  .tradable-gold {
    color: #ffd700;
    font-weight: 600;
  }

  .extra-income {
    color: #51cf66;
  }

  .extra-income-gold {
    font-weight: 600;
  }

  .box-cost {
    color: #ff8787;
  }

  .box-purchase-cost {
    font-weight: 600;
  }

  .progress-percentage {
    font-size: 1.75rem;
    font-weight: 700;
    color: #ffffff;
    text-shadow: 0 0 6px rgba(0, 0, 0, 0.45);
  }

  /* Header Stats */
  .header-stats {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.5rem;
    width: var(--dashboard-frame-width);
    box-sizing: border-box;
    margin-bottom: 0.6rem;
  }

  .stat-card {
    flex: 0 1 154px;
    min-width: 132px;
    max-width: 172px;
    box-sizing: border-box;
    background: var(--surface-variant);
    border: 1px solid rgba(255, 140, 0, 0.25);
    border-radius: 8px;
    padding: 0.52rem 0.65rem;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
  }

  .stat-icon {
    width: 30px;
    height: 30px;
    flex: 0 0 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--primary);
    border-radius: 8px;
  }

  .stat-icon img {
    width: 20px;
    height: 20px;
    object-fit: contain;
  }

  .stat-content {
    flex: 1;
    min-width: 0;
    text-align: center;
  }

  .stat-value {
    font-size: clamp(1rem, 1.4vw, 1.25rem);
    font-weight: 700;
    color: var(--on-surface);
    line-height: 1;
    white-space: nowrap;
  }

  .stat-status {
    min-height: 1.35rem;
    display: inline-flex;
    max-width: 100%;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
    line-height: 1;
    color: var(--on-surface);
    white-space: nowrap;
  }

  .stat-open-count {
    font-size: 1.02rem;
    font-weight: 800;
    color: var(--on-surface);
  }

  .stat-open-label {
    color: var(--on-surface-variant);
    font-size: 0.68rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0;
  }

  .stat-status.done,
  .stat-status.idle,
  .stat-status.empty {
    min-width: 0;
    max-width: 100%;
  }

  .stat-status.done .stat-status-text {
    color: color-mix(in srgb, #36d399 72%, var(--on-surface));
  }

  .stat-status.idle .stat-status-text,
  .stat-status.empty .stat-status-text {
    color: var(--on-surface-variant);
  }

  .stat-status-text {
    color: var(--on-surface);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 0.76rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0;
  }

  .stat-label {
    font-size: 0.72rem;
    color: var(--on-surface-variant);
    margin-top: 0.2rem;
    text-align: center;
    white-space: nowrap;
  }

  .stat-label.event-name {
    font-size: 0.72rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Characters Grid */
  .dashboard-view-toolbar {
    width: var(--dashboard-frame-width);
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.55rem;
  }

  .dashboard-view-toolbar h3 {
    margin: 0;
    color: var(--on-surface);
    font-size: 1rem;
    font-weight: 700;
  }

  .view-switch {
    display: inline-flex;
    padding: 0.25rem;
    border-radius: 10px;
    background: var(--surface-variant);
    border: 1px solid rgba(255, 140, 0, 0.25);
    box-shadow: 0 3px 10px rgba(0, 0, 0, 0.12);
  }

  .view-switch button {
    position: relative;
    border: 0;
    background: transparent;
    color: var(--on-surface-variant);
    border-radius: 8px;
    padding: 0.45rem 0.8rem;
    font-size: 0.8rem;
    font-weight: 700;
    cursor: pointer;
    transition: background 0.2s ease, color 0.2s ease;
  }

  .view-switch button.active {
    background: linear-gradient(135deg, var(--primary), color-mix(in srgb, var(--primary) 78%, #ffd700));
    color: var(--on-primary);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.25),
      0 2px 10px color-mix(in srgb, var(--primary) 35%, transparent);
  }

  .view-switch button.active::after {
    content: '';
    position: absolute;
    left: 50%;
    bottom: 0.2rem;
    width: 18px;
    height: 2px;
    border-radius: 999px;
    background: currentColor;
    transform: translateX(-50%);
    opacity: 0.9;
  }

  .characters-grid {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    width: var(--dashboard-frame-width);
    box-sizing: border-box;
    align-items: center;
  }

  .characters-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, 180px);
    grid-auto-flow: dense;
    grid-auto-rows: 72px;
    gap: 1rem;
    align-items: stretch;
    width: 100%;
    box-sizing: border-box;
    overflow: visible;
    justify-content: center;
  }

  .characters-list:not(.compact-list) :global(.character-card) {
    grid-column: span 2;
    grid-row: span 2;
    min-height: 0;
  }

  .characters-list:not(.compact-list) :global(.character-card.minimal-card) {
    grid-row: span 1;
  }

  .characters-list.compact-list {
    grid-template-columns: repeat(3, minmax(0, 1fr));
    grid-auto-rows: auto;
    grid-auto-flow: row;
    gap: 0.5rem;
    align-items: stretch;
  }

  .characters-list.compact-list :global(.character-card:not(.minimal-card)) {
    grid-column: 1 / -1;
  }

  .roster-section {
    --roster-border-color: rgba(255, 140, 0, 0.45);
    box-sizing: border-box;
    background: var(--surface-variant);
    border-radius: 8px;
    padding: 0.7rem 0.75rem 0.75rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    transition: transform 0.2s ease, box-shadow 0.2s ease;
    border: 1px solid var(--roster-border-color);
    position: relative;
    width: 100%;
    max-width: none;
  }

  .roster-section:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.15);
    border-color: rgba(255, 140, 0, 0.5);
  }

  .roster-separator {
    height: 3px;
    background: linear-gradient(90deg, transparent, var(--primary), transparent);
    margin: 0.5rem 0;
    border-radius: 2px;
    opacity: 0.7;
    position: relative;
  }

  .roster-separator::before {
    content: '';
    position: absolute;
    top: -8px;
    left: 50%;
    transform: translateX(-50%);
    width: 8px;
    height: 8px;
    background: var(--primary);
    border-radius: 50%;
    box-shadow: 0 0 0 4px var(--primary);
  }

  .roster-title {
    margin: 0 0 0.65rem;
    color: var(--roster-border-color);
    font-size: 0.68rem;
    line-height: 1;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    text-transform: uppercase;
    letter-spacing: 0;
  }

  .roster-title-text {
    flex: 0 1 auto;
    max-width: min(70%, 22rem);
    min-width: 0;
    display: inline-flex;
    align-items: baseline;
    justify-content: center;
    gap: 0.25rem;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .character-count {
    flex: 0 0 auto;
    background: transparent;
    color: inherit;
    padding: 0;
    border-radius: 0;
    font-size: 0.62rem;
    font-weight: 500;
  }

  .empty-state {
    text-align: center;
    padding: 4rem 2rem;
    color: var(--on-surface-variant);
  }

  .empty-icon {
    font-size: 4rem;
    margin-bottom: 1rem;
    opacity: 0.5;
  }

  .empty-state h3 {
    margin: 0 0 0.5rem 0;
    color: var(--on-surface);
  }

  .empty-state p {
    margin: 0;
    opacity: 0.8;
  }

  @media (max-width: 768px) {
    .dashboard-container {
      padding: 0.5rem;
    }

    .progress-content {
      flex-direction: column;
      gap: 1rem;
      text-align: center;
    }

    .progress-bar-container {
      max-width: 100%;
    }

    .header-stats {
      gap: 0.45rem;
    }

    .stat-card {
      flex-basis: 148px;
    }

    .characters-list {
      grid-template-columns: 1fr;
    }

    .characters-list:not(.compact-list) :global(.character-card) {
      grid-column: 1 / -1;
    }

    .characters-list.compact-list {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .dashboard-view-toolbar {
      align-items: stretch;
      flex-direction: column;
    }

    .view-switch {
      width: 100%;
    }

    .view-switch button {
      flex: 1;
    }
  }

  @media (max-width: 480px) {
    .characters-list.compact-list {
      grid-template-columns: 1fr;
    }

    .stat-card {
      flex-basis: min(100%, 154px);
    }
  }
</style>
