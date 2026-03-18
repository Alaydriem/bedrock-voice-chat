#!/usr/bin/env node
/**
 * Generates the Tauri v2 updater JSON manifest.
 * Usage: node generate-updater-json.js <semver-version> <repo> <tag>
 *
 * Reads .sig files from the release-files directory (RELEASE_FILES_DIR env var)
 * and constructs the updater JSON with download URLs pointing to GitHub Releases.
 *
 * The version in the JSON uses the ENCODED version (from tauri.conf.json)
 * because the Tauri updater compares it against the app's compiled-in version.
 *
 * Output path is controlled by OUTPUT_PATH env var (default: updater.json).
 */

const fs = require('fs');
const path = require('path');

const semverVersion = process.argv[2];
const repo = process.argv[3];
const tag = process.argv[4];

if (!semverVersion || !repo || !tag) {
  console.error('Usage: node generate-updater-json.js <version> <repo> <tag>');
  process.exit(1);
}

/**
 * Encode semantic version to monotonic 3-component version.
 * Formula: major.minor.(patch*1000 + channel*100 + prerelease)
 * Channels: 1=alpha, 5=beta, 8=rc, 9=stable
 *
 * Must match the encoding in patch-versions.js exactly.
 */
function encodeVersion(version) {
  const [core, prerelease] = version.split('-');
  const [major = 0, minor = 0, patch = 0] = core.split('.').map(Number);

  let channel = 9;
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
  return `${major}.${minor}.${encodedPatch}`;
}

const encodedVersion = encodeVersion(semverVersion);
const releaseFilesDir = process.env.RELEASE_FILES_DIR || './release-files';
const baseUrl = `https://github.com/${repo}/releases/download/${tag}`;

function readSig(filename) {
  const sigPath = path.join(releaseFilesDir, filename);
  if (fs.existsSync(sigPath)) {
    return fs.readFileSync(sigPath, 'utf8').trim();
  }
  return null;
}

const platforms = {};

const winSig = readSig('bvc-client-windows-x64-updater.exe.sig');
if (winSig) {
  platforms['windows-x86_64'] = {
    signature: winSig,
    url: `${baseUrl}/bvc-client-windows-x64-updater.exe`
  };
}

const macSig = readSig('bvc-client-macos-arm64-updater.app.tar.gz.sig');
if (macSig) {
  platforms['darwin-aarch64'] = {
    signature: macSig,
    url: `${baseUrl}/bvc-client-macos-arm64-updater.app.tar.gz`
  };
}

if (Object.keys(platforms).length === 0) {
  console.error('No platform signatures found in ' + releaseFilesDir);
  process.exit(1);
}

const manifest = {
  version: encodedVersion,
  notes: `Release ${semverVersion}`,
  pub_date: new Date().toISOString(),
  platforms
};

const output = JSON.stringify(manifest, null, 2) + '\n';
const outputPath = process.env.OUTPUT_PATH || 'updater.json';
fs.writeFileSync(outputPath, output);

console.log(`Generated updater manifest (${semverVersion} -> encoded ${encodedVersion}):`);
console.log(output);
console.log(`Written to ${outputPath}`);
