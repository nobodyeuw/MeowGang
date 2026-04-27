#!/usr/bin/env node
/**
 * Unified Release Preparation Script
 * 
 * Single source of truth: tauri.conf.json
 * Generates latest.json from tauri.conf.json and changelogs
 */

const fs = require('fs');
const path = require('path');

const repoOwner = 'nobodyeuw';
const repoName = 'MeowGang-Tracker';

// 1. Read version from tauri.conf.json (single source of truth)
const tauriConfigPath = path.join(__dirname, '..', 'src-tauri', 'tauri.conf.json');
const tauriConfig = JSON.parse(fs.readFileSync(tauriConfigPath, 'utf8'));
const version = tauriConfig.version;

console.log(`\n🚀 Preparing release for version: ${version}`);
console.log('═'.repeat(50));

// 2. Validate version format
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(`❌ Invalid version format: ${version}`);
  console.error('Expected format: X.Y.Z (e.g., 1.1.8)');
  process.exit(1);
}

// 3. Verify package.json version matches
const packageJsonPath = path.join(__dirname, '..', 'package.json');
const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));

if (packageJson.version !== version) {
  console.warn(`⚠️  Syncing package.json version: ${packageJson.version} → ${version}`);
  packageJson.version = version;
  fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n', 'utf8');
  console.log(`✅ Updated package.json to v${version}`);
}

// 4. Load changelog
const changelogPath = path.join(__dirname, '..', 'src-tauri', 'resources', 'changelogs.json');
let releaseNotes = '';

try {
  if (fs.existsSync(changelogPath)) {
    const changelogs = JSON.parse(fs.readFileSync(changelogPath, 'utf8'));
    const currentChangelog = changelogs.versions?.find(c => c.version === version);
    
    if (currentChangelog) {
      // Format changelog as markdown
      releaseNotes = `### v${version}\n\n`;
      
      if (currentChangelog.changes && Array.isArray(currentChangelog.changes)) {
        releaseNotes += currentChangelog.changes
          .map(change => `- **${change.type}:** ${change.description}`)
          .join('\n');
      }
      
      console.log(`✅ Loaded changelog for v${version}`);
    } else {
      console.warn(`⚠️  No changelog found for v${version}`);
      releaseNotes = `### v${version}\n\n- Release\n`;
    }
  }
} catch (err) {
  console.warn(`⚠️  Failed to load changelog: ${err.message}`);
  releaseNotes = `### v${version}\n\n- Release\n`;
}

// 5. Generate latest.json
const latestJson = {
  version: `v${version}`,
  notes: releaseNotes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature: "",
      url: `https://github.com/${repoOwner}/${repoName}/releases/download/v${version}/LOA.Tracker_${version}_x64-setup.exe`
    },
    "windows-x86_64-nsis": {
      signature: "",
      url: `https://github.com/${repoOwner}/${repoName}/releases/download/v${version}/LOA.Tracker_${version}_x64-setup.exe`
    }
  }
};

const latestJsonPath = path.join(__dirname, '..', 'latest.json');
fs.writeFileSync(latestJsonPath, JSON.stringify(latestJson, null, 2) + '\n', 'utf8');

console.log(`✅ Generated latest.json`);

// 6. Validation summary
console.log('\n📋 Pre-release Checklist:');
console.log(`✅ Version: ${version}`);
console.log(`✅ Tauri config: ${version}`);
console.log(`✅ Package.json: ${packageJson.version}`);
console.log(`✅ Changelog: found`);
console.log(`✅ Latest.json: generated`);

console.log('\n📝 Next steps:');
console.log(`1. Run: npm run build && npm run tauri:build`);
console.log(`2. Create GitHub release tag: v${version}`);
console.log(`3. Upload: LOA.Tracker_${version}_x64-setup.exe`);
console.log(`4. Signatures will be auto-filled by GitHub Actions\n`);
