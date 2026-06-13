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
 * - mods/bds/bp/manifest.json
 * - mods/bds/rp/manifest.json
 */

const fs = require('fs');
const path = require('path');
const { VersionEncoder } = require('./lib/encode-version');

const version = process.argv[2];
if (!version) {
  console.error('Usage: node patch-versions.js <version>');
  process.exit(1);
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

  const encoded = VersionEncoder.encode(version);
  const displayVersion = `${encoded.major}.${encoded.minor}.${encoded.encodedPatch}`;

  content.version = displayVersion;
  content.bundle.android.versionCode = VersionEncoder.versionCode(version);

  const bundleVersion = String(VersionEncoder.versionCode(version));
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
  const encoded = VersionEncoder.encode(version);
  const encodedVersion = [encoded.major, encoded.minor, encoded.encodedPatch];
  content.header.version = encodedVersion;
  for (const mod of content.modules || []) {
    if (Array.isArray(mod.version)) {
      mod.version = encodedVersion;
    }
  }
  for (const dep of content.dependencies || []) {
    if (dep.uuid && !dep.module_name && Array.isArray(dep.version)) {
      dep.version = encodedVersion;
    }
  }
  fs.writeFileSync(filePath, JSON.stringify(content, null, 2) + '\n');
  console.log(`Patched: ${filePath} (version: [${encodedVersion}])`);
}

/**
 * Patch Apple Info.plist - updates CFBundleShortVersionString and CFBundleVersion
 */
function patchInfoPlist(filePath, version) {
  if (!fs.existsSync(filePath)) {
    console.log(`Skipping (not found): ${filePath}`);
    return;
  }

  const encoded = VersionEncoder.encode(version);
  const shortVersion = `${encoded.major}.${encoded.minor}.${encoded.encodedPatch}`;
  const bundleVersion = String(VersionEncoder.versionCode(version));

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
console.log(`Android versionCode will be: ${VersionEncoder.versionCode(version)}`);
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
patchBdsManifest(path.join(rootDir, 'mods/bds/bp/manifest.json'), version);
patchBdsManifest(path.join(rootDir, 'mods/bds/rp/manifest.json'), version);

// Java mod files (using encoded version for consistency with BDS)
const encoded = VersionEncoder.encode(version);
const encodedDisplay = `${encoded.major}.${encoded.minor}.${encoded.encodedPatch}`;
console.log(`\nEncoded mod version: ${encodedDisplay}`);

patchGradleProperties(path.join(rootDir, 'mods/java/gradle.properties'), encodedDisplay);
patchGradleProperties(path.join(rootDir, 'mods/java/fabric/gradle.properties'), encodedDisplay);
patchHytaleManifest(path.join(rootDir, 'mods/java/hytale/src/main/resources/manifest.json'), encodedDisplay);

console.log('');
console.log(`All files patched to version ${version}`);
