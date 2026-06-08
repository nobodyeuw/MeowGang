# Local Release Checklist

GitHub Actions release builds are intentionally disabled. Build releases locally so the private scraper implementations stay local and do not need to be uploaded as GitHub secrets.

## Before Building

1. Make sure the private scraper files are present locally:

```text
src-tauri/src/market/market_scraper.rs
src-tauri/src/roster/bible_loadout.rs
src-tauri/src/roster/detailed_scraper.rs
src-tauri/src/roster/item_mapping.rs
src-tauri/src/roster/scraper.rs
src-tauri/src/roster/skill_mapping.rs
```

2. Verify version numbers:

```text
package.json
package-lock.json
src-tauri/tauri.conf.json
src-tauri/tauri.windows.conf.json
src-tauri/src/version.rs
latest.json
src-tauri/resources/changelogs.json
```

3. Verify `latest.json` notes and `changelogs.json` are valid JSON.

## Checks

Run from the repo root:

```bash
npm run check
```

Run from `src-tauri`:

```bash
cargo check
```

## Build

Run from the repo root:

```bash
npm run tauri build
```

The Windows installer is created under:

```text
src-tauri/target/release/bundle/nsis/
```

## Publish

1. Create or edit the GitHub release for the version tag, for example `v1.3.3`.
2. Upload the generated installer and updater artifacts manually.
3. Copy the generated updater signature into `latest.json`.
4. Commit and push public release metadata:

```bash
git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/tauri.windows.conf.json src-tauri/src/version.rs latest.json src-tauri/resources/changelogs.json README.md
git commit -m "Release vX.Y.Z"
git push
```

Do not commit private scraper implementations.
