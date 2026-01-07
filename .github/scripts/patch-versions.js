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
 * Formula: major * 10000 + minor * 100 + patch
 * For prerelease versions (e.g., 1.0.0-beta.1), uses core version only
 */
function calculateVersionCode(version) {
  const cleanVersion = version.split('-')[0];
  const parts = cleanVersion.split('.').map(Number);
  const [major = 0, minor = 0, patch = 0] = parts;
  return major * 10000 + minor * 100 + patch;
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
