<script lang="ts">
  import { rosters, characters, activeRosterId } from '$lib/store';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import type { Character } from '$lib/store';
  import type { GoldStatsResponse } from '$lib/types/gold';
  import { GAME_CLASSES } from '$lib/data/classes';
  import { RAIDS } from '$lib/data/raids';
  import CharacterCard from './dashboard/CharacterCard.svelte';

  // Props for header communication
  export let setHeaderContent: (content: string) => void;

  // State
  let visibleCharacters: Character[] = [];
  let loading = true;
  let goldStats: GoldStatsResponse | null = null;
  let totalRaidsCompleted = 0;
  let totalDailiesCompleted = 0;
  let totalWeekliesCompleted = 0;
  let totalRaidsPossible = 0;
  let totalDailiesPossible = 0;
  let totalWeekliesPossible = 0;
  let progressPercentage = 0;
  let estimatedGoldDisplay = 0;

  interface CompletionStatusEntry {
    content_id: string;
    is_completed: number;
  }

  interface RestedValueEntry {
    content_id: string;
    current_value: number;
  }

  interface TrackingStatusEntry {
    content_id: string;
    is_tracked: number;
  }

  interface RaidConfigEntry {
    content_id: string;
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

  let currentDashboardSnapshot: DashboardSnapshot | null = null;

  // Load characters for ALL rosters
  async function loadAllCharacters() {
    visibleCharacters = $characters.filter(char => !char.hide_from_dashboard);
    loading = false;
    
    // Calculate stats
    await calculateGlobalStats(visibleCharacters);
    
    // Update header
    if (setHeaderContent) {
      setHeaderContent('');
    }
  }

  // Calculate global statistics using reactive data
  async function calculateGlobalStats(characters: Character[]) {
    try {
      // Load gold stats for all rosters combined
      console.log('Fetching gold stats for all rosters');
      const result = await invoke('get_weekly_gold_stats', { rosterId: null });
      
      // Backend returns snake_case, map to camelCase for TypeScript interface
      if (result && typeof result === 'object') {
        const backendResult = result as any;
        goldStats = {
          weekly: {
            tradableGold: backendResult.tradable_gold || 0,
            boundGold: backendResult.bound_gold || 0,
            totalGold: backendResult.total_gold || 0,
            totalEntries: backendResult.total_entries || 0,
            extraIncomeGold: 0,
            boxPurchaseCost: 0
          },
          recentEntries: []
        } as GoldStatsResponse;
        console.log('Processed goldStats:', goldStats);
        console.log('Actual gold from backend:', goldStats.weekly.totalGold);
      } else {
        console.warn('Invalid gold stats response, using default:', result);
        goldStats = {
          weekly: {
            tradableGold: 0,
            boundGold: 0,
            totalGold: 0,
            totalEntries: 0,
            extraIncomeGold: 0,
            boxPurchaseCost: 0
          },
          recentEntries: []
        };
      }
      
      // Calculate completion stats
      let raidsCompleted = 0;
      let dailiesCompleted = 0;
      let weekliesCompleted = 0;
      let raidsPossible = 0;
      let dailiesPossible = 0;
      let weekliesPossible = 0;
      
      // Collect raid configs for all rosters
      let allRaidConfigsByCharacter: Record<string, RaidConfigEntry[]> = {};
      
      // Load data for each roster once
      const rosterDataMap: Record<string, DashboardSnapshot> = {};
      for (const roster of $rosters) {
        const snapshot = await invoke<DashboardSnapshot>('get_dashboard_snapshot', {
          rosterId: roster.id
        });
        rosterDataMap[roster.id] = snapshot;
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
          characterDataMap[key] = {
            restedValues,
            completionStatus,
            raidConfigs: characterRaidConfigs,
            trackingStatus
          };
          
          // Collect raid configs for gold calculation
          if (characterRaidConfigs.length > 0) {
            allRaidConfigsByCharacter[key] = characterRaidConfigs;
          }
          
          // Count dailies (chaos + guardian)
          const chaosTracked = trackingStatus.some((t: any) => t.content_id === 'chaos' && t.is_tracked === 1);
          const guardianTracked = trackingStatus.some((t: any) => t.content_id === 'guardian' && t.is_tracked === 1);
          
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
            const weeklyTracked = trackingStatus.some((t: any) => t.content_id === weeklyTask && t.is_tracked === 1);
            if (weeklyTracked) {
              weekliesPossible++;
              const weeklyCompleted = completionStatus.some((c: any) => c.content_id === weeklyTask && c.is_completed === 1);
              if (weeklyCompleted) weekliesCompleted++;
            }
          }
          
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
        } catch (error) {
          console.error(`Failed to load stats for character ${character.char_id}:`, error);
        }
      }
      
      totalRaidsCompleted = raidsCompleted;
      totalDailiesCompleted = dailiesCompleted;
      totalWeekliesCompleted = weekliesCompleted;
      totalRaidsPossible = raidsPossible;
      totalDailiesPossible = dailiesPossible;
      totalWeekliesPossible = weekliesPossible;
      
      // Update progress percentage and maximum gold display
      const maxGold = calculateTotalEstimatedGold(characters, allRaidConfigsByCharacter);
      estimatedGoldDisplay = maxGold;
      
      // Calculate progress percentage using actual gold stats
      if (goldStats && goldStats.weekly && maxGold > 0) {
        progressPercentage = Math.min((goldStats.weekly.totalGold / maxGold) * 100, 100);
        console.log('Progress calculation - Actual gold:', goldStats.weekly.totalGold, 'Max gold:', maxGold, 'Progress %:', progressPercentage);
      } else {
        progressPercentage = 0;
        console.log('Progress calculation failed - goldStats:', goldStats, 'maxGold:', maxGold);
      }
      
    } catch (error) {
      console.error('Failed to calculate global stats:', error);
    }
  }

  // Initialize app and load all data
  onMount(() => {
    (async () => {
      await loadAllCharacters();
    })();
    
    // Listen for raid settings updates
    const handleRaidSettingsUpdate = async () => {
      console.log('DEBUG: Dashboard received raid-settings-updated event');
      console.log('Raid settings updated, refreshing dashboard...');
      // Add small delay to ensure database updates are committed
      await new Promise(resolve => setTimeout(resolve, 100));
      await calculateGlobalStats(visibleCharacters);
    };
    
    // Listen for raid completions
    const handleRaidCompleted = async () => {
      console.log('Raid completed, refreshing dashboard...');
      await calculateGlobalStats(visibleCharacters);
    };
    
    window.addEventListener('raid-settings-updated', handleRaidSettingsUpdate);
    window.addEventListener('raid-completed', handleRaidCompleted);
    
    // Cleanup on unmount
    return () => {
      window.removeEventListener('raid-settings-updated', handleRaidSettingsUpdate);
      window.removeEventListener('raid-completed', handleRaidCompleted);
    };
  });

  $: if (!loading && $characters) {
    visibleCharacters = $characters.filter(char => !char.hide_from_dashboard);
  }

  // Reload data when active roster changes
  $: if ($activeRosterId && !loading) {
    calculateGlobalStats(visibleCharacters);
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
    completionStatus: Array<{ content_id: string; is_completed: number }>;
    raidConfigs: Array<{ content_id: string; difficulty: string; take_gold: number }>;
    trackingStatus: Array<{ content_id: string; is_tracked: number }>;
  }> = {};

  // Pre-build a lookup map for RAIDS by id+difficulty to avoid O(n) scans
  const raidLookup: Record<string, typeof RAIDS[0]> = {};
  for (const raid of RAIDS) {
    raidLookup[`${raid.id}-${raid.difficulty}`] = raid;
  }

  // Calculate maximum possible gold income based on conf_raid data
  function calculateTotalEstimatedGold(
    characters: Character[],
    raidConfigsByCharacter: Record<string, RaidConfigEntry[]>
  ): number {
    let totalGold = 0;
    
    for (const character of characters) {
      if (!character.earns_gold) continue;
      
      try {
        const raidConfigs = raidConfigsByCharacter[String(character.char_id)] || [];
        const goldRaids = raidConfigs.filter(config => config.take_gold === 1);
        const uniqueRaids = new Set<string>();
        
        for (const config of goldRaids) {
          const raidKey = `${config.content_id}-${config.difficulty}`;
          if (!uniqueRaids.has(raidKey)) {
            uniqueRaids.add(raidKey);
            
            const raid = raidLookup[raidKey];
            if (raid) {
              const raidGold = raid.gates.reduce((sum: number, gate) => {
                const gateGold = (gate.tradableGold || 0) + (gate.boundGold || 0);
                const boxPrice = config.buy_box === 1 ? (gate.boxPrice || 0) : 0;
                return sum + gateGold - boxPrice;
              }, 0);
              totalGold += raidGold;
            }
          }
        }
      } catch (error) {
        console.error(`Failed to calculate gold for character ${character.char_id}:`, error);
      }
    }
    
    return totalGold;
  }

  function getClassIcon(classId: string): string {
    const classInfo = GAME_CLASSES[classId];
    return classInfo ? classInfo.iconId : '0';
  }

  function getClassName(classId: string): string {
    const classInfo = GAME_CLASSES[classId];
    return classInfo ? classInfo.displayName : classId;
  }

  // Calculate progress percentage
  async function getGoldProgressPercentage(): Promise<number> {
    if (!goldStats) return 0;
    const dashboardSnapshot = await invoke<DashboardSnapshot>('get_dashboard_snapshot', {
      rosterId: $activeRosterId || null
    });
    const estimatedGold = calculateTotalEstimatedGold(
      dashboardSnapshot.characters?.filter(char => !char.hide_from_dashboard) ?? visibleCharacters,
      dashboardSnapshot.raid_configs_by_character || {}
    );
    if (estimatedGold === 0) return 0;
    return Math.min((goldStats.weekly.totalGold / estimatedGold) * 100, 100);
  }
