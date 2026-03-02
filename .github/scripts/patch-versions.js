#!/usr/bin/env node
/**
 * Patches version numbers across BVC server and client files
 * Usage: node patch-versions.js <version>
 *
 * Files patched:
 * - server/server/Cargo.toml
 * - client/src-tauri/Cargo.toml
 * - client/src-tauri/tauri.conf.json (version, versionCode, bundleVersion)
 * - client/src-tauri/Info.ios.plist (CFBundleShortVersionString, CFBundleVersion)
 * - client/package.json
 * - mods/bds/package.json
 * - mods/bds/manifest.json
 */

const fs = require('fs');
const path = require('path');

const version = process.argv[2];
if (!version) {
  console.error('Usage: node patch-versions.js <version>');
  process.exit(1);
}

/**
 * Encode semantic version to monotonic 3-component version
 * Formula: major.minor.(patch*1000 + channel*100 + prerelease)
 * Channels: 1=alpha, 5=beta, 8=rc, 9=stable
 *
 * This is the canonical encoding used by mods, Apple CFBundleShortVersionString,
 * and (when flattened) Android versionCode and Apple CFBundleVersion.
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

/**
 * Flatten the mod encoding into a single integer for Android versionCode / Apple CFBundleVersion
 * Formula: major * 1000000 + minor * 10000 + encodedPatch
 *
 * Uses the same channel values as encodeModVersion (1=alpha, 5=beta, 8=rc, 9=stable).
 * Example: 1.0.0-beta.8 → encodeModVersion gives 1.0.508 → flattened: 1000508
 */
