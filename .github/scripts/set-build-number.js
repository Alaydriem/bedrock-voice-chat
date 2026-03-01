#!/usr/bin/env node
/**
 * Sets the build number (versionCode / CFBundleVersion) across platform configs.
 * Does NOT touch display versions (CFBundleShortVersionString, version, versionName).
 *
 * Usage: node set-build-number.js <build_number>
 *
 * Files patched:
 * - client/src-tauri/tauri.conf.json (versionCode, bundleVersion)
 * - client/src-tauri/Info.ios.plist (CFBundleVersion)
 */

const fs = require('fs');
const path = require('path');

const buildNumber = parseInt(process.argv[2]);
if (isNaN(buildNumber) || buildNumber <= 0) {
  console.error('Usage: node set-build-number.js <build_number>');
  console.error('  build_number must be a positive integer');
  process.exit(1);
}

const rootDir = path.resolve(__dirname, '../..');

// 1. Patch tauri.conf.json
const confPath = path.join(rootDir, 'client/src-tauri/tauri.conf.json');
if (!fs.existsSync(confPath)) {
  console.error(`File not found: ${confPath}`);
  process.exit(1);
}

const conf = JSON.parse(fs.readFileSync(confPath, 'utf8'));

conf.bundle.android.versionCode = buildNumber;
if (!conf.bundle.iOS) conf.bundle.iOS = {};
conf.bundle.iOS.bundleVersion = String(buildNumber);
if (!conf.bundle.macOS) conf.bundle.macOS = {};
conf.bundle.macOS.bundleVersion = String(buildNumber);

fs.writeFileSync(confPath, JSON.stringify(conf, null, 2) + '\n');
console.log(`Patched: ${confPath}`);
console.log(`  android.versionCode = ${buildNumber}`);
console.log(`  iOS.bundleVersion   = "${buildNumber}"`);
console.log(`  macOS.bundleVersion = "${buildNumber}"`);

// 2. Patch Info.ios.plist (CFBundleVersion only)
const iosPlistPath = path.join(rootDir, 'client/src-tauri/Info.ios.plist');
if (fs.existsSync(iosPlistPath)) {
  let plist = fs.readFileSync(iosPlistPath, 'utf8');
  plist = plist.replace(
    /(<key>CFBundleVersion<\/key>\s*<string>)[^<]*/,
    `$1${buildNumber}`
  );
  fs.writeFileSync(iosPlistPath, plist);
  console.log(`Patched: ${iosPlistPath}`);
  console.log(`  CFBundleVersion = "${buildNumber}"`);
} else {
  console.log(`Skipping (not found): ${iosPlistPath}`);
}

console.log(`\nBuild number ${buildNumber} applied.`);
