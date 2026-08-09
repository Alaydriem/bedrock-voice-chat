// BDS build, deploy and environment helpers, shared by `dev-server.js` and `jukebox-test.js`.
//
// Errors throw rather than exit: the two callers report failure differently, and a library that
// calls `process.exit` cannot be used by a script that needs to shut a server down first.

const { execSync } = require('child_process');
const {
  existsSync,
  mkdirSync,
  copyFileSync,
  readdirSync,
  statSync,
  readFileSync,
  writeFileSync,
} = require('fs');
const path = require('path');

const { VersionEncoder } = require('../../.github/scripts/lib/encode-version');
const { UUIDS } = require('../bds/manifest.config');

const modsDir = path.join(__dirname, '..');
const bdsDir = path.join(modsDir, 'bds');

function parseKeyValueFile(file) {
  const out = {};
  for (const raw of readFileSync(file, 'utf8').split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    const eq = line.indexOf('=');
    if (eq === -1) continue;
    const key = line.slice(0, eq).trim();
    let val = line.slice(eq + 1).trim();
    if (
      (val.startsWith('"') && val.endsWith('"')) ||
      (val.startsWith("'") && val.endsWith("'"))
    ) {
      val = val.slice(1, -1);
    }
    out[key] = val;
  }
  return out;
}

function loadEnv() {
  const envPath = path.join(modsDir, '.env');
  if (!existsSync(envPath)) {
    throw new Error(
      'Missing mods/.env. Copy mods/.env.example to mods/.env and set your paths.'
    );
  }
  return parseKeyValueFile(envPath);
}

function requireEnv(env, key) {
  const val = env[key];
  if (!val) throw new Error(`${key} is not set in mods/.env`);
  if (!existsSync(val)) throw new Error(`${key} does not exist: ${val}`);
  return val;
}

function buildAndDeployBds(bdsRoot, noNet) {
  console.log('\n[dev-server] building BDS pack...\n');
  execSync('yarn run pack', { cwd: bdsDir, stdio: 'inherit' });

  const bpDest = path.join(bdsRoot, 'development_behavior_packs');
  const rpDest = path.join(bdsRoot, 'development_resource_packs');
  const bp = path.join(
    bdsDir,
    noNet ? 'bedrock-voice-chat-bp-no-net.mcpack' : 'bedrock-voice-chat-bp.mcpack'
  );
  const rp = path.join(
    bdsDir,
    noNet ? 'bedrock-voice-chat-rp-no-net.mcpack' : 'bedrock-voice-chat-rp.mcpack'
  );
  if (!existsSync(bp)) throw new Error(`BDS BP pack not found: ${bp}`);
  if (!existsSync(rp)) throw new Error(`BDS RP pack not found: ${rp}`);

  mkdirSync(bpDest, { recursive: true });
  mkdirSync(rpDest, { recursive: true });
  copyFileSync(bp, path.join(bpDest, 'bedrock-voice-chat-bp.zip'));
  copyFileSync(rp, path.join(rpDest, 'bedrock-voice-chat-rp.zip'));
  console.log(`[dev-server] deployed BDS ${noNet ? 'no-net' : 'full'} packs`);

  updateWorldPacks(bdsRoot, noNet ? 'no-net' : 'full');
}

function updatePackFile(file, ourUuids, targetUuid, versionArray) {
  if (!existsSync(file)) return false;
  let data;
  try {
    data = JSON.parse(readFileSync(file, 'utf8'));
  } catch {
    console.warn(`[dev-server] skipping unparseable ${file}`);
    return false;
  }
  if (!Array.isArray(data)) return false;

  let changed = false;
  const next = data.map((entry) => {
    if (entry && ourUuids.includes(entry.pack_id)) {
      changed = true;
      return { ...entry, pack_id: targetUuid, version: versionArray };
    }
    return entry;
  });
  if (changed) writeFileSync(file, JSON.stringify(next, null, 4) + '\n');
  return changed;
}

function updateWorldPacks(bdsRoot, variantKey) {
  const worldsDir = path.join(bdsRoot, 'worlds');
  if (!existsSync(worldsDir)) return;

  const versionArray = VersionEncoder.encode(
    require('../bds/package.json').version
  ).array;
  const ourBp = [UUIDS.full.bp.header, UUIDS['no-net'].bp.header];
  const ourRp = [UUIDS.full.rp.header, UUIDS['no-net'].rp.header];
  const targetBp = UUIDS[variantKey].bp.header;
  const targetRp = UUIDS[variantKey].rp.header;

  let touched = 0;
  for (const name of readdirSync(worldsDir)) {
    const wdir = path.join(worldsDir, name);
    if (!statSync(wdir).isDirectory()) continue;
    const bpChanged = updatePackFile(
      path.join(wdir, 'world_behavior_packs.json'),
      ourBp,
      targetBp,
      versionArray
    );
    const rpChanged = updatePackFile(
      path.join(wdir, 'world_resource_packs.json'),
      ourRp,
      targetRp,
      versionArray
    );
    if (bpChanged || rpChanged) touched++;
  }
  console.log(
    `[dev-server] pointed world packs at ${variantKey} (v${versionArray.join('.')}) in ${touched} world(s)`
  );
}

module.exports = {
  parseKeyValueFile,
  loadEnv,
  requireEnv,
  buildAndDeployBds,
  updatePackFile,
  updateWorldPacks,
};
