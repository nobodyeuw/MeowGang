<script lang="ts">
  import { goto } from '$app/navigation';
  import type { Character } from '$lib/store';
  import { GAME_CLASSES } from '$lib/data/classes';
  import { GAME_TASKS } from '$lib/data/tasks';
  import { RAIDS } from '$lib/data/raids';
  import { activeFilterCharId, activeRosterId } from '$lib/store';
  
  export let character: Character;
  export let classIcon: string = '';
  export let className: string = '';
  export let viewMode: 'cards' | 'compact' = 'cards';
  export let restedValues: Array<{ content_id: string; current_value: number }> = [];
  export let completionStatus: Array<{ content_id: string; is_completed: number; details?: string | null; session_id?: string | null; }> = [];
  export let raidConfigs: Array<{ content_id: string; gate?: string; difficulty: string; take_gold: number; is_tracked?: number }> = [];
  export let trackingStatus: Array<{ content_id: string; is_tracked: number; lazy_daily?: number }> = [];

  // Reactive values
  $: classInfo = GAME_CLASSES[character.class_id];
  $: displayName = className || (classInfo ? classInfo.displayName : "Unknown Class");
  $: iconId = classIcon || (classInfo ? classInfo.iconId : "0");

  function getCompletionStatus(contentId: string): boolean {
    const completion = completionStatus.find(c => c.content_id === contentId);
    return completion?.is_completed === 1;
  }

  function getDailyIconTitle(contentId: string, completed: boolean, lazyWaiting: boolean): string {
    const label = contentId === 'chaos' ? 'Chaos Dungeon' : 'Guardian Raid';
    if (completed) return `${label}: done`;
    if (lazyWaiting) return `${label}: resting until 20+ rested`;
    return `${label}: available`;
  }

  function getRaidGateProgress(raidId: string, difficulty: string): { completed: number; total: number } {
    const raidDef = RAIDS.find(r => r.id === raidId && r.difficulty === difficulty)
      ?? RAIDS.find(r => r.id === raidId);
    const total = raidDef?.gates.length ?? 0;

    // session_id format: "<raidId>_Gate <N>" (both encounter-sync and manual toggle)
    const completed = completionStatus.filter(c =>
      c.content_id === raidId &&
      c.is_completed === 1 &&
      (c.session_id ?? '').startsWith(raidId + '_') &&
      (c.session_id ?? '').includes('_Gate ')
    ).length;

    return { completed, total };
  }

  function getCompletedRaidDetails(contentId: string): string | undefined {
    const entry = [...completionStatus].reverse().find(c =>
      c.content_id === contentId && c.is_completed === 1 && c.details
    );
    return entry?.details ? normalizeDifficulty(entry.details) : undefined;
  }

  function normalizeDifficulty(difficulty: string): string {
    const normalized = difficulty.trim().toLowerCase();
    if (normalized.includes('hard')) return 'Hard';
    if (normalized.includes('nightmare')) return 'Nightmare';
    if (normalized.includes('solo')) return 'Solo';
    if (normalized.includes('normal')) return 'Normal';
    return difficulty.charAt(0).toUpperCase() + difficulty.slice(1);
  }

  function getRaidMaxIlvl(contentId: string, difficulty: string): number {
    const raid = RAIDS.find(r => r.id === contentId && r.difficulty === difficulty)
      ?? RAIDS.find(r => r.id === contentId);
    return Math.max(...(raid?.gates.map(g => g.minIlvl) || [0]));
  }

  $: trackedRaidIds = new Set(
    trackingStatus
      .filter(t => Number(t.is_tracked) === 1 && RAIDS.some(raid => raid.id === t.content_id))
      .map(t => t.content_id)
  );

  // Group raids by content_id with difficulty and aggregate gate tracking state.
  $: groupedRaids = raidConfigs.reduce((groups: Record<string, any>, raid: any) => {
    const key = raid.content_id;
    if (!groups[key]) {
      groups[key] = {
        content_id: raid.content_id,
        difficulty: raid.difficulty,
        take_gold: Number(raid.take_gold) === 1 ? 1 : 0,
        is_tracked: trackedRaidIds.has(raid.content_id) ? 1 : 0
      };
    } else if (Number(raid.take_gold) === 1) {
      groups[key].take_gold = 1;
    }

    return groups;
  }, {});

  function getRaidDisplayName(contentId: string, difficulty: string): string {
    const raid = RAIDS.find(r => r.id === contentId && r.difficulty === difficulty);
    const raidName = raid ? raid.name : contentId;
    const formattedDifficulty = normalizeDifficulty(difficulty);
    return `${raidName} ${formattedDifficulty}`;
  }

  // Get raids to display
  $: displayRaids = (() => {
    const raids = Object.values(groupedRaids).map((r: any) => {
      const actualDifficulty = getCompletedRaidDetails(r.content_id);
      const plannedDifficulty = normalizeDifficulty(r.difficulty);
      const mismatch = actualDifficulty != null && actualDifficulty !== plannedDifficulty;

      const gateProgress = getRaidGateProgress(r.content_id, r.difficulty);
      const fullyCompleted = gateProgress.total > 0 && gateProgress.completed >= gateProgress.total;

      return {
        ...r,
        isGoldRaid: Number(r.take_gold) === 1 && character.earns_gold,
        isTrackedRaid: Number(r.is_tracked) === 1 && !character.earns_gold,
        completed: fullyCompleted,   // only true when ALL gates done
        gateProgress,
        completionMismatch: mismatch,
        completionTooltip: mismatch
          ? `Planned to run ${plannedDifficulty} mode but finished in ${actualDifficulty} mode`
          : undefined
      };
    });

    if (character.earns_gold) {
      // Gold earners: show gold raids with gold styling
      return raids
        .filter((r: any) => Number(r.take_gold) === 1)
        .slice(0, 3);
    }

    // Non-gold earners: show max 3 tracked raids, sorted by latest/highest required ilvl.
    return raids
      .filter((r: any) => Number(r.is_tracked) === 1)
      .sort((a: any, b: any) => getRaidMaxIlvl(b.content_id, b.difficulty) - getRaidMaxIlvl(a.content_id, a.difficulty))
      .slice(0, 3);
  })();

  $: trackedWeeklyTasks = (() => {
    return ['cube', 'paradise', 'shop', 'guild']
      .filter(contentId => trackingStatus.some(t => t.content_id === contentId && Number(t.is_tracked) === 1))
      .map(contentId => ({
        content_id: contentId,
        name: GAME_TASKS[contentId]?.name ?? contentId,
        completed: completionStatus.some(c => c.content_id === contentId && Number(c.is_completed) === 1)
      }))
      .slice(0, 4);
  })();
  $: displayWeeklyTasks = character.earns_gold || displayRaids.length > 0
    ? []
    : trackedWeeklyTasks.slice(0, 4);
  $: cardTopWeeklyTasks = displayRaids.length > 0
    ? trackedWeeklyTasks
    : displayWeeklyTasks.filter(task => ['shop', 'guild'].includes(task.content_id) || displayWeeklyTasks.length === 1);
  $: cardBodyWeeklyTasks = displayRaids.length === 0
    ? displayWeeklyTasks.filter(task => !['shop', 'guild'].includes(task.content_id) && displayWeeklyTasks.length !== 1)
    : displayWeeklyTasks;
  $: compactWeeklyTasks = !isMinimalCard && displayRaids.length > 0 ? trackedWeeklyTasks : [];
  $: trackedWeeklyTaskCount = trackedWeeklyTasks.length;
  $: hasCompactLabels = displayRaids.length > 0 || displayWeeklyTasks.length > 0;

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

  
  function formatItemLevel(itemLevel: number): string {
    return itemLevel.toFixed(2);
  }

  function formatCombatPower(combatPower: number): string {
    return combatPower.toLocaleString('de-DE', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
      useGrouping: false
    });
  }

  function getClassIconUrl(iconId: string): string {
    return `/images/classes/${iconId}.png`;
  }

  function getTaskIcon(taskId: string): string {
    if (taskId.startsWith('event_')) {
      return '/images/event_quest.webp';
    }

    const iconMap: Record<string, string> = {
      'chaos': '/images/chaos-dungeon.webp',
      'guardian': '/images/guardian.png',
      'cube': '/images/ebony1720.png',
      'paradise': '/images/paradise.webp',
      'shop': '/images/daily.webp',
      'guild': '/images/guild.webp',
    };
    
    return iconMap[taskId] || '/images/daily.webp';
  }

  function getRaidName(contentId: string, difficulty: string): string {
    const raid = RAIDS.find(r => r.id === contentId && r.difficulty === difficulty);
    return raid ? raid.name : contentId;
  }

  function isRaidCompleted(contentId: string): boolean {
    return getCompletionStatus(contentId);
  }
