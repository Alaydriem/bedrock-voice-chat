#!/usr/bin/env node
/**
 * Patches version numbers across all mod files using encoded versioning
 * Usage: node patch-mod-versions.js <version>
 *
 * Encoding: major.minor.(patch*1000 + channel*100 + prerelease)
 * Channels: 1=alpha, 5=beta, 8=rc, 9=stable
 */

const fs = require('fs');
const path = require('path');

const version = process.argv[2];
if (!version) {
  console.error('Usage: node patch-mod-versions.js <version>');
  process.exit(1);
}

/**
 * Encode semantic version to monotonic 3-component version
 * @param {string} version - Semantic version like "1.2.3" or "1.2.3-beta.1"
 * @returns {{ major: number, minor: number, encodedPatch: number, display: string }}
 */
function encodeVersion(version) {
  const [core, prerelease] = version.split('-');
  const [major = 0, minor = 0, patch = 0] = core.split('.').map(Number);

  let channel = 9; // stable
  let prereleaseNum = 0;

  if (prerelease) {
    const match = prerelease.match(/^(alpha|beta|rc)\.?(\d+)?$/);
    if (match) {
      const channelName = match[1];
      prereleaseNum = parseInt(match[2]) || 1;

      if (channelName === 'alpha') channel = 1;
      else if (channelName === 'beta') channel = 5;
      else if (channelName === 'rc') channel = 8;
    }
  }

  const encodedPatch = patch * 1000 + channel * 100 + prereleaseNum;
  const display = `${major}.${minor}.${encodedPatch}`;

  return { major, minor, encodedPatch, display };
}

// Resolve paths - script can be called from anywhere
const rootDir = path.resolve(__dirname, '../..');
const modsDir = path.join(rootDir, 'mods');

const encoded = encodeVersion(version);

console.log(`Patching mod files...`);
console.log(`  Semantic version: ${version}`);
console.log(`  Encoded version:  ${encoded.display}`);
console.log(`  Array format:     [${encoded.major}, ${encoded.minor}, ${encoded.encodedPatch}]`);
console.log('');

// 1. Patch Java gradle.properties
const gradleProps = path.join(modsDir, 'java/gradle.properties');
if (fs.existsSync(gradleProps)) {
  const content = fs.readFileSync(gradleProps, 'utf8');
  const updated = content.replace(/^modVersion\s*=\s*.*/m, `modVersion=${encoded.display}`);
  fs.writeFileSync(gradleProps, updated);
  console.log(`Patched: gradle.properties -> modVersion=${encoded.display}`);
}

// 2. Patch BDS package.json (keeps full semantic version for npm compatibility)
const bdsPackage = path.join(modsDir, 'bds/package.json');
if (fs.existsSync(bdsPackage)) {
  const content = JSON.parse(fs.readFileSync(bdsPackage, 'utf8'));
  content.version = version; // Keep semantic version here
  fs.writeFileSync(bdsPackage, JSON.stringify(content, null, 2) + '\n');
  console.log(`Patched: bds/package.json -> version="${version}"`);
}

// 3. Patch BDS manifest.json (uses encoded array)
const bdsManifest = path.join(modsDir, 'bds/manifest.json');
if (fs.existsSync(bdsManifest)) {
  const content = JSON.parse(fs.readFileSync(bdsManifest, 'utf8'));
  content.header.version = [encoded.major, encoded.minor, encoded.encodedPatch];
  fs.writeFileSync(bdsManifest, JSON.stringify(content, null, 2) + '\n');
  console.log(`Patched: bds/manifest.json -> version=[${encoded.major}, ${encoded.minor}, ${encoded.encodedPatch}]`);
}

console.log('');
console.log('All mod files patched successfully.');
