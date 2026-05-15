#!/usr/bin/env node
// Usage: node scripts/bump-version.js <new-version>
//
// Updates version in:
//   - package.json
//   - src-tauri/tauri.conf.json
//   - latest.json

const fs = require('fs');
const path = require('path');

const newVersion = process.argv[2];
if (!newVersion) {
  console.error('Usage: node scripts/bump-version.js <new-version>');
  console.error('Example: node scripts/bump-version.js 1.0.17');
  process.exit(1);
}

if (!/^\d+\.\d+\.\d+$/.test(newVersion)) {
  console.error(`Invalid semver: "${newVersion}". Expected format: X.Y.Z`);
  process.exit(1);
}

const root = path.resolve(__dirname, '..');

const files = [
  { rel: 'package.json', key: 'version' },
  { rel: 'src-tauri/tauri.conf.json', key: 'version' },
  { rel: 'latest.json', key: 'version' },
];

for (const { rel, key } of files) {
  const filePath = path.join(root, rel);
  if (!fs.existsSync(filePath)) {
    console.warn(`  SKIP  ${rel} (not found)`);
    continue;
  }
  const json = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
  const oldVersion = json[key];
  json[key] = newVersion;
  fs.writeFileSync(filePath, JSON.stringify(json, null, 2) + '\n');
  console.log(`  ${rel}: ${oldVersion} -> ${newVersion}`);
}

console.log(`\nVersion bumped to ${newVersion}`);