</script>

<div class="character-card"
     class:compact={viewMode === 'compact'}
     class:minimal-card={isMinimalCard}
     class:daily-only-minimal={isDailyOnlyMinimalCard}
     class:gold-earner={character.earns_gold}
     class:non-gold-earner={!character.earns_gold}
     role="button" 
     tabindex="0" 
     on:click={handleCharacterClick} 
     on:keydown={(e) => e.key === 'Enter' && handleCharacterClick()} 
     aria-label={`Select character ${character.char_name}`}>

  {#if viewMode === 'compact'}
    <div
      class="compact-main-row"
      class:no-raids={!hasCompactLabels}
      class:has-dailies={chaosConfigured || guardianConfigured}
      class:has-labels={hasCompactLabels}
      class:has-weeklies={compactWeeklyTasks.length > 0}
    >
      <div class="compact-identity">
        <img
          src={getClassIconUrl(iconId)}
          alt={displayName}
          class="class-icon compact-class-icon"
          on:error={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
        />
        <h4 class="character-name compact-name">{character.char_name}</h4>
      </div>

      <div class="compact-stats">
        <span class="item-level">iLvl {formatItemLevel(character.item_level)}</span>
        <span class="combat-power">CP {formatCombatPower(character.combat_power)}</span>
      </div>

      {#if hasCompactLabels || chaosConfigured || guardianConfigured}
        <div class="compact-daily-icons" aria-label="Daily task status">
          {#if chaosConfigured}
            <span
              class="compact-daily-state"
              class:available={chaosAvailable}
              class:inactive={!chaosAvailable}
              title={chaosIconTitle}
            >
              <span class="compact-daily-icon">
                <img src={getTaskIcon('chaos')} alt="Chaos" />
              </span>
              <span class="compact-daily-progress" aria-hidden="true">
                <span style="width: {chaosRested}%"></span>
              </span>
            </span>
          {:else}
            <span class="compact-daily-state placeholder" aria-hidden="true">
              <span class="compact-daily-icon"></span>
              <span class="compact-daily-progress"><span></span></span>
            </span>
          {/if}
          {#if guardianConfigured}
            <span
              class="compact-daily-state"
              class:available={guardianAvailable}
              class:inactive={!guardianAvailable}
              title={guardianIconTitle}
            >
              <span class="compact-daily-icon">
                <img src={getTaskIcon('guardian')} alt="Guardian" />
              </span>
              <span class="compact-daily-progress" aria-hidden="true">
                <span style="width: {guardianRested}%"></span>
              </span>
            </span>
          {:else}
            <span class="compact-daily-state placeholder" aria-hidden="true">
              <span class="compact-daily-icon"></span>
              <span class="compact-daily-progress"><span></span></span>
            </span>
          {/if}
          {#if isMinimalCard && displayWeeklyTasks.length === 1}
            {@const weeklyTask = displayWeeklyTasks[0]}
            <span
              class="compact-daily-state weekly"
              class:inactive={weeklyTask.completed}
              title={`${weeklyTask.name}: ${weeklyTask.completed ? 'done' : 'open'}`}
            >
              <span class="compact-daily-icon">
                <img src={getTaskIcon(weeklyTask.content_id)} alt={weeklyTask.name} />
              </span>
            </span>
          {/if}
        </div>
      {/if}

      {#if compactWeeklyTasks.length > 0}
        <div class="compact-weekly-icons" aria-label="Weekly task status">
          {#each compactWeeklyTasks as task}
            <span
              class="compact-weekly-state"
              class:inactive={task.completed}
              title={`${task.name}: ${task.completed ? 'done' : 'open'}`}
            >
              <img src={getTaskIcon(task.content_id)} alt={task.name} />
            </span>
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
              class:mismatch={raid.completionMismatch}
              title={raid.completionTooltip ?? ''}
            >
              <div class="raid-content">
                <img src="/images/kazeros-raid.webp" alt="Raid" class="raid-icon">
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
                  <img src="/images/gold.png" alt="Gold" class="gold-icon">
                {/if}
              </div>
            </div>
          {/each}
          {#each displayWeeklyTasks as task}
            <div
              class="raid-item compact-raid weekly-task"
              class:completed={task.completed}
              title={`${task.name}: ${task.completed ? 'done' : 'open'}`}
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
        <div class="class-section">
          <img
            src={getClassIconUrl(iconId)}
            alt={displayName}
            class="class-icon"
            on:error={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
          />
          <div class="character-details">
            <h4 class="character-name">{character.char_name}</h4>
            <div class="character-stats">
              <span class="item-level">iLvl {formatItemLevel(character.item_level)}</span>
              <span class="combat-power">CP {formatCombatPower(character.combat_power)}</span>
            </div>
          </div>
        </div>
            </div>
    </div>

    <!-- Activity Section (Dailies & Events) -->
    <div class="activity-section">
      {#each cardTopWeeklyTasks as weeklyTask}
        <div
          class="activity-item weekly-inline"
          class:inactive={weeklyTask.completed}
          title={`${weeklyTask.name}: ${weeklyTask.completed ? 'done' : 'open'}`}
        >
          <div class="activity-icon">
            <img src={getTaskIcon(weeklyTask.content_id)} alt={weeklyTask.name} class="task-icon" />
          </div>
        </div>
      {/each}
      {#if chaosConfigured}
        <div class="activity-item" class:inactive={!chaosAvailable} title={chaosIconTitle}>
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
        <div class="activity-item" class:inactive={!guardianAvailable} title={guardianIconTitle}>
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
              class:mismatch={raid.completionMismatch}
              title={raid.completionTooltip ?? ''}
            >
              <div class="raid-content">
                <img src="/images/kazeros-raid.webp" alt="Raid" class="raid-icon">
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
                  <img src="/images/gold.png" alt="Gold" class="gold-icon">
                {/if}
              </div>
            </div>
          {/each}
          {#each cardBodyWeeklyTasks as task}
            <div
              class="raid-item weekly-task"
              class:completed={task.completed}
              title={`${task.name}: ${task.completed ? 'done' : 'open'}`}
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
</div>

  
  
<style>
  .character-card {
    box-sizing: border-box;
    background: var(--surface-variant);
    border-radius: 12px;
    padding: 0.8rem;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    cursor: pointer;
    position: relative;
    overflow: hidden;
    border: 2px solid transparent;
    min-height: 124px;
    display: flex;
    flex-direction: column;
  }

  .character-card.gold-earner {
    border-color: #ffd700;
    box-shadow: 0 4px 20px rgba(255, 215, 0, 0.2);
  }

  .character-card.non-gold-earner {
    border-color: rgba(56, 189, 248, 0.45);
    box-shadow: 0 4px 18px rgba(56, 189, 248, 0.12);
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

  .character-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  }

  .character-card.gold-earner:hover {
    box-shadow: 0 8px 24px rgba(255, 215, 0, 0.3);
  }

  .character-card.non-gold-earner:hover {
    box-shadow: 0 8px 24px rgba(56, 189, 248, 0.18);
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
    border-color: rgba(148, 163, 184, 0.28);
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
    background: color-mix(in srgb, var(--surface) 86%, #000000);
    border: 1px solid rgba(255, 255, 255, 0.08);
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
    background: rgba(255, 255, 255, 0.1);
  }

  .compact-daily-progress span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, #38bdf8, #2dd4bf);
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
    width: 22px;
    height: 22px;
    border-radius: 5px;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--surface) 86%, #000000);
    border: 1px solid rgba(148, 163, 184, 0.22);
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
  }

  .class-icon {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.2);
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
    color: rgba(255, 255, 255, 0.7);
    font-size: 0.75rem;
  }

  .combat-power {
    font-weight: 500;
    color: rgba(255, 107, 53, 0.7);
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

  .activity-item {
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
    border: 1px solid rgba(148, 163, 184, 0.28);
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
    background: rgba(255, 255, 255, 0.1);
    border-radius: 2px;
    overflow: hidden;
  }

  .rested-fill {
    height: 100%;
    background: linear-gradient(90deg, #10b981, #34d399);
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
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .raid-item.gold-raid {
    background: linear-gradient(135deg, rgba(255, 215, 0, 0.1), rgba(255, 215, 0, 0.2));
    border: 1px solid rgba(255, 215, 0, 0.3);
    color: #ffd700;
  }

  .raid-item.tracked-raid {
    background: linear-gradient(135deg, rgba(56, 189, 248, 0.1), rgba(45, 212, 191, 0.15));
    border: 1px solid rgba(56, 189, 248, 0.28);
    color: #7dd3fc;
  }

  .raid-item.weekly-task {
    background: linear-gradient(135deg, rgba(148, 163, 184, 0.1), rgba(100, 116, 139, 0.16));
    border: 1px solid rgba(148, 163, 184, 0.24);
    color: #cbd5e1;
  }

  .raid-item.completed {
    opacity: 0.5;
    text-decoration: line-through;
  }

  .raid-item.gold-raid.completed {
    opacity: 0.4;
    text-decoration: line-through;
    color: rgba(255, 215, 0, 0.6);
  }

  .raid-item.tracked-raid.completed {
    opacity: 0.42;
    text-decoration: line-through;
    color: rgba(125, 211, 252, 0.6);
  }

  .raid-item.weekly-task.completed {
    opacity: 0.42;
    text-decoration: line-through;
    color: rgba(203, 213, 225, 0.62);
  }

  .raid-item.mismatch {
    opacity: 0.9;
    background: rgba(239, 68, 68, 0.12);
    color: #b91c1c;
    border: 1px solid rgba(248, 113, 113, 0.35);
    text-decoration: line-through;
    text-decoration-color: #dc2626;
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
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.55);
    white-space: nowrap;
    flex-shrink: 0;
  }
  .gate-progress-partial { background: rgba(255, 255, 255, 0.08); color: rgba(255, 255, 255, 0.55); }
  .gate-progress-done    { background: rgba(255, 255, 255, 0.08); color: rgba(255, 255, 255, 0.55); }

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
</style>
