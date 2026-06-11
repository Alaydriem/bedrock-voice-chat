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
const { VersionEncoder } = require('./lib/encode-version');

const version = process.argv[2];
if (!version) {
  console.error('Usage: node patch-mod-versions.js <version>');
  process.exit(1);
}

// Resolve paths - script can be called from anywhere
const rootDir = path.resolve(__dirname, '../..');
const modsDir = path.join(rootDir, 'mods');

const encoded = VersionEncoder.encode(version);

console.log(`Patching mod files...`);
console.log(`  Semantic version: ${version}`);
console.log(`  Encoded version:  ${encoded.display}`);
console.log(`  Array format:     [${encoded.major}, ${encoded.minor}, ${encoded.encodedPatch}]`);
console.log('');

// 1a. Patch Java gradle.properties (root)
const gradleProps = path.join(modsDir, 'java/gradle.properties');
if (fs.existsSync(gradleProps)) {
  const content = fs.readFileSync(gradleProps, 'utf8');
  const updated = content.replace(/^modVersion\s*=\s*.*/m, `modVersion=${encoded.display}`);
  fs.writeFileSync(gradleProps, updated);
  console.log(`Patched: gradle.properties -> modVersion=${encoded.display}`);
}

// 1b. Patch Fabric gradle.properties (separate Gradle project)
const fabricGradleProps = path.join(modsDir, 'java/fabric/gradle.properties');
if (fs.existsSync(fabricGradleProps)) {
  const content = fs.readFileSync(fabricGradleProps, 'utf8');
  const updated = content.replace(/^modVersion\s*=\s*.*/m, `modVersion=${encoded.display}`);
  fs.writeFileSync(fabricGradleProps, updated);
  console.log(`Patched: fabric/gradle.properties -> modVersion=${encoded.display}`);
}

// 2. Patch BDS package.json (keeps full semantic version for npm compatibility)
const bdsPackage = path.join(modsDir, 'bds/package.json');
if (fs.existsSync(bdsPackage)) {
  const content = JSON.parse(fs.readFileSync(bdsPackage, 'utf8'));
  content.version = version; // Keep semantic version here
  fs.writeFileSync(bdsPackage, JSON.stringify(content, null, 2) + '\n');
  console.log(`Patched: bds/package.json -> version="${version}"`);
}

// 3. BDS manifests are generated as build artifacts by mods/bds/bundle.js,
// which derives the encoded version from bds/package.json at pack time.
// Nothing to patch here.

// 4. Patch Hytale manifest.json
const hytaleManifest = path.join(modsDir, 'java/hytale/src/main/resources/manifest.json');
if (fs.existsSync(hytaleManifest)) {
  const content = JSON.parse(fs.readFileSync(hytaleManifest, 'utf8'));
  content.Version = encoded.display;
  fs.writeFileSync(hytaleManifest, JSON.stringify(content, null, 2) + '\n');
  console.log(`Patched: hytale/manifest.json -> Version="${encoded.display}"`);
}

console.log('');
console.log('All mod files patched successfully.');
