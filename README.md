# <img src="static/images/LOAtracker_appicon.png" alt="MeowGang Tracker icon" width="36" align="left" /> MeowGang Tracker

**A lightweight desktop companion for Lost Ark roster, raid, gold, and friend availability tracking.**

[![Version](https://img.shields.io/badge/version-1.3.1-blue.svg)](https://github.com/nobodyeuw/MeowGang/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Platform](https://img.shields.io/badge/platform-Windows-blue.svg)](#)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri-lightgrey.svg)](https://tauri.app/)

MeowGang Tracker helps reduce the manual work of managing Lost Ark rosters. It tracks characters, raid configuration, weekly gold, daily and weekly tasks, LOA Logs completion data, and MeowConnect friend availability in one local-first desktop app.

[Download Latest Release](https://github.com/nobodyeuw/MeowGang/releases) | [Report Bug](https://github.com/nobodyeuw/MeowGang/issues)

---

## Highlights

| Feature | Details |
| :--- | :--- |
| **Dashboard** | View roster status, tracked raids, weekly gold progress, and MeowConnect status at a glance. |
| **To Do** | Track daily, weekly, roster, and raid-gate completion states with configurable raid settings. |
| **MeowConnect** | Share selected character and raid completion data with accepted friends through the MeowGangConnect Supabase backend. |
| **Raid Together** | See how many matching open runs you and each friend can still run for selected raids. |
| **LOA Logs Integration** | Encounter clears can update completion status and sync to MeowConnect when enabled. |
| **Local First** | Core roster, tracking, gold, and settings data are stored locally in SQLite. |
| **Tauri Desktop App** | Built with Tauri 2, SvelteKit, TypeScript, and Rust for a small native Windows app. |

---

## MeowConnect

MeowConnect replaces the old Party Planner flow. Instead of Google Sheets and Apps Script groups, it uses Discord login plus Supabase row-level security to share availability only between accepted friends.

Users can:

- opt in or disable MeowConnect locally
- choose which characters are shared
- sync selected character data and raid completions
- add friends by whitelist name/Discord identity
- mark favorites locally
- mark characters as reserved for static or friend runs per raid and difficulty
- view recent clear logs from LOA Logs and manual completion tracking

The old Party Planner code and Apps Script template have been removed. Existing `party_plans.json` files are ignored as legacy data.

---

## Under Development

### Progression Planner

The Progression Planner is still under active development. Parts of the feature may be incomplete, hidden, or change between releases while market price support and character progression data are refined.

Stable areas of the app are the dashboard, roster setup, tracking, raid configuration, gold progress, LOA Logs integration, and MeowConnect.

---

## Quick Start

### Installation

1. Download the installer from the [Releases](https://github.com/nobodyeuw/MeowGang/releases) page.
2. Run the installer and launch the app.
3. Sign in with Discord when prompted so the app can verify whitelist access.
4. Add a roster character, configure raids, and enable MeowConnect if you want friend availability sharing.

### Development

```bash
# Clone and install
git clone https://github.com/nobodyeuw/MeowGang.git
cd MeowGang
npm install

# Run the Tauri app in development
npm run tauri dev

# Build production installer
npm run tauri build
```

### Prerequisites

- Node.js 18+
- Rust 1.70+
- Git
- Tauri prerequisites for Windows

### Useful Commands

```bash
npm run build          # Build frontend only
npm run dev            # Frontend dev server only
npm run check          # Svelte/TypeScript diagnostics
cargo check            # Check Rust code from src-tauri
cargo fmt              # Format Rust code from src-tauri
```

### Discord Auth

The desktop app uses Discord OAuth with Authorization Code + PKCE. No Discord client secret is shipped in the app.

The Discord application must allow these local redirect URIs:

```text
http://127.0.0.1:53682/discord/callback
http://127.0.0.1:53682/supabase/callback
```

For local development, whitelist access still requires `DISCORD_WHITELIST_URL` to point at the configured whitelist source.

---

## Privacy & Safety

- Local app data: core roster, tracking, gold, and settings data stay on the user's machine.
- MeowConnect: only explicitly enabled characters and completion/reservation data are uploaded, and only accepted friends can read shared data through Supabase RLS.
- Discord: login is used for whitelist verification and MeowConnect identity.
- Secrets: never commit Supabase service-role keys, Discord client secrets, local `.env` files, or local app data files.
- Third-party notice: this is a fan project and is not affiliated with Smilegate RPG or Amazon Games.

## License

Distributed under the MIT License. See `LICENSE` for more information.

## Credits

Made with ♥ for MeowGang.
