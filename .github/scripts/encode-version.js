#!/usr/bin/env node
/**
 * Encodes a semantic version to monotonic 3-component format for mods.
 * Usage: node encode-version.js <version>
 * Output: encoded version string (e.g., "1.0.507")
 *
 * Formula: major.minor.(patch*1000 + channel*100 + prerelease)
 * Channels: 1=alpha, 5=beta, 8=rc, 9=stable
 *
 * Examples:
 *   1.0.0-beta.7  -> 1.0.507
 *   1.0.0-alpha.3 -> 1.0.103
 *   1.0.0-rc.2    -> 1.0.802
 *   1.0.0         -> 1.0.900
 *   2.1.3         -> 2.1.3900
 */

const version = process.argv[2];
if (!version) {
  console.error('Usage: node encode-version.js <version>');
  process.exit(1);
}

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
process.stdout.write(`${major}.${minor}.${encodedPatch}`);
