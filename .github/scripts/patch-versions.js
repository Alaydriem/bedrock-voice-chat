#!/usr/bin/env node
/**
 * Patches version numbers across BVC server and client files
 * Usage: node patch-versions.js <version>
 *
 * Files patched:
 * - server/server/Cargo.toml
 * - client/src-tauri/Cargo.toml
 * - client/src-tauri/tauri.conf.json
 * - client/package.json
 */

const fs = require('fs');
const path = require('path');

const version = process.argv[2];
if (!version) {
  console.error('Usage: node patch-versions.js <version>');
  process.exit(1);
}

/**
 * Calculate Android versionCode from semantic version
 * Formula: major * 1000000 + minor * 10000 + patch * 1000 + type * 100 + prerelease_num
 *
 * Type values: 1=beta, 2=rc, 3=release
 * This ensures: beta.N < beta.N+1 < release < next_patch.beta.1
 */
function calculateVersionCode(version) {
  const [core, prerelease] = version.split('-');
  const [major = 0, minor = 0, patch = 0] = core.split('.').map(Number);

  let type = 3; // release
  let prereleaseNum = 0;

  if (prerelease) {
    const match = prerelease.match(/^(alpha|beta|rc)\.?(\d+)?$/);
    if (match) {
      const channel = match[1];
      prereleaseNum = parseInt(match[2]) || 1;

      if (channel === 'alpha') type = 0;
      else if (channel === 'beta') type = 1;
      else if (channel === 'rc') type = 2;
    }
  }

  return major * 1000000 + minor * 10000 + patch * 1000 + type * 100 + prereleaseNum;
}

/**
 * Patch Cargo.toml files - updates the version field
 */
function patchCargoToml(filePath, version) {
  if (!fs.existsSync(filePath)) {
    console.error(`File not found: ${filePath}`);
    process.exit(1);
  }
  const content = fs.readFileSync(filePath, 'utf8');
  const updated = content.replace(
    /^version\s*=\s*"[^"]*"/m,
    `version = "${version}"`
  );
  fs.writeFileSync(filePath, updated);
  console.log(`Patched: ${filePath}`);
}

/**
 * Patch tauri.conf.json - updates version and Android versionCode
 */
function patchTauriConf(filePath, version) {
  if (!fs.existsSync(filePath)) {
    console.error(`File not found: ${filePath}`);
    process.exit(1);
  }
  const content = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  content.version = version;
  content.bundle.android.versionCode = calculateVersionCode(version);
  fs.writeFileSync(filePath, JSON.stringify(content, null, 2) + '\n');
  console.log(`Patched: ${filePath} (versionCode: ${content.bundle.android.versionCode})`);
}

/**
 * Patch package.json - updates version field
 */
function patchPackageJson(filePath, version) {
  if (!fs.existsSync(filePath)) {
    console.error(`File not found: ${filePath}`);
    process.exit(1);
  }
  const content = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  content.version = version;
  fs.writeFileSync(filePath, JSON.stringify(content, null, 2) + '\n');
  console.log(`Patched: ${filePath}`);
}

// Main execution
const rootDir = path.resolve(__dirname, '../..');

console.log(`Patching files to version ${version}...`);
console.log(`Android versionCode will be: ${calculateVersionCode(version)}`);
console.log('');

patchCargoToml(path.join(rootDir, 'server/server/Cargo.toml'), version);
patchCargoToml(path.join(rootDir, 'client/src-tauri/Cargo.toml'), version);
patchTauriConf(path.join(rootDir, 'client/src-tauri/tauri.conf.json'), version);
patchPackageJson(path.join(rootDir, 'client/package.json'), version);

console.log('');
console.log(`All files patched to version ${version}`);
