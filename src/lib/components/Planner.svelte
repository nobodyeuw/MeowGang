<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { characters } from '$lib/store';

  // ── Types ──────────────────────────────────────────────────────────────────
  interface EngravingRow {
    id: number;
    characterId: number;
    engravingName: string;
    booksRead: number;
    maxBooks: number;
    stoneBonus: number;
    isManualEntry: boolean;
    updatedAt: number;
  }

  interface EquipmentRow {
    id: number;
    characterId: number;
    slot: string;
    enhancementLevel: number | null;
    tier: string | null;
    quality: number | null;
    itemLevel: number | null;
    isManualEntry: boolean;
    updatedAt: number;
  }

  interface GemRow {
    id: number;
    characterId: number;
    slotIndex: number;
    gemName: string;
    skillName: string;
    gemType: string;     // "attack" | "cooldown"
    gemLevel: number;
    isBound: boolean;
    isManualEntry: boolean;
    updatedAt: number;
  }

  interface ProgressionSnapshot {
    characterId: number;
    engravings: EngravingRow[];
    equipment: EquipmentRow[];
    gems: GemRow[];
    goals: any[];
  }

  // ── State ──────────────────────────────────────────────────────────────────
  let selectedCharacterId: number | null = null;
  let snapshot: ProgressionSnapshot | null = null;
  let loadingSnapshot = false;
  let scraping = false;
  let scrapeError: string | null = null;
  let scrapeSuccess: string | null = null;

  $: allCharacters = $characters;

  // ── Helpers ────────────────────────────────────────────────────────────────
  const SLOT_LABELS: Record<string, string> = {
    weapon:        'Weapon',
    head:          'Head',
    chest:         'Chest',
    pants:         'Pants',
    gloves:        'Gloves',
    shoulder:      'Shoulder',
    neck:          'Necklace',
    earring1:      'Earring 1',
    earring2:      'Earring 2',
    ring1:         'Ring 1',
    ring2:         'Ring 2',
    bracelet:      'Bracelet',
    ability_stone: 'Ability Stone',
  };

  const SLOT_ORDER = [
    'weapon','head','chest','pants','gloves','shoulder',
    'neck','earring1','earring2','ring1','ring2','bracelet','ability_stone'
  ];

  const ARMOR_SLOTS  = new Set(['weapon','head','chest','pants','gloves','shoulder']);
  const ACCESSORY_SLOTS = new Set(['neck','earring1','earring2','ring1','ring2','bracelet','ability_stone']);

  function qualityColor(q: number | null): string {
    if (q === null) return 'var(--md-sys-color-outline-variant)';
    if (q >= 90) return '#f59e0b';   // gold
    if (q >= 70) return '#10b981';   // green
    if (q >= 30) return '#3b82f6';   // blue
    return 'var(--md-sys-color-on-surface-variant)';
  }

  function gemTypeLabel(gemType: string, isBound: boolean): { icon: string; label: string; bound: boolean } {
    if (gemType === 'attack')   return { icon: '⚔', label: 'Atk', bound: isBound };
    if (gemType === 'cooldown') return { icon: '⏱', label: 'CD',  bound: isBound };
    return { icon: '💎', label: gemType, bound: isBound };
  }

  function gemLevelColor(level: number): string {
    if (level >= 10) return '#f59e0b';
    if (level >= 8)  return '#a855f7';
    if (level >= 6)  return '#3b82f6';
    return 'var(--md-sys-color-on-surface-variant)';
  }

  function engravingNodes(booksRead: number, stoneBonus: number): number {
    // Each node = 5 books. Stone bonus adds directly to node count.
    return Math.floor(booksRead / 5) + stoneBonus;
  }

  function lastScrapedLabel(updatedAt: number): string {
    if (!updatedAt) return '';
    const d = new Date(updatedAt * 1000);
    return d.toLocaleString();
  }

  function sortedEquipment(equipment: EquipmentRow[]): EquipmentRow[] {
    return [...equipment].sort((a, b) => {
      const ai = SLOT_ORDER.indexOf(a.slot);
      const bi = SLOT_ORDER.indexOf(b.slot);
      return (ai === -1 ? 99 : ai) - (bi === -1 ? 99 : bi);
    });
  }

  function sortedEngravings(engravings: EngravingRow[]): EngravingRow[] {
    return [...engravings].sort((a, b) => (b.booksRead + b.stoneBonus) - (a.booksRead + a.stoneBonus));
  }

  function sortedGems(gems: GemRow[]): GemRow[] {
    return [...gems].sort((a, b) => a.slotIndex - b.slotIndex);
  }

  // ── Actions ────────────────────────────────────────────────────────────────
  async function loadSnapshot() {
    if (!selectedCharacterId) return;
    loadingSnapshot = true;
    scrapeError = null;
    try {
      snapshot = await invoke<ProgressionSnapshot>('get_character_progression_snapshot', {
        characterId: selectedCharacterId
      });
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
      const result = await invoke<string>('scrape_character_details', {
        request: {
          characterName: character.char_name,
          characterId: character.char_id,
          rosterName: character.roster_name
        }
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
      <section class="panel">
        <h3 class="panel-title">⚔ Equipment</h3>
        {#if (snapshot?.equipment ?? []).filter(e => ARMOR_SLOTS.has(e.slot)).length > 0}
          <div class="equip-group-label">Armor &amp; Weapon</div>
          <div class="equipment-list">
            {#each sortedEquipment(snapshot?.equipment ?? []).filter(e => ARMOR_SLOTS.has(e.slot)) as item}
              <div class="equip-row">
                <div class="equip-slot-label">{SLOT_LABELS[item.slot] ?? item.slot}</div>
                <div class="equip-details">
                  <div class="equip-top">
                    {#if item.enhancementLevel !== null}
                      <span class="honing-badge">+{item.enhancementLevel}</span>
                    {/if}
                    {#if item.tier}<span class="tier-badge">{item.tier}</span>{/if}
                    {#if item.itemLevel !== null}<span class="ilvl-text">{item.itemLevel.toFixed(0)} ilvl</span>{/if}
                  </div>
                  {#if item.quality !== null}
                    <div class="quality-bar-wrap">
                      <div class="quality-bar-fill" style="width:{item.quality}%; background:{qualityColor(item.quality)}"></div>
                      <span class="quality-label" style="color:{qualityColor(item.quality)}">{item.quality}</span>
                    </div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
        {#if (snapshot?.equipment ?? []).filter(e => ACCESSORY_SLOTS.has(e.slot)).length > 0}
          <div class="equip-group-label" style="margin-top:0.75rem">Accessories</div>
          <div class="equipment-list">
            {#each sortedEquipment(snapshot?.equipment ?? []).filter(e => ACCESSORY_SLOTS.has(e.slot)) as item}
              <div class="equip-row">
                <div class="equip-slot-label">{SLOT_LABELS[item.slot] ?? item.slot}</div>
                <div class="equip-details">
                  <div class="equip-top">
                    {#if item.tier}<span class="tier-badge">{item.tier}</span>{/if}
                    {#if item.quality !== null}
                      <span class="quality-label" style="color:{qualityColor(item.quality)}">{item.quality} quality</span>
                    {/if}
                  </div>
                </div>
              </div>
            {/each}
          </div>
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
        <div class="gems-grid">
          {#each sortedGems(snapshot?.gems ?? []) as gem}
            {@const info = gemTypeLabel(gem.gemType, gem.isBound)}
            <div class="gem-card" title={gem.gemName}>
              <div class="gem-level-badge" style="color:{gemLevelColor(gem.gemLevel)}">Lv.{gem.gemLevel}</div>
              <div class="gem-type-icon" title={info.label}>{info.icon}</div>
              <div class="gem-skill">{gem.skillName}</div>
              {#if info.bound}<div class="gem-bound">Bound</div>{/if}
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

      <!-- Ark Passive placeholder -->
      <section class="panel placeholder-panel">
        <h3 class="panel-title">✨ Ark Passive</h3>
        <div class="placeholder-content">
          <span class="placeholder-icon">📖</span>
          <p>Ark Passive data coming soon</p>
          <p class="placeholder-sub">Evolution / Enlightenment / Leap trees will appear here.</p>
        </div>
      </section>

      <!-- Cards placeholder -->
      <section class="panel placeholder-panel">
        <h3 class="panel-title">🃏 Cards</h3>
        <div class="placeholder-content">
          <span class="placeholder-icon">🎴</span>
          <p>Card set data coming soon</p>
          <p class="placeholder-sub">Card set name and awakening level will appear here.</p>
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
  .equipment-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .equip-row {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .equip-group-label {
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--md-sys-color-on-surface-variant);
    margin-bottom: 0.35rem;
    opacity: 0.7;
  }

  .equip-slot-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--md-sys-color-on-surface-variant);
    width: 80px;
    flex-shrink: 0;
    padding-top: 0.15rem;
  }

  .equip-details {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .equip-top {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .honing-badge {
    font-size: 0.8125rem;
    font-weight: 700;
    color: #f59e0b;
    background: rgba(245, 158, 11, 0.12);
    border: 1px solid rgba(245, 158, 11, 0.3);
    border-radius: 4px;
    padding: 0.05rem 0.35rem;
  }

  .tier-badge {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--md-sys-color-primary);
    background: rgba(var(--md-sys-color-primary-rgb, 99, 102, 241), 0.1);
    border: 1px solid var(--md-sys-color-outline-variant);
    border-radius: 4px;
    padding: 0.05rem 0.3rem;
  }

  .ilvl-text {
    font-size: 0.8125rem;
    color: var(--md-sys-color-on-surface);
    font-weight: 500;
  }

  .quality-bar-wrap {
    position: relative;
    height: 6px;
    background: var(--md-sys-color-surface-variant);
    border-radius: 3px;
    overflow: visible;
    display: flex;
    align-items: center;
  }

  .quality-bar-fill {
    height: 100%;
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .quality-label {
    font-size: 0.7rem;
    font-weight: 600;
    margin-left: 0.4rem;
    white-space: nowrap;
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
  .gems-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 0.6rem;
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
