#!/usr/bin/env node
/**
 * Patches Apple plist files with correct version numbers derived from tauri.conf.json.
 *
 * Usage:
 *   node patch-apple-plists.js                     # patches default iOS plists
 *   node patch-apple-plists.js --plist <path> ...   # patches only specified files
 *
 * Sets:
 *   CFBundleShortVersionString → encodeModVersion (e.g. "1.0.507")
 *   CFBundleVersion            → calculateVersionCode (e.g. "1000107")
 */

const fs = require('fs');
const path = require('path');

// ---------------------------------------------------------------------------
// Version encoding (same logic as patch-versions.js)
// ---------------------------------------------------------------------------

/**
 * Calculate Android/Apple versionCode from semantic version
 * Formula: major * 1000000 + minor * 10000 + patch * 1000 + type * 100 + prerelease_num
 * Type values: 0=alpha, 1=beta, 2=rc, 3=release
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
 * Encode semantic version to monotonic 3-component version
 * Formula: major.minor.(patch*1000 + channel*100 + prerelease)
 * Channels: 1=alpha, 5=beta, 8=rc, 9=stable
 */
function encodeModVersion(version) {
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
  return { major, minor, encodedPatch };
}

// ---------------------------------------------------------------------------
// Plist patching
// ---------------------------------------------------------------------------

function patchPlist(filePath, shortVersion, bundleVersion) {
  if (!fs.existsSync(filePath)) {
    console.log(`Skipping (not found): ${filePath}`);
    return;
  }

  let content = fs.readFileSync(filePath, 'utf8');

  // Replace CFBundleShortVersionString value
  content = content.replace(
    /(<key>CFBundleShortVersionString<\/key>\s*<string>)[^<]*/,
    `$1${shortVersion}`
  );

  // Replace CFBundleVersion value
  content = content.replace(
    /(<key>CFBundleVersion<\/key>\s*<string>)[^<]*/,
    `$1${bundleVersion}`
  );

  fs.writeFileSync(filePath, content);
  console.log(`Patched: ${filePath}`);
  console.log(`  CFBundleShortVersionString = ${shortVersion}`);
  console.log(`  CFBundleVersion            = ${bundleVersion}`);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const rootDir = path.resolve(__dirname, '../..');
const tauriConfPath = path.join(rootDir, 'client/src-tauri/tauri.conf.json');

if (!fs.existsSync(tauriConfPath)) {
  console.error(`tauri.conf.json not found at: ${tauriConfPath}`);
  process.exit(1);
}

const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'));
const version = tauriConf.version;

if (!version) {
  console.error('No version found in tauri.conf.json');
  process.exit(1);
}

const encoded = encodeModVersion(version);
const shortVersion = `${encoded.major}.${encoded.minor}.${encoded.encodedPatch}`;
const bundleVersion = String(calculateVersionCode(version));

console.log(`Apple plist patching for version: ${version}`);
console.log(`  CFBundleShortVersionString: ${shortVersion}`);
console.log(`  CFBundleVersion:            ${bundleVersion}`);
console.log('');

// Parse --plist arguments
const args = process.argv.slice(2);
const plistIdx = args.indexOf('--plist');

if (plistIdx !== -1) {
  // Patch only specified files
  const plistPaths = args.slice(plistIdx + 1);
  if (plistPaths.length === 0) {
    console.error('--plist requires at least one file path');
    process.exit(1);
  }
  for (const p of plistPaths) {
    patchPlist(p, shortVersion, bundleVersion);
  }
} else {
  // Default: patch iOS plists
  const iosPlist = path.join(rootDir, 'client/src-tauri/Info.ios.plist');
  const genPlist = path.join(rootDir, 'client/src-tauri/gen/apple/bedrock-voice-chat-client_iOS/Info.plist');

  patchPlist(iosPlist, shortVersion, bundleVersion);
  patchPlist(genPlist, shortVersion, bundleVersion);
}

console.log('');
console.log('Apple plist patching complete.');
