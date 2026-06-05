<script lang="ts">
  import { characters } from '$lib/store';
  import { loadCharacterProgressionSnapshot, scrapeCharacterProgressionDetails } from '$lib/services/progression';
  import type { EquipmentRow, ProgressionSnapshot } from '$lib/components/progression/types';
  import {
    ACCESSORY_ORDER,
    ARMOR_ORDER,
    SLOT_LABELS,
    engravingNodes,
    equipmentEffects,
    formatEffectValue,
    gemSocketRows,
    gemLevelColor,
    hoverEquipmentEffects,
    lastScrapedLabel,
    qualityColor,
    sortedEngravings,
    visibleEquipmentEffects
  } from '$lib/components/progression/helpers';

  // ── Types ──────────────────────────────────────────────────────────────────
  // ── State ──────────────────────────────────────────────────────────────────
  let selectedCharacterId: number | null = null;
  let snapshot: ProgressionSnapshot | null = null;
  let loadingSnapshot = false;
  let scraping = false;
  let scrapeError: string | null = null;
  let scrapeSuccess: string | null = null;

  $: allCharacters = $characters;

  // ── Helpers ────────────────────────────────────────────────────────────────
  function gemTypeLabel(gemType: string, isBound: boolean): { icon: string; label: string; bound: boolean } {
    if (gemType === 'attack')   return { icon: '⚔', label: 'Atk', bound: isBound };
    if (gemType === 'cooldown') return { icon: '⏱', label: 'CD',  bound: isBound };
    return { icon: '💎', label: gemType, bound: isBound };
  }

  function equipmentBySlot(slot: string): EquipmentRow | null {
    return snapshot?.equipment.find(e => e.slot === slot) ?? null;
  }

  // ── Actions ────────────────────────────────────────────────────────────────
  async function loadSnapshot() {
    if (!selectedCharacterId) return;
    loadingSnapshot = true;
    scrapeError = null;
    try {
      snapshot = await loadCharacterProgressionSnapshot<ProgressionSnapshot>(selectedCharacterId);
    } catch (e) {
      console.error('Failed to load snapshot:', e);
      snapshot = null;
    } finally {
      loadingSnapshot = false;
    }
  }

  async function scrapeCharacter() {
    if (!selectedCharacterId) return;
    const character = allCharacters.find(c => c.char_id === selectedCharacterId);
    if (!character) return;

    scraping = true;
    scrapeError = null;
    scrapeSuccess = null;
    try {
      const result = await scrapeCharacterProgressionDetails({
        characterName: character.char_name,
        characterId: character.char_id,
        rosterName: character.roster_name
      });
      scrapeSuccess = result;
      await loadSnapshot();
    } catch (e: any) {
      scrapeError = typeof e === 'string' ? e : (e?.message ?? 'Unknown error');
    } finally {
      scraping = false;
    }
  }

  function onCharacterChange() {
    snapshot = null;
    scrapeError = null;
    scrapeSuccess = null;
    loadSnapshot();
  }

  // Derived
  $: hasData = snapshot && (
    snapshot.engravings.length > 0 ||
    snapshot.equipment.length > 0 ||
    snapshot.gems.length > 0
  );

  $: lastUpdated = (() => {
    if (!snapshot) return null;
    const all = [
      ...snapshot.engravings.map(e => e.updatedAt),
      ...snapshot.equipment.map(e => e.updatedAt),
      ...snapshot.gems.map(g => g.updatedAt),
    ];
    if (!all.length) return null;
    return Math.max(...all);
  })();
</script>

