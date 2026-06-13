#!/usr/bin/env node
/**
 * Encodes a semantic version to monotonic 3-component format for mods.
 * Usage: node encode-version.js <version>
 * Output: encoded version string (e.g., "1.0.507")
 *
 * Formula: major.minor.(patch*1000 + channel*100 + prerelease)
 * Channels: 1=alpha, 2=internal, 5=beta, 8=rc, 9=stable
 *
 * Examples:
 *   1.0.0-beta.7     -> 1.0.507
 *   1.0.0-internal.4 -> 1.0.204
 *   1.0.0-alpha.3    -> 1.0.103
 *   1.0.0-rc.2       -> 1.0.802
 *   1.0.0            -> 1.0.900
 *   2.1.3            -> 2.1.3900
 */

const { VersionEncoder } = require('./lib/encode-version');

const version = process.argv[2];
if (!version) {
  console.error('Usage: node encode-version.js <version>');
  process.exit(1);
}

process.stdout.write(VersionEncoder.encode(version).display);
