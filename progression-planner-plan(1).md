# Progression Planner — Technical Plan

## Data Sources Explored

### 1. LOA Buddy Market API (loabuddy.pages.dev)
**Endpoint:** `POST https://marketdata-api.yrzhao1068589.workers.dev/v1/prices/latest`  
**Request body:**
```json
{
  "region_slug": "euc",           // "euc" for Europe Central, "nae" for NA East
  "item_slugs": ["adrenaline", "grudge", ...]  // kebab-case item names
}
```
**Response:** Array of `{ item_slug, price, timestamp }`

**Available categories:**
- **Engraving Recipes** (43 items): `adrenaline`, `grudge`, `keen-blunt-weapon`, `raid-captain`, etc.
- **Honing Materials** (36 items): `destiny-destruction-stone`, `destiny-guardian-stone`, `destiny-leapstone`, `great-destiny-leapstone`, etc.
- **Additional Honing Materials**, **Weapon Evolution Materials** — also available

**No auth required.** Simple REST API, can call from Rust backend directly.

### 2. lostark.bible Character Page
**URL pattern:** `https://lostark.bible/character/CE/{character_name}`  
**Data available (HTML parsing):**

| Section | Data | Example from Vaanyar |
|---------|------|---------------------|
| Equipment | Gear pieces with +level, tier, quality score, item level | Head +11 T4, quality 90, ilvl 1730 |
| Engravings | Name + books read (X/20) + stone bonus | Adrenaline 20/20 +1, Grudge 15/20 |
| Gems | Skill gem type (attack/cooldown) + level | Lv. 8 Attack, Lv. 9 Attack |
| Ark Grid | Points per slot, grid type | Jumper 19, Order Sun / Chaos Moon |
| Ark Passive | Evolution/Enlightenment/Leap trees + levels | T1 Crit Lv.10, T3 Zealous Smite Lv.2 |
| Cards | Card set + awakening level | Light of Salvation 6 Set (30-piece) |
| Combat Power | Total + breakdown by category | 3303.36 (Engravings +142.58%) |
| Stats | Item Level, Combat Power, Roster Level | 1740.83 ilvl |

**Protected by Cloudflare.** Must scrape via browser-like request with proper headers (existing scraper pattern already handles this for the roster page).

---

## Architecture Plan

### Phase 1: Market Data (separate `market.db`)

**New file:** `src-tauri/src/market/` module
- `market_database.rs` — Separate SQLite DB manager for `market.db`
- `market_scraper.rs` — Fetches from LOA Buddy API
- `mod.rs` — Module exports

**Schema (`market.db`):**
```sql
-- Market prices cache
CREATE TABLE market_prices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_slug TEXT NOT NULL,
    item_name TEXT NOT NULL,
    category TEXT NOT NULL,        -- 'engraving', 'honing', 'additional_honing', 'weapon_evolution'
    region_slug TEXT NOT NULL,     -- 'euc', 'nae'
    price INTEGER NOT NULL,
    fetched_at INTEGER NOT NULL,   -- unix timestamp
    UNIQUE(item_slug, region_slug)
);

-- Manual price overrides (fallback)
CREATE TABLE manual_price_overrides (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_slug TEXT NOT NULL,
    item_name TEXT NOT NULL,
    category TEXT NOT NULL,
    region_slug TEXT NOT NULL,
    price INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(item_slug, region_slug)
);

-- Settings
CREATE TABLE market_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
-- Default: region_slug = 'euc', auto_refresh_interval = 3600 (seconds)
```

**Rust scraper flow:**
1. Call LOA Buddy API with item slugs per category
2. Insert/update `market_prices` table
3. Fall back to `manual_price_overrides` when API fails or data is stale
4. Expose Tauri commands: `fetch_market_prices`, `get_market_price`, `set_manual_price`, `get_all_prices`

### Phase 2: Character Detail Scraping (extend existing DB)

