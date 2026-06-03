<script lang="ts">
  import { classAsset, iconAsset } from '$lib/assets';
  import { getClassIcon } from '$lib/components/settings/tracking-settings/helpers';

  export let characters: any[] = [];
  export let hasHiddenTrackingRows = false;
  export let collapseUntrackedRows = false;
  export let onSetCollapseUntrackedRows: (value: boolean) => void;
</script>

<thead>
  <tr class="header-row">
    <th class="sticky-col first-col">
      <div class="matrix-corner-header">
        <span>Tasks/Character</span>
        {#if hasHiddenTrackingRows}
          <button
            type="button"
            class:active={collapseUntrackedRows}
            class="collapse-empty-rows-btn"
            title={collapseUntrackedRows ? 'Show untracked rows' : 'Hide untracked rows'}
            on:click={() => onSetCollapseUntrackedRows(!collapseUntrackedRows)}
          >
            {collapseUntrackedRows ? '+' : '-'}
          </button>
        {/if}
      </div>
    </th>
    {#each characters as char}
      <th class="char-header sticky-col">
        <div class="char-info">
          <img src={classAsset(getClassIcon(char.class_id))} alt="" class="class-icon" />
          <div class="char-name-section">
            <span class="char-name">{char.char_name}</span>
            {#if char.earns_gold}
              <img src={iconAsset('gold.png')} alt="Gold Earner" class="gold-earner-icon" />
            {/if}
          </div>
          <div class="matrix-character-stats">
            <div class="matrix-character-stat-pair">
              <span class="matrix-character-stat-label">iLvl</span>
              <span class="matrix-character-ilvl">{char.item_level.toFixed(0)}</span>
            </div>
            <div class="matrix-character-stat-pair">
              <span class="matrix-character-stat-label cp-label">CP</span>
              <span class="matrix-character-cp">{char.combat_power.toFixed(0)}</span>
            </div>
          </div>
        </div>
      </th>
    {/each}
  </tr>
</thead>

<style>
  .header-row th {
    background: var(--md-sys-color-surface-variant);
    padding: 12px 8px;
    text-align: center;
    border-bottom: 2px solid var(--md-sys-color-outline);
    font-weight: 600;
    color: var(--md-sys-color-on-surface-variant);
    position: sticky;
    top: 0;
    z-index: 20;
  }

  .header-row th.first-col {
    z-index: 30;
  }

  .matrix-corner-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.35rem;
  }

  .collapse-empty-rows-btn {
    width: 1.35rem;
    height: 1.35rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 6px;
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.85rem;
    line-height: 1;
    cursor: pointer;
  }

  .collapse-empty-rows-btn.active {
    border-color: var(--md-sys-color-primary);
    color: var(--md-sys-color-primary);
    background: color-mix(in srgb, var(--md-sys-color-primary) 10%, var(--md-sys-color-surface-container-high));
  }

  .char-header {
    min-width: 120px;
    border-left: 1px solid var(--md-sys-color-outline);
  }

  .char-info {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .class-icon {
    width: 32px;
    height: 32px;
    border-radius: 50%;
  }

  .char-name {
    font-weight: 600;
    font-size: 12px;
    color: var(--md-sys-color-on-surface);
  }

  .char-name-section {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }

  .gold-earner-icon {
    width: 14px;
    height: 14px;
    object-fit: contain;
  }


  .sticky-col {
    position: sticky;
    left: 0;
    z-index: 10;
    background: var(--md-sys-color-surface);
    box-shadow: 2px 0 0 0 var(--md-sys-color-outline);
  }

  .first-col {
    z-index: 20;
    min-width: var(--task-column-width);
    background: var(--md-sys-color-surface-variant);
    box-shadow: 2px 0 0 0 var(--md-sys-color-outline);
  }

  .char-header.sticky-col {
    background: var(--md-sys-color-surface-variant);
    z-index: 15;
    top: 0;
  }

  @media (max-width: 768px) {
    .char-header {
      min-width: 100px;
    }
  }
</style>
