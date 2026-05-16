<script lang="ts">
  import { goto } from '$app/navigation';
  import type { Character } from '$lib/store';
  import { GAME_CLASSES } from '$lib/data/classes';
  import { RAIDS } from '$lib/data/raids';
  import { activeFilterCharId, activeRosterId } from '$lib/store';
  
  export let character: Character;
  export let classIcon: string = '';
  export let className: string = '';
  export let restedValues: Array<{ content_id: string; current_value: number }> = [];
  export let completionStatus: Array<{ content_id: string; is_completed: number; details?: string | null }> = [];
  export let raidConfigs: Array<{ content_id: string; difficulty: string; take_gold: number; is_tracked?: number }> = [];
  export let trackingStatus: Array<{ content_id: string; is_tracked: number }> = [];

  // Reactive values
  $: classInfo = GAME_CLASSES[character.class_id];
  $: displayName = className || (classInfo ? classInfo.displayName : "Unknown Class");
  $: iconId = classIcon || (classInfo ? classInfo.iconId : "0");

  function getRestedValue(contentId: string): number {
    const rested = restedValues.find(r => r.content_id === contentId);
    return rested?.current_value || 0;
  }

  function getCompletionStatus(contentId: string): boolean {
    const completion = completionStatus.find(c => c.content_id === contentId);
    return completion?.is_completed === 1;
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

  // Group raids by content_id with difficulty
  $: groupedRaids = raidConfigs.reduce((groups: Record<string, any>, raid: any) => {
    const key = raid.content_id;
    if (!groups[key]) {
      groups[key] = {
        content_id: raid.content_id,
        difficulty: raid.difficulty,
        take_gold: raid.take_gold,
        is_tracked: raid.is_tracked
      };
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
      const completed = getCompletionStatus(r.content_id);
      const actualDifficulty = getCompletedRaidDetails(r.content_id);
      const plannedDifficulty = normalizeDifficulty(r.difficulty);
      const mismatch = completed && actualDifficulty && actualDifficulty !== plannedDifficulty;
      return {
        ...r,
        isGoldRaid: r.take_gold === 1,
        completed,
        completionMismatch: mismatch,
        completionTooltip: mismatch ? `Planned to run ${plannedDifficulty} mode but finished in ${actualDifficulty} mode` : undefined
      };
    });

    if (character.earns_gold) {
      // Gold earners: show gold raids with gold styling
      return raids
        .filter((r: any) => r.take_gold === 1)
        .slice(0, 3);
    }

    // Non-gold earners: show max 3 tracked raids, sorted by difficulty
    return raids
      .filter((r: any) => r.is_tracked === 1)
      .sort((a: any, b: any) => {
        const raidA = RAIDS.find(r => r.id === a.content_id && r.difficulty === a.difficulty);
        const raidB = RAIDS.find(r => r.id === b.content_id && r.difficulty === b.difficulty);
        const maxIlvlA = Math.max(...(raidA?.gates.map(g => g.minIlvl) || [0]));
        const maxIlvlB = Math.max(...(raidB?.gates.map(g => g.minIlvl) || [0]));
        return maxIlvlB - maxIlvlA;
      })
      .slice(0, 3);
  })();

  // Chaos and Guardian status
  $: chaosRested = restedValues.length > 0 ? getRestedValue('chaos') : 0;
  $: guardianRested = restedValues.length > 0 ? getRestedValue('guardian') : 0;
  $: chaosCompleted = completionStatus.length > 0 ? getCompletionStatus('chaos') : false;
  $: guardianCompleted = completionStatus.length > 0 ? getCompletionStatus('guardian') : false;

  // Check if chaos/guardian are tracked for this character
  $: chaosTracked = trackingStatus.some(t => t.content_id === 'chaos' && t.is_tracked === 1);
  $: guardianTracked = trackingStatus.some(t => t.content_id === 'guardian' && t.is_tracked === 1);

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

  function getClassIconUrl(iconId: string): string {
    return `/images/classes/${iconId}.png`;
  }

  function getTaskIcon(taskId: string): string {
    const iconMap: Record<string, string> = {
      'chaos': '/images/chaos-dungeon.webp',
      'guardian': '/images/guardian.png',
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
     class:gold-earner={character.earns_gold}
     role="button" 
     tabindex="0" 
     on:click={handleCharacterClick} 
     on:keydown={(e) => e.key === 'Enter' && handleCharacterClick()} 
     aria-label={`Select character ${character.char_name}`}>
  
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
            <span class="combat-power">CP {character.combat_power.toLocaleString('de-DE', { useGrouping: false })}</span>
          </div>
        </div>
      </div>
          </div>
  </div>

  <!-- Activity Section (Dailies & Events) -->
  <div class="activity-section">
    {#if chaosTracked}
      <div class="activity-item {chaosCompleted ? 'completed' : ''}">
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
    {#if guardianTracked}
      <div class="activity-item {guardianCompleted ? 'completed' : ''}">
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
  {#if displayRaids.length > 0}
    <div class="raid-section">
      <div class="raid-list">
        {#each displayRaids as raid}
          <div
            class="raid-item {raid.completed ? 'completed' : ''} {raid.isGoldRaid ? 'gold-raid' : ''} {raid.completionMismatch ? 'mismatch' : ''}"
            title={raid.completionTooltip || ''}
          >
            <div class="raid-content">
              <img src="/images/kazeros-raid.webp" alt="Raid" class="raid-icon" />
              <span class="raid-name">{getRaidDisplayName(raid.content_id, raid.difficulty)}</span>
              {#if raid.isGoldRaid}
                <img src="/images/gold.png" alt="Gold" class="gold-icon" />
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}
  </div>

  
  
<style>
  .character-card {
    background: var(--surface-variant);
    border-radius: 12px;
    padding: 1rem;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    cursor: pointer;
    position: relative;
    overflow: hidden;
    border: 2px solid transparent;
    min-height: 140px;
    display: flex;
    flex-direction: column;
  }

  .character-card.gold-earner {
    border-color: #ffd700;
    box-shadow: 0 4px 20px rgba(255, 215, 0, 0.2);
  }

  .character-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  }

  .character-card.gold-earner:hover {
    box-shadow: 0 8px 24px rgba(255, 215, 0, 0.3);
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }

  .character-info {
    flex: 1;
  }

  .class-section {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .class-icon {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.2);
  }

  .character-details {
    flex: 1;
  }

  .character-name {
    margin: 0 0 0.25rem 0;
    color: var(--on-surface);
    font-size: 0.9rem;
    font-weight: 600;
    line-height: 1.2;
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
    gap: 0.5rem;
    align-items: center;
  }

  .activity-item {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    flex: 1;
  }

  .activity-item.completed {
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
    gap: 0.125rem;
  }

  .raid-item {
    padding: 0.25rem 0.5rem;
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

  .raid-item.completed {
    opacity: 0.5;
    text-decoration: line-through;
  }

  .raid-item.gold-raid.completed {
    opacity: 0.4;
    text-decoration: line-through;
    color: rgba(255, 215, 0, 0.6);
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
    align-items: center;
    gap: 0.25rem;
    flex: 1;
  }

  .raid-icon {
    width: 14px;
    height: 14px;
    border-radius: 2px;
  }

  .raid-name {
    flex: 1;
  }

  .gold-icon {
    width: 12px;
    height: 12px;
    border-radius: 2px;
  }

  @media (max-width: 768px) {
    .character-card {
      padding: 0.75rem;
      min-height: 120px;
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
  }
</style>
