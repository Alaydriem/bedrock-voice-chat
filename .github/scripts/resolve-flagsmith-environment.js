#!/usr/bin/env node
/**
 * Print the Flagsmith environment name for a release version.
 * Usage: node resolve-flagsmith-environment.js <version>
 *
 * The CI build maps the printed environment to the matching
 * client-side SDK key secret and bakes it in as FLAGSMITH_KEY.
 */

const { VersionEncoder } = require('./lib/encode-version');

const version = process.argv[2];
if (!version) {
  console.error('Usage: node resolve-flagsmith-environment.js <version>');
  process.exit(1);
}

process.stdout.write(VersionEncoder.flagsmithEnvironment(version));
