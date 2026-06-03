<script lang="ts">
  import { classAsset, iconAsset } from '$lib/assets';
  import { GAME_CLASSES } from '$lib/data/classes';

  export let characters: any[] = [];
  export let highlightCharId: number | null = null;

  function getClassName(classId: string): string {
    const classInfo = GAME_CLASSES[classId];
    return classInfo ? classInfo.displayName : 'Unknown';
  }

  function getClassIcon(classId: string): string {
    const classInfo = GAME_CLASSES[classId];
    return classInfo ? classInfo.iconId : '0';
  }
</script>

<thead>
  <tr class="header-row">
    <th class="sticky-col first-col">Tasks/Character</th>
    {#each characters as character}
      <th class="char-header sticky-col {character.id === highlightCharId ? 'highlighted' : ''}">
        <div class="char-info">
          <img
            src={classAsset(getClassIcon(character.class))}
            alt={getClassName(character.class)}
            class="class-icon"
            on:error={(event: any) => { event.currentTarget.style.display = 'none'; }}
          />
          <div class="char-name-section">
            <span class="char-name">{character.name}</span>
            {#if character.earns_gold}
              <img src={iconAsset('gold.png')} alt="Gold Earner" class="gold-earner-icon" />
            {/if}
          </div>
          <div class="matrix-character-stats">
            <div class="matrix-character-stat-pair">
              <span class="matrix-character-stat-label">iLvl</span>
              <span class="matrix-character-ilvl">{character.ilvl?.toFixed(0) || '0'}</span>
            </div>
            <div class="matrix-character-stat-pair">
              <span class="matrix-character-stat-label cp-label">CP</span>
              <span class="matrix-character-cp">{character.combat_power?.toFixed(0) || '0'}</span>
            </div>
          </div>
        </div>
      </th>
    {/each}
  </tr>
</thead>

<style>
  .header-row {
    background: var(--md-sys-color-surface-container);
  }

  .header-row th {
    position: sticky;
    top: 0;
    z-index: 20;
    background: var(--md-sys-color-surface-container);
  }

  .header-row th.first-col {
    z-index: 30;
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

  .char-header {
    min-width: 120px;
    text-align: center;
  }

  .char-header.highlighted {
    background: var(--app-color-highlight-surface);
    box-shadow: var(--app-shadow-highlight);
  }

  .char-header.highlighted .char-name {
    color: var(--app-color-highlight-text) !important;
    text-shadow: var(--app-shadow-highlight-text);
  }

  .char-info {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
  }

  .char-name-section {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .class-icon {
    width: 24px;
    height: 24px;
    border-radius: 50%;
  }

  .char-name {
    font-weight: 600;
    font-size: 0.875rem;
  }

  .gold-earner-icon {
    width: 14px;
    height: 14px;
    object-fit: contain;
  }

</style>
