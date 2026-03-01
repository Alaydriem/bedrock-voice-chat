#!/usr/bin/env node
/**
 * Patches Apple plist files with version numbers read from tauri.conf.json.
 *
 * Usage:
 *   node patch-apple-plists.js                     # patches default iOS plists
 *   node patch-apple-plists.js --plist <path> ...   # patches only specified files
 *
 * Reads from tauri.conf.json:
 *   CFBundleShortVersionString → version (e.g. "1.0.508")
 *   CFBundleVersion            → bundle.iOS.bundleVersion (e.g. "1001003")
 */

const fs = require('fs');
const path = require('path');

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

const shortVersion = tauriConf.version;
if (!shortVersion) {
  console.error('No version found in tauri.conf.json');
  process.exit(1);
}

const bundleVersion = (tauriConf.bundle.iOS && tauriConf.bundle.iOS.bundleVersion)
  || (tauriConf.bundle.macOS && tauriConf.bundle.macOS.bundleVersion);
if (!bundleVersion) {
  console.error('No bundleVersion found in tauri.conf.json (bundle.iOS.bundleVersion or bundle.macOS.bundleVersion)');
  process.exit(1);
}

console.log(`Apple plist patching from tauri.conf.json:`);
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