<div class="planner-container">
  <!-- Header -->
  <div class="planner-header">
    <div class="header-left">
      <h2 class="planner-title">Progression Planner</h2>
      <p class="planner-subtitle">View and track your character's current gear state</p>
    </div>
  </div>

  <!-- Character selector + scrape button -->
  <div class="selector-bar">
    <div class="selector-group">
      <label for="char-select">Character</label>
      <select
        id="char-select"
        bind:value={selectedCharacterId}
        on:change={onCharacterChange}
      >
        <option value={null}>— Select a character —</option>
        {#each allCharacters as c}
          <option value={c.char_id}>
            {c.char_name} &nbsp;·&nbsp; iLvl {c.item_level.toFixed(0)}
          </option>
        {/each}
      </select>
    </div>

    {#if selectedCharacterId}
      <div class="scrape-group">
        {#if lastUpdated}
          <span class="last-scraped">Last scraped: {lastScrapedLabel(lastUpdated)}</span>
        {/if}
        <button
          class="scrape-btn"
          on:click={scrapeCharacter}
          disabled={scraping}
        >
          {#if scraping}
            <span class="spinner"></span> Scraping…
          {:else}
            🔄 Scrape from lostark.bible
          {/if}
        </button>
      </div>
    {/if}
  </div>

  <!-- Feedback banners -->
  {#if scrapeError}
    <div class="banner error">⚠ {scrapeError}</div>
  {/if}
  {#if scrapeSuccess}
    <div class="banner success">✓ {scrapeSuccess}</div>
  {/if}

  <!-- Body -->
  {#if !selectedCharacterId}
    <div class="empty-state">
      <div class="empty-icon">📋</div>
      <h3>Select a character to get started</h3>
      <p>Choose a character from the dropdown above, then click <strong>Scrape from lostark.bible</strong> to load their current gear state.</p>
    </div>

  {:else if loadingSnapshot}
    <div class="loading-state">
      <span class="spinner large"></span>
      <p>Loading character data…</p>
    </div>

  {:else if !hasData}
    <div class="empty-state">
      <div class="empty-icon">🔍</div>
      <h3>No data yet</h3>
      <p>Click <strong>Scrape from lostark.bible</strong> above to fetch this character's engravings, gems, and equipment.</p>
    </div>

  {:else}
    <!-- ── Main content grid ── -->
    <div class="content-grid">

      <!-- Equipment panel -->
      <section class="panel equipment-panel">
        <h3 class="panel-title">⚔ Equipment</h3>
        {#if (snapshot?.equipment ?? []).length > 0}
          <div class="equipment-board">
            <div class="equipment-column">
              {#each ARMOR_ORDER as slot}
                {@const item = equipmentBySlot(slot)}
                {#if item}
                  <div class="equip-card">
                    <div class="equip-main-line">
                      <span class="equip-name">{SLOT_LABELS[item.slot] ?? item.slot}</span>
                      {#if item.enhancementLevel !== null}<span class="honing-inline">+{item.enhancementLevel}</span>{/if}
                      {#if item.tier}<span class="tier-inline">{item.tier}</span>{/if}
                    </div>
                    <div class="equip-meta-line">
                      {#if item.quality !== null}<span class="quality-pill" style="background:{qualityColor(item.quality)}">{item.quality}</span>{/if}
                      {#if item.itemLevel !== null}<span>{item.itemLevel.toFixed(0)}</span>{/if}
                    </div>
                  </div>
                {/if}
              {/each}
            </div>

            <div class="equipment-column">
              {#each ACCESSORY_ORDER as slot}
                {@const item = equipmentBySlot(slot)}
                {#if item}
                  <div class="equip-card accessory-card">
                    <div class="equip-main-line">
                      <span class="equip-name">{SLOT_LABELS[item.slot] ?? item.slot}</span>
                      {#if item.tier}<span class="tier-inline">{item.tier}</span>{/if}
                    </div>
                    {#if visibleEquipmentEffects(item).length > 0}
                      <div class="effect-list">
                        {#each visibleEquipmentEffects(item) as effect}
                          <div class="effect-line" class:grade-blue={effect.grade === 'blue'} class:grade-purple={effect.grade === 'purple'} class:grade-orange={effect.grade === 'orange' || effect.grade === 'legendary'}>
                            <span>{effect.label}</span>
                            <strong>{formatEffectValue(effect.value)}</strong>
                          </div>
                        {/each}
                      </div>
                    {/if}
                    {#if hoverEquipmentEffects(item).length > 0}
                      <div class="accessory-hover-details">
                        {#each hoverEquipmentEffects(item) as effect}
                          <div class="effect-line muted">
                            <span>{effect.label}</span>
                            <strong>{formatEffectValue(effect.value)}</strong>
                          </div>
                        {/each}
                      </div>
                    {/if}
                  </div>
                {/if}
              {/each}
            </div>
          </div>

          {@const bracelet = equipmentBySlot('bracelet')}
          {#if bracelet}
            <div class="bracelet-card">
              <div class="equip-main-line">
                <span class="equip-name">{SLOT_LABELS.bracelet}</span>
                {#if bracelet.tier}<span class="tier-inline">{bracelet.tier}</span>{/if}
              </div>
              {#if equipmentEffects(bracelet).length > 0}
                <div class="effect-list bracelet-effects">
                  {#each equipmentEffects(bracelet) as effect}
                    <div class="effect-line" class:grade-blue={effect.grade === 'blue'} class:grade-purple={effect.grade === 'purple'} class:grade-orange={effect.grade === 'orange' || effect.grade === 'legendary'}>
                      <span>{effect.label}</span>
                      <strong>{formatEffectValue(effect.value)}</strong>
                    </div>
                  {/each}
                </div>
              {:else}
                <p class="empty-msg">No bracelet stats were available in the scraped data.</p>
              {/if}
            </div>
          {/if}
        {/if}
        {#if (snapshot?.equipment ?? []).length === 0}
          <p class="empty-msg">No equipment data scraped yet.</p>
        {/if}
      </section>

      <!-- Engravings panel -->
      <section class="panel">
        <h3 class="panel-title">📚 Engravings</h3>
        <div class="engraving-list">
          {#each sortedEngravings(snapshot?.engravings ?? []) as eng}
            {@const nodes = engravingNodes(eng.booksRead, eng.stoneBonus)}
            <div class="eng-row">
              <div class="eng-header">
                <span class="eng-name">{eng.engravingName}</span>
                <div class="eng-badges">
                  {#if eng.stoneBonus > 0}
                    <span class="stone-badge">+{eng.stoneBonus} stone</span>
                  {/if}
                  <span class="node-badge" class:node-full={nodes >= 4}>{nodes} node{nodes !== 1 ? 's' : ''}</span>
                </div>
              </div>
              <div class="eng-progress-wrap">
                <div
                  class="eng-progress-fill"
                  style="width:{(eng.booksRead / eng.maxBooks) * 100}%"
                ></div>
                <span class="eng-progress-label">{eng.booksRead}/{eng.maxBooks}</span>
              </div>
            </div>
          {/each}
          {#if (snapshot?.engravings ?? []).length === 0}
            <p class="empty-msg">No engraving data scraped yet.</p>
          {/if}
        </div>
      </section>

      <!-- Gems panel -->
      <section class="panel gems-panel">
        <h3 class="panel-title">💎 Gems</h3>
        <div class="gem-socket-board">
          {#each gemSocketRows(snapshot?.gems ?? []) as row, rowIndex}
            <div class="gem-socket-row" class:center-row={rowIndex === 1}>
              {#each row as gem}
                {@const info = gemTypeLabel(gem.gemType, gem.isBound)}
                <div class="gem-card" title={`${gem.gemName} | Slot ${gem.slotIndex + 1}`}>
                  <div class="gem-level-badge" style="color:{gemLevelColor(gem.gemLevel)}">Lv.{gem.gemLevel}</div>
                  <div class="gem-type-icon" title={info.label}>{info.icon}</div>
                  <div class="gem-skill">{gem.skillName}</div>
                  {#if info.bound}<div class="gem-bound">Bound</div>{/if}
                </div>
              {/each}
            </div>
          {/each}
          {#if (snapshot?.gems ?? []).length === 0}
            <p class="empty-msg">No gem data scraped yet.</p>
          {/if}
        </div>
      </section>

      <!-- Ark Grid placeholder -->
      <section class="panel placeholder-panel">
        <h3 class="panel-title">🌟 Ark Grid</h3>
        <div class="placeholder-content">
          <span class="placeholder-icon">🔮</span>
          <p>Ark Grid data coming soon</p>
          <p class="placeholder-sub">Jumper / Order Sun / Chaos Moon points will appear here once scraping support is added.</p>
        </div>
      </section>

    </div>
  {/if}
</div>

<style>
  /* ── Layout ── */
  .planner-container {
    padding: 1.25rem 1.5rem;
    max-width: 1100px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .planner-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
  }

  .planner-title {
    font-size: 1.375rem;
    font-weight: 700;
    margin: 0 0 0.25rem;
    color: var(--md-sys-color-on-surface);
  }

  .planner-subtitle {
    font-size: 0.8125rem;
    color: var(--md-sys-color-on-surface-variant);
    margin: 0;
  }

  /* ── Selector bar ── */
  .selector-bar {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
    background: var(--md-sys-color-surface-container-low, var(--md-sys-color-surface));
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 10px;
    padding: 0.75rem 1rem;
  }

  .selector-group {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .selector-group label {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--md-sys-color-on-surface-variant);
    white-space: nowrap;
  }

  .selector-group select {
    padding: 0.4rem 0.65rem;
    border: 1px solid var(--md-sys-color-outline-variant);
    background: var(--md-sys-color-surface);
    color: var(--md-sys-color-on-surface);
    border-radius: 7px;
    font-size: 0.875rem;
    min-width: 240px;
  }

  .scrape-group {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-left: auto;
  }

  .last-scraped {
    font-size: 0.75rem;
    color: var(--md-sys-color-on-surface-variant);
  }

  .scrape-btn {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.45rem 0.9rem;
    border: 1px solid var(--md-sys-color-primary);
    background: transparent;
    color: var(--md-sys-color-primary);
    font-size: 0.8125rem;
    font-weight: 600;
    border-radius: 7px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
    white-space: nowrap;
  }

  .scrape-btn:hover:not(:disabled) {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
  }

  .scrape-btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  /* ── Banners ── */
  .banner {
    padding: 0.6rem 1rem;
    border-radius: 8px;
    font-size: 0.8125rem;
    font-weight: 500;
  }

  .banner.error {
    background: rgba(239, 68, 68, 0.12);
    color: #ef4444;
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  .banner.success {
    background: rgba(16, 185, 129, 0.12);
    color: #10b981;
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  /* ── Empty / loading states ── */
  .empty-state,
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 320px;
    gap: 0.75rem;
    text-align: center;
    color: var(--md-sys-color-on-surface-variant);
  }

  .empty-icon {
    font-size: 3rem;
    opacity: 0.7;
  }

  .empty-state h3 {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
    margin: 0;
  }

  .empty-state p {
    font-size: 0.875rem;
    max-width: 380px;
    margin: 0;
    line-height: 1.5;
  }

  /* ── Content grid ── */
  .content-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .gems-panel {
    grid-column: 1 / -1;
  }

  .equipment-panel {
    grid-column: 1 / -1;
  }

  /* ── Panel base ── */
  .panel {
    background: var(--md-sys-color-surface);
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 12px;
    padding: 1.125rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .panel-title {
    font-size: 0.9375rem;
    font-weight: 700;
    color: var(--md-sys-color-on-surface);
    margin: 0 0 0.25rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--md-sys-color-outline-variant);
  }

  /* ── Equipment ── */
  .equipment-board {
    display: grid;
    grid-template-columns: minmax(240px, 1fr) minmax(280px, 1.2fr);
    gap: 0.75rem;
  }

  .equipment-column {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .equip-card,
  .bracelet-card {
    position: relative;
    background: var(--md-sys-color-surface-container-low, var(--md-sys-color-surface-variant));
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 8px;
    padding: 0.55rem 0.65rem;
  }

  .bracelet-card {
    margin-top: 0.75rem;
  }

  .equip-main-line {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.35rem;
    min-height: 1.35rem;
  }

  .equip-name {
    color: var(--md-sys-color-on-surface);
    font-size: 0.8125rem;
    font-weight: 700;
  }

  .honing-inline,
  .tier-inline {
    color: var(--md-sys-color-on-surface);
    font-size: 0.75rem;
    font-weight: 700;
  }

  .tier-inline {
    color: var(--md-sys-color-primary);
  }

  .equip-meta-line {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.75rem;
    margin-top: 0.2rem;
  }

  .quality-pill {
    min-width: 1.6rem;
    padding: 0.05rem 0.28rem;
    border-radius: 999px;
    color: #fff;
    font-size: 0.7rem;
    font-weight: 800;
    text-align: center;
  }

  .effect-list {
    display: flex;
    flex-direction: column;
    gap: 0.18rem;
    margin-top: 0.35rem;
  }

  .effect-line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem;
    border-left: 2px solid var(--md-sys-color-outline-variant);
    padding-left: 0.45rem;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
    line-height: 1.25;
  }

  .effect-line strong {
    color: var(--md-sys-color-on-surface);
    font-weight: 700;
    white-space: nowrap;
  }

  .effect-line[class*="grade-"] {
    color: var(--effect-grade-color);
    border-left-color: var(--effect-grade-color);
  }

  .effect-line[class*="grade-"] strong {
    color: var(--effect-grade-color);
  }

  .effect-line.muted {
    border-left-color: var(--md-sys-color-outline-variant);
    opacity: 0.85;
  }

  .accessory-hover-details {
    position: absolute;
    left: 0.65rem;
    right: 0.65rem;
    top: calc(100% - 0.25rem);
    z-index: 5;
    display: none;
    flex-direction: column;
    gap: 0.2rem;
    padding: 0.45rem;
    background: var(--md-sys-color-surface-container-high, var(--md-sys-color-surface));
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 7px;
    box-shadow: var(--app-shadow-md);
  }

  .accessory-card:hover .accessory-hover-details {
    display: flex;
  }

  .effect-line.grade-blue {
    --effect-grade-color: #3b82f6;
  }

  .effect-line.grade-purple {
    --effect-grade-color: #a855f7;
  }

  .effect-line.grade-orange {
    --effect-grade-color: #f59e0b;
  }

  /* ── Engravings ── */
  .engraving-list {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .eng-row {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .eng-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .eng-name {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface);
  }

  .eng-badges {
    display: flex;
    gap: 0.3rem;
    align-items: center;
    flex-shrink: 0;
  }

  .stone-badge {
    font-size: 0.7rem;
    font-weight: 600;
    color: #10b981;
    background: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.3);
    border-radius: 4px;
    padding: 0.05rem 0.3rem;
  }

  .node-badge {
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface-variant);
    background: var(--md-sys-color-surface-variant);
    border-radius: 4px;
    padding: 0.05rem 0.3rem;
  }

  .node-badge.node-full {
    color: #f59e0b;
    background: rgba(245, 158, 11, 0.12);
    border: 1px solid rgba(245, 158, 11, 0.3);
  }

  .eng-progress-wrap {
    position: relative;
    height: 6px;
    background: var(--md-sys-color-surface-variant);
    border-radius: 3px;
    display: flex;
    align-items: center;
  }

  .eng-progress-fill {
    height: 100%;
    background: var(--md-sys-color-primary);
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .eng-progress-label {
    font-size: 0.7rem;
    color: var(--md-sys-color-on-surface-variant);
    margin-left: 0.4rem;
    white-space: nowrap;
  }

  /* ── Gems ── */
  .gem-socket-board {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
  }

  .gem-socket-row {
    display: grid;
    grid-template-columns: repeat(4, minmax(92px, 118px));
    justify-content: center;
    gap: 0.6rem;
    width: 100%;
  }

  .gem-socket-row.center-row {
    grid-template-columns: repeat(3, minmax(92px, 118px));
  }

  .gem-card {
    background: var(--md-sys-color-surface-variant);
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 8px;
    padding: 0.6rem 0.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    text-align: center;
  }

  .gem-level-badge {
    font-size: 0.875rem;
    font-weight: 800;
    line-height: 1;
  }

  .gem-type-icon {
    font-size: 1.1rem;
    line-height: 1;
  }

  .gem-skill {
    font-size: 0.7rem;
    color: var(--md-sys-color-on-surface-variant);
    line-height: 1.3;
    word-break: break-word;
  }

  .gem-bound {
    font-size: 0.65rem;
    color: #ef4444;
    font-weight: 600;
    background: rgba(239, 68, 68, 0.1);
    border-radius: 3px;
    padding: 0.05rem 0.25rem;
  }

  /* ── Placeholder panels ── */
  .placeholder-panel {
    opacity: 0.7;
  }

  .placeholder-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 1.5rem 1rem;
    text-align: center;
    color: var(--md-sys-color-on-surface-variant);
  }

  .placeholder-icon {
    font-size: 2rem;
    opacity: 0.6;
  }

  .placeholder-content p {
    margin: 0;
    font-size: 0.875rem;
    font-weight: 500;
  }

  .placeholder-sub {
    font-size: 0.75rem !important;
    font-weight: 400 !important;
    opacity: 0.8;
  }

  /* ── Misc ── */
  .empty-msg {
    font-size: 0.8125rem;
    color: var(--md-sys-color-on-surface-variant);
    font-style: italic;
    margin: 0;
  }

  .spinner {
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 2px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex-shrink: 0;
  }

  .spinner.large {
    width: 24px;
    height: 24px;
    border-width: 3px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* ── Responsive ── */
  @media (max-width: 720px) {
    .content-grid {
      grid-template-columns: 1fr;
    }

    .equipment-board {
      grid-template-columns: 1fr;
    }

    .gems-panel {
      grid-column: 1;
    }

    .selector-bar {
      flex-direction: column;
      align-items: stretch;
    }

    .scrape-group {
      margin-left: 0;
      flex-wrap: wrap;
    }

    .selector-group select {
      min-width: 100%;
    }
  }
</style>