</script>

<div class="dashboard-container">
  {#if loading}
    <div class="loading-state">
      <div class="loading-spinner"></div>
      <p>Loading characters...</p>
    </div>
  {:else}
    <!-- Progress Banner -->
    <div class="progress-banner">
      <div class="progress-content">
        <div class="progress-info">
          <h3>Weekly Gold Progress</h3>
          <div class="progress-stats">
            <span class="current-gold">{goldStats && goldStats.weekly ? goldStats.weekly.totalGold : 0}</span>
            <span class="separator">/</span>
            <span class="estimated-gold">{estimatedGoldDisplay}</span>
            <span class="gold-label">Gold</span>
          </div>
          <div class="gold-breakdown">
            <div class="gold-type">
              <span class="bound-gold">{goldStats && goldStats.weekly ? goldStats.weekly.boundGold : 0}</span>
              <span class="gold-type-label">Bound</span>
            </div>
            <div class="gold-type">
              <span class="tradable-gold">{goldStats && goldStats.weekly ? goldStats.weekly.tradableGold : 0}</span>
              <span class="gold-type-label">Tradable</span>
            </div>
            {#if goldStats && goldStats.weekly && goldStats.weekly.extraIncomeGold > 0}
              <div class="gold-type extra-income">
                <span class="extra-income-gold">+{goldStats.weekly.extraIncomeGold}</span>
                <span class="gold-type-label">Extra</span>
              </div>
            {/if}
            {#if goldStats && goldStats.weekly && goldStats.weekly.boxPurchaseCost > 0}
              <div class="gold-type box-cost">
                <span class="box-purchase-cost">-{goldStats.weekly.boxPurchaseCost}</span>
                <span class="gold-type-label">Boxes</span>
              </div>
            {/if}
          </div>
        </div>
        <div class="progress-bar-container">
          <div class="progress-bar" style="width: {progressPercentage}%"></div>
          <div class="progress-percentage">{Math.round(progressPercentage)}%</div>
        </div>
      </div>
    </div>

    <!-- Header Stats -->
    <div class="header-stats">
      <div class="stat-card">
        <div class="stat-icon">
          <img src="/images/kazeros-raid.webp" alt="Raids" />
        </div>
        <div class="stat-content">
          <div class="stat-value">{totalRaidsCompleted}/{totalRaidsPossible}</div>
          <div class="stat-label">Raids</div>
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-icon">
          <img src="/images/icons8-last-24-hours-80.png" alt="Dailies" />
        </div>
        <div class="stat-content">
          <div class="stat-value">{totalDailiesCompleted}/{totalDailiesPossible}</div>
          <div class="stat-label">Dailies</div>
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-icon">
          <img src="images/calendar_7743808.png" alt="Weeklies" />
        </div>
        <div class="stat-content">
          <div class="stat-value">{totalWeekliesCompleted}/{totalWeekliesPossible}</div>
          <div class="stat-label">Weeklies</div>
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-icon">
          <img src="/images/gold.png" alt="Gold Earners" />
        </div>
        <div class="stat-content">
          <div class="stat-value">{visibleCharacters.filter(c => c.earns_gold).length}</div>
          <div class="stat-label">Gold Earners</div>
        </div>
      </div>
    </div>

    <!-- Character Cards Grid -->
    <div class="characters-grid">
      {#each Object.entries(charactersByRoster) as [rosterId, rosterCharacters], index}
        <div class="roster-section roster-{rosterId}">
          {#if index > 0 && rosterId !== 'Vaanyar'}
            <div class="roster-separator"></div>
          {/if}
          <h3 class="roster-title">
            {#each $rosters as roster}
              {#if roster.id === rosterId}
                {roster.roster_name}
              {/if}
            {/each}
            <span class="character-count">({rosterCharacters.length})</span>
          </h3>
          
          <div class="characters-list">
            {#each rosterCharacters as character}
              <CharacterCard 
                {character}
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
    padding: 1rem;
    width: 100%;
    max-width: min(calc(100vw - 320px), 1920px);
    margin: 0 auto;
    background: var(--background);
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
  .progress-banner {
    background: linear-gradient(135deg, var(--primary), var(--secondary));
    border-radius: 16px;
    padding: 1.5rem;
    margin-bottom: 2rem;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
    position: relative;
    overflow: hidden;
  }

  .progress-banner::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: linear-gradient(45deg, transparent, rgba(255, 255, 255, 0.1), transparent);
    animation: shimmer 3s linear infinite;
  }

  @keyframes shimmer {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(100%); }
  }

  .progress-content {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 2rem;
    position: relative;
    z-index: 1;
  }

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

  .progress-bar-container {
    flex: 1;
    max-width: 300px;
    height: 12px;
    background: rgba(255, 255, 255, 0.2);
    border-radius: 6px;
    overflow: hidden;
    position: relative;
  }

  .progress-bar {
    height: 100%;
    background: linear-gradient(90deg, #ffd700, #ffed4e);
    border-radius: 6px;
    transition: width 0.5s ease;
    box-shadow: 0 0 10px rgba(255, 215, 0, 0.5);
  }

  .progress-percentage {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    font-size: 10px;
    font-weight: 600;
    color: #f86008;
    text-shadow: 0 0 2px rgba(0, 0, 0, 0.8);
    z-index: 1;
  }

  /* Header Stats */
  .header-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
    margin-bottom: 2rem;
  }

  .stat-card {
    background: var(--surface-variant);
    border-radius: 12px;
    padding: 1.25rem;
    display: flex;
    align-items: center;
    gap: 1rem;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
    transition: transform 0.2s ease, box-shadow 0.2s ease;
  }

  .stat-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.15);
  }

  .stat-icon {
    font-size: 2rem;
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--primary);
    border-radius: 12px;
  }

  .stat-icon img {
    width: 28px;
    height: 28px;
    object-fit: contain;
  }

  .stat-content {
    flex: 1;
  }

  .stat-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--on-surface);
    line-height: 1;
  }

  .stat-label {
    font-size: 0.875rem;
    color: var(--on-surface-variant);
    margin-top: 0.25rem;
  }

  /* Characters Grid */
  .characters-grid {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .characters-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 1rem;
    align-items: start;
    width: 100%;
  }

  .roster-section {
    background: var(--surface-variant);
    border-radius: 12px;
    padding: 1rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    transition: transform 0.2s ease, box-shadow 0.2s ease;
    border: 1px solid rgba(255, 140, 0, 0.3);
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
    margin: 2rem 0;
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
    margin: 0 0 1rem 0;
    color: var(--on-surface-variant);
    font-size: 1.25rem;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding-bottom: 0.75rem;
    border-bottom: 2px solid var(--primary);
  }

  .character-count {
    background: var(--primary);
    color: var(--on-primary);
    padding: 0.25rem 0.5rem;
    border-radius: 12px;
    font-size: 0.875rem;
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
      padding: 0.75rem;
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
      grid-template-columns: repeat(2, 1fr);
    }

    .characters-list {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 480px) {
    .header-stats {
      grid-template-columns: 1fr;
    }
  }
</style>
