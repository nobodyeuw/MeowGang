<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { characters, activeRosterId } from '$lib/store';

  // Character detail state
  let selectedCharacterId: number | null = null;
  let characterProgression: any = null;
  let loadingProgression = false;
  let scrapingProgression = false;

  // Reactive values - show all characters, not just from active roster
  $: allCharacters = $characters;

  async function loadCharacterProgression() {
    if (!selectedCharacterId) return;
    loadingProgression = true;
    try {
      characterProgression = await invoke('get_character_progression_snapshot', {
        characterId: selectedCharacterId
      });
    } catch (error) {
      console.error('Failed to load character progression:', error);
      characterProgression = null;
    } finally {
      loadingProgression = false;
    }
  }

  async function scrapeCharacterProgression() {
    if (!selectedCharacterId) return;
    const character = allCharacters.find(c => c.char_id === selectedCharacterId);
    if (!character) return;

    scrapingProgression = true;
    try {
      const result = await invoke('scrape_character_details', {
        request: {
          characterName: character.char_name,
          characterId: character.char_id,
          rosterName: character.roster_name
        }
      });
      console.log('Scraped character details:', result);
      // Reload progression after scraping
      await loadCharacterProgression();
    } catch (error) {
      console.error('Failed to scrape character details:', error);
    } finally {
      scrapingProgression = false;
    }
  }

  function selectCharacter(characterId: number) {
    selectedCharacterId = characterId;
    loadCharacterProgression();
  }
</script>

<div class="planner-container">
  <div class="planner-header">
    <h2 class="planner-title">Progression Planner</h2>
    <p class="planner-subtitle">Set progression goals and track your character's progress</p>
  </div>

  <div class="character-details-section">
    <div class="character-selector">
      <label for="character-select">Select Character:</label>
      <select id="character-select" bind:value={selectedCharacterId} on:change={loadCharacterProgression}>
        <option value="">-- Choose a character --</option>
        {#each allCharacters as character}
          <option value={character.char_id}>{character.char_name} (iLvl {character.item_level.toFixed(2)})</option>
        {/each}
      </select>
    </div>

    {#if !selectedCharacterId}
      <div class="in-progress-state">
        <div class="in-progress-content">
          <div class="in-progress-icon">📋</div>
          <h3>Progression Planner</h3>
          <p>Select a character to get started with the progression planner. You can:</p>
          <ul>
            <li>View your current engravings, gems, and equipment</li>
            <li>Scrape the latest data from lostark.bible</li>
            <li>Track your progression goals</li>
          </ul>
          <p class="select-prompt">👆 Choose a character from the dropdown above to begin</p>
        </div>
      </div>
    {:else if loadingProgression}
      <div class="loading-state">
        <span class="spinner large"></span>
        <p>Loading character details...</p>
      </div>
    {:else}
      <div class="in-progress-state">
        <div class="in-progress-content">
          <div class="in-progress-icon">🔨</div>
          <h3>Coming Soon</h3>
          <p>The detailed progression planner is currently under development.</p>
          <p class="select-prompt">Check back soon for planning features!</p>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .planner-container {
    padding: 1.5rem;
    max-width: 1000px;
    margin: 0 auto;
  }

  .planner-header {
    margin-bottom: 1.5rem;
  }

  .planner-title {
    font-size: 1.5rem;
    font-weight: 600;
    margin: 0 0 0.5rem 0;
    color: var(--md-sys-color-on-surface);
  }

  .planner-subtitle {
    font-size: 0.875rem;
    color: var(--md-sys-color-on-surface-variant);
    margin: 0;
  }

  .character-details-section {
    background: var(--md-sys-color-surface);
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 12px;
    padding: 1.5rem;
  }

  .character-selector {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.5rem;
    padding-bottom: 1.5rem;
    border-bottom: 1px solid var(--md-sys-color-outline-variant);
  }

  .character-selector label {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--md-sys-color-on-surface);
  }

  .character-selector select {
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    border-radius: 8px;
    font-size: 0.875rem;
    min-width: 250px;
  }

  .refresh-btn {
    padding: 0.45rem 0.9rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    font-size: 0.75rem;
    font-weight: 500;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .refresh-btn:hover:not(:disabled) {
    background: var(--md-sys-color-surface-variant);
  }

  .refresh-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .planner-split-view {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2rem;
    margin-top: 1.5rem;
  }

  .planner-left,
  .planner-right {
    background: var(--md-sys-color-surface);
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 12px;
    padding: 1.5rem;
  }

  .planner-left h3,
  .planner-right h3 {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
    margin: 0 0 1rem 0;
  }

  .progression-content {
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }

  .progression-section h4 {
    font-size: 1rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
    margin: 0 0 0.75rem 0;
  }

  .goals-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 200px;
    color: var(--md-sys-color-on-surface-variant);
    font-style: italic;
  }

  .engravings-grid,
  .gems-grid,
  .equipment-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 0.75rem;
  }

  .engraving-item,
  .gem-item,
  .equipment-item {
    background: var(--md-sys-color-surface);
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .engraving-name,
  .gem-name,
  .equipment-slot {
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
    font-size: 0.875rem;
  }

  .engraving-level,
  .gem-info,
  .enhancement,
  .tier,
  .quality,
  .item-level {
    font-size: 0.8125rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .stone-bonus {
    font-size: 0.8125rem;
    color: #10b981;
    font-weight: 500;
  }

  .empty-message {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.875rem;
    font-style: italic;
  }

  .loading-state, .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1rem;
    gap: 0.75rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .in-progress-state {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 400px;
    padding: 2rem;
  }

  .in-progress-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    max-width: 500px;
    gap: 1.5rem;
  }

  .in-progress-icon {
    font-size: 3.5rem;
    opacity: 0.8;
  }

  .in-progress-content h3 {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
    margin: 0;
  }

  .in-progress-content p {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.95rem;
    margin: 0;
    line-height: 1.5;
  }

  .in-progress-content ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    text-align: left;
    background: var(--md-sys-color-surface-variant);
    border-radius: 8px;
    padding: 1rem;
  }

  .in-progress-content li {
    font-size: 0.9rem;
    color: var(--md-sys-color-on-surface-variant);
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .in-progress-content li::before {
    content: '✓';
    color: var(--md-sys-color-primary);
    font-weight: bold;
  }

  .select-prompt {
    font-weight: 500;
    color: var(--md-sys-color-on-surface);
    margin: 0.5rem 0 0 0 !important;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--md-sys-color-outline-variant);
    border-top-color: var(--md-sys-color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .spinner.large {
    width: 24px;
    height: 24px;
    border-width: 3px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @media (max-width: 768px) {
    .planner-split-view {
      grid-template-columns: 1fr;
    }

    .character-selector {
      flex-direction: column;
      align-items: stretch;
    }

    .character-selector select {
      min-width: 100%;
    }
  }
</style>