function calculateVersionCode(version) {
  const { major, minor, encodedPatch } = encodeModVersion(version);
  return major * 1000000 + minor * 10000 + encodedPatch;
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
 * Patch tauri.conf.json - updates version, versionCode, and bundleVersion
 *
 * The `version` field is set to the encoded mod version (e.g. "1.0.508") instead
 * of the raw semver (e.g. "1.0.0-beta.8") because Tauri uses this field directly
 * for CFBundleShortVersionString on Apple platforms. Semver prerelease tags get
 * stripped and mangled by Tauri's xcode-script, so we must provide a clean
 * 3-component version.
 */
function patchTauriConf(filePath, version) {
  if (!fs.existsSync(filePath)) {
    console.error(`File not found: ${filePath}`);
    process.exit(1);
  }
  const content = JSON.parse(fs.readFileSync(filePath, 'utf8'));

  const encoded = encodeModVersion(version);
  const displayVersion = `${encoded.major}.${encoded.minor}.${encoded.encodedPatch}`;

  content.version = displayVersion;
  content.bundle.android.versionCode = calculateVersionCode(version);

  const bundleVersion = String(calculateVersionCode(version));
  if (!content.bundle.iOS) content.bundle.iOS = {};
  content.bundle.iOS.bundleVersion = bundleVersion;
  if (!content.bundle.macOS) content.bundle.macOS = {};
  content.bundle.macOS.bundleVersion = bundleVersion;

  fs.writeFileSync(filePath, JSON.stringify(content, null, 2) + '\n');
  console.log(`Patched: ${filePath} (version: ${displayVersion}, versionCode: ${content.bundle.android.versionCode}, bundleVersion: ${bundleVersion})`);
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

/**
 * Patch BDS manifest.json - updates version field with encoded array
 */
function patchBdsManifest(filePath, version) {
  if (!fs.existsSync(filePath)) {
    console.log(`Skipping (not found): ${filePath}`);
    return;
  }
  const content = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  const encoded = encodeModVersion(version);
  content.header.version = [encoded.major, encoded.minor, encoded.encodedPatch];
  fs.writeFileSync(filePath, JSON.stringify(content, null, 2) + '\n');
  console.log(`Patched: ${filePath} (version: [${encoded.major}, ${encoded.minor}, ${encoded.encodedPatch}])`);
}

/**
 * Patch Apple Info.plist - updates CFBundleShortVersionString and CFBundleVersion
 */
function patchInfoPlist(filePath, version) {
  if (!fs.existsSync(filePath)) {
    console.log(`Skipping (not found): ${filePath}`);
    return;
  }

  const encoded = encodeModVersion(version);
  const shortVersion = `${encoded.major}.${encoded.minor}.${encoded.encodedPatch}`;
  const bundleVersion = String(calculateVersionCode(version));

  let content = fs.readFileSync(filePath, 'utf8');

  content = content.replace(
    /(<key>CFBundleShortVersionString<\/key>\s*<string>)[^<]*/,
    `$1${shortVersion}`
  );

  content = content.replace(
    /(<key>CFBundleVersion<\/key>\s*<string>)[^<]*/,
    `$1${bundleVersion}`
  );

  fs.writeFileSync(filePath, content);
  console.log(`Patched: ${filePath} (CFBundleShortVersionString: ${shortVersion}, CFBundleVersion: ${bundleVersion})`);
}

/**
 * Patch Cargo.lock - updates the version for a specific package
 */
function patchCargoLock(filePath, packageName, version) {
  if (!fs.existsSync(filePath)) {
    console.log(`Skipping (not found): ${filePath}`);
    return;
  }
  const content = fs.readFileSync(filePath, 'utf8');
  const pattern = new RegExp(
    `(\\[\\[package\\]\\]\\nname = "${packageName}"\\nversion = ")[^"]*"`,
  );
  const updated = content.replace(pattern, `$1${version}"`);
  fs.writeFileSync(filePath, updated);
  console.log(`Patched: ${filePath} (${packageName} -> ${version})`);
}

/**
 * Patch gradle.properties - updates modVersion field
 */
function patchGradleProperties(filePath, encodedVersion) {
  if (!fs.existsSync(filePath)) {
    console.log(`Skipping (not found): ${filePath}`);
    return;
  }
  const content = fs.readFileSync(filePath, 'utf8');
  const updated = content.replace(/^modVersion\s*=\s*.*/m, `modVersion=${encodedVersion}`);
  fs.writeFileSync(filePath, updated);
  console.log(`Patched: ${filePath} -> modVersion=${encodedVersion}`);
}

/**
 * Patch Hytale manifest.json - updates Version field
 */
function patchHytaleManifest(filePath, encodedVersion) {
  if (!fs.existsSync(filePath)) {
    console.log(`Skipping (not found): ${filePath}`);
    return;
  }
  const content = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  content.Version = encodedVersion;
  fs.writeFileSync(filePath, JSON.stringify(content, null, 2) + '\n');
  console.log(`Patched: ${filePath} -> Version="${encodedVersion}"`);
}

// Main execution
const rootDir = path.resolve(__dirname, '../..');

console.log(`Patching files to version ${version}...`);
console.log(`Android versionCode will be: ${calculateVersionCode(version)}`);
console.log('');

patchCargoToml(path.join(rootDir, 'server/server/Cargo.toml'), version);
patchCargoLock(path.join(rootDir, 'server/Cargo.lock'), 'bedrock-voice-chat-server', version);
patchCargoToml(path.join(rootDir, 'client/src-tauri/Cargo.toml'), version);
patchCargoLock(path.join(rootDir, 'Cargo.lock'), 'bedrock-voice-chat-client', version);
patchCargoLock(path.join(rootDir, 'client/src-tauri/Cargo.lock'), 'bedrock-voice-chat-client', version);
patchTauriConf(path.join(rootDir, 'client/src-tauri/tauri.conf.json'), version);
patchInfoPlist(path.join(rootDir, 'client/src-tauri/Info.ios.plist'), version);
patchPackageJson(path.join(rootDir, 'client/package.json'), version);

// BDS mod files
patchPackageJson(path.join(rootDir, 'mods/bds/package.json'), version);
patchBdsManifest(path.join(rootDir, 'mods/bds/manifest.json'), version);

// Java mod files (using encoded version for consistency with BDS)
const encoded = encodeModVersion(version);
const encodedDisplay = `${encoded.major}.${encoded.minor}.${encoded.encodedPatch}`;
console.log(`\nEncoded mod version: ${encodedDisplay}`);

patchGradleProperties(path.join(rootDir, 'mods/java/gradle.properties'), encodedDisplay);
patchGradleProperties(path.join(rootDir, 'mods/java/fabric/gradle.properties'), encodedDisplay);
patchHytaleManifest(path.join(rootDir, 'mods/java/hytale/src/main/resources/manifest.json'), encodedDisplay);

console.log('');
console.log(`All files patched to version ${version}`);