**New tables in main DB:**
```sql
CREATE TABLE character_engravings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    character_id INTEGER NOT NULL,
    engraving_name TEXT NOT NULL,
    books_read INTEGER NOT NULL DEFAULT 0,  -- e.g. 15 out of 20
    max_books INTEGER NOT NULL DEFAULT 20,
    stone_bonus INTEGER NOT NULL DEFAULT 0, -- from ability stone
    is_manual_entry INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    UNIQUE(character_id, engraving_name)
);

CREATE TABLE character_equipment (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    character_id INTEGER NOT NULL,
    slot TEXT NOT NULL,             -- 'head', 'shoulder', 'chest', 'pants', 'gloves', 'weapon'
    enhancement_level INTEGER,     -- +11, +13, +17
    tier TEXT,                      -- 'T4'
    quality INTEGER,               -- 0-100
    item_level REAL,               -- 1730, 1740
    is_manual_entry INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    UNIQUE(character_id, slot)
);

CREATE TABLE character_gems (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    character_id INTEGER NOT NULL,
    skill_name TEXT NOT NULL,
    gem_type TEXT NOT NULL,         -- 'attack', 'cooldown'
    gem_level INTEGER NOT NULL,
    is_manual_entry INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);

CREATE TABLE progression_goals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    character_id INTEGER NOT NULL,
    goal_type TEXT NOT NULL,        -- 'engraving', 'gear_level', 'gem_level', 'item_level'
    target_name TEXT NOT NULL,      -- e.g. 'Adrenaline', 'Weapon', 'All gems'
    target_value INTEGER NOT NULL,  -- e.g. 20 (books), +19 (enhance), 10 (gem level)
    created_at INTEGER NOT NULL,
    completed_at INTEGER,
    UNIQUE(character_id, goal_type, target_name)
);
```

**Scraper extension:**
- Add `scrape_character_details(character_name)` to existing `HumanizedScraper`
- Parses the character page HTML for engravings, equipment, gems
- Separate rate limit from roster scraping (different endpoint)

### Phase 3: Progression Calculator Service

**New file:** `src-tauri/src/services/progression_service.rs`

**Core logic:**
```
For each goal:
  1. current_state = character_engravings/equipment/gems WHERE character_id
  2. delta = goal.target_value - current_state.value
  3. cost = delta × market_price(item_slug)
  4. weekly_income = avg from gold_logging
  5. estimated_weeks = cost / weekly_income
```

**Example calculation — Engravings:**
- Current: Grudge 15/20 → Goal: Grudge 20/20
- Need: 5 more books
- Market price: 147,000g per book (EU Central)
- Total cost: 5 × 147,000 = 735,000g
- Weekly income: ~50,000g → ~14.7 weeks

### Phase 4: Frontend UI

Replace the current WIP placeholder in `ProgressionPlanner.svelte` with:

1. **Character Selector** — dropdown of characters from existing roster
2. **Current State Panel** — auto-populated from scrape, editable
   - Engravings table (name, books read, stone bonus)
   - Equipment table (slot, +level, quality, ilvl)
   - Gems table (skill, type, level)
3. **Goal Picker** — select category + target
   - "Set Engraving Goal: [Engraving dropdown] → Level [1-20]"
   - "Set Item Level Goal: [target ilvl]"
   - "Set Gem Goal: All gems to Lv. [target]"
4. **Cost Breakdown** — calculated results
   - Items needed × market price = gold cost
   - Gold cost ÷ weekly income = estimated time
   - Manual price override option
5. **Market Data Panel** — current prices with last-updated timestamp + manual override

---

## Implementation Order

| Step | What | Est. Complexity |
|------|------|-----------------|
| 1 | Market DB + API scraper | Medium |
| 2 | Tauri commands for market data | Low |
| 3 | Character detail scraper (extend existing) | High |
| 4 | Character detail DB tables + Tauri commands | Medium |
| 5 | Progression calculator service | Medium |
| 6 | Frontend: Market data display | Medium |
| 7 | Frontend: Character state + goals | High |
| 8 | Frontend: Cost breakdown + time estimate | Medium |

---

## Questions for You

1. **Region default**: Should I default to "euc" (Europe Central) or make it configurable in Settings? (I'd recommend configurable with euc default)
2. **Refresh frequency**: How often should market data auto-refresh? (I'd suggest every hour, with manual refresh button)
3. **Which phase to start with?** I'd suggest Market Data first (simplest, immediately useful), then Character Details, then Calculator.
