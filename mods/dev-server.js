const { execSync, spawnSync } = require('child_process');
const {
  existsSync,
  mkdirSync,
  copyFileSync,
  readdirSync,
  statSync,
  readFileSync,
  writeFileSync,
  rmSync,
} = require('fs');
const path = require('path');

const { VersionEncoder } = require('../.github/scripts/lib/encode-version');
const { UUIDS } = require('./bds/manifest.config');

const modsDir = __dirname;
const javaDir = path.join(modsDir, 'java');
const bdsDir = path.join(modsDir, 'bds');
const isWindows = process.platform === 'win32';
const gradlew = isWindows ? '.\\gradlew.bat' : './gradlew';

const args = process.argv.slice(2);
const flags = new Set(args.filter((a) => a.startsWith('--')).map((a) => a.replace(/^--/, '')));

function fail(msg) {
  console.error(`\n[dev-server] ${msg}\n`);
  process.exit(1);
}

function usage() {
  console.log(`
Usage: yarn dev-server <--bds | --paper | --fabric> [options]

  Builds the selected mod, deploys it to your local test server (paths read
  from mods/.env), then starts that server attached to this terminal so you
  can type console commands. Ctrl+C stops it; re-run to rebuild and restart.

Platforms (pick exactly one):
  --bds       Bedrock Dedicated Server
  --paper     Paper server
  --fabric    Fabric server

Options:
  --no-net    (BDS only) Deploy the no-net pack variant and point the world at it
  --release   Build the Rust native library / mods in release mode
  --no-build  Skip build + deploy; just (re)start the server

Configure paths in mods/.env (copy mods/.env.example to get started).
`);
}

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
    fail('Missing mods/.env. Copy mods/.env.example to mods/.env and set your paths.');
  }
  return parseKeyValueFile(envPath);
}

function requireEnv(env, key) {
  const val = env[key];
  if (!val) fail(`${key} is not set in mods/.env`);
  if (!existsSync(val)) fail(`${key} does not exist: ${val}`);
  return val;
}

function runGradle(cwd, tasks, release) {
  const cmd = [gradlew, ...tasks];
  if (release) cmd.push('-Prelease');
  const line = cmd.join(' ');
  console.log(`\n[dev-server] gradle (${path.basename(cwd)}): ${line}\n`);
  execSync(line, { cwd, stdio: 'inherit' });
}

function deployJar(jarPath, destDir, cleanPrefix) {
  if (!existsSync(jarPath)) {
    fail(`Built jar not found: ${jarPath}\nDid the build succeed?`);
  }
  mkdirSync(destDir, { recursive: true });
  for (const f of readdirSync(destDir)) {
    if (f.startsWith(cleanPrefix) && f.endsWith('.jar')) {
      rmSync(path.join(destDir, f));
    }
  }
  const dest = path.join(destDir, path.basename(jarPath));
  copyFileSync(jarPath, dest);
  console.log(`[dev-server] deployed ${path.basename(jarPath)} -> ${destDir}`);
}

function findServerJar(serverDir, regex, label) {
  const jars = readdirSync(serverDir).filter((f) => regex.test(f));
  if (jars.length === 0) {
    fail(`No ${label} launcher jar matching ${regex} found in ${serverDir}`);
  }
  if (jars.length > 1) {
    const launchers = jars.filter((f) => /launcher/i.test(f));
    if (launchers.length === 1) return launchers[0];
    fail(
      `Multiple candidate ${label} jars in ${serverDir}:\n  ${jars.join('\n  ')}\n` +
        'Leave only one launcher jar in that directory.'
    );
  }
  return jars[0];
}

function runJavaServer(env, serverDir, jarName) {
  const javaBin = requireEnv(env, 'JAVA_BIN');
  const xms = env.JVM_XMS || '4G';
  const xmx = env.JVM_XMX || '4G';
  const jvmArgs = [`-Xms${xms}`, `-Xmx${xmx}`, '-jar', jarName, '--nogui'];
  console.log(`\n[dev-server] starting ${jarName} (Ctrl+C to stop)\n`);
  const res = spawnSync(javaBin, jvmArgs, { cwd: serverDir, stdio: 'inherit' });
  process.exit(res.status ?? 0);
}

function runBds(bdsRoot) {
  const exe = path.join(bdsRoot, 'bedrock_server.exe');
  if (!existsSync(exe)) fail(`bedrock_server.exe not found in ${bdsRoot}`);
  console.log('\n[dev-server] starting BDS (Ctrl+C to stop)\n');
  const res = spawnSync(exe, [], { cwd: bdsRoot, stdio: 'inherit' });
  process.exit(res.status ?? 0);
}

function buildAndDeployBds(bdsRoot, noNet) {
  console.log('\n[dev-server] building BDS pack...\n');
  execSync('yarn run pack', { cwd: bdsDir, stdio: 'inherit' });

  const bpDest = path.join(bdsRoot, 'development_behavior_packs');
  const rpDest = path.join(bdsRoot, 'development_resource_packs');
  const bp = path.join(bdsDir, noNet ? 'bedrock-voice-chat-bp-no-net.mcpack' : 'bedrock-voice-chat-bp.mcpack');
  const rp = path.join(bdsDir, noNet ? 'bedrock-voice-chat-rp-no-net.mcpack' : 'bedrock-voice-chat-rp.mcpack');
  if (!existsSync(bp)) fail(`BDS BP pack not found: ${bp}`);
  if (!existsSync(rp)) fail(`BDS RP pack not found: ${rp}`);

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

  const versionArray = VersionEncoder.encode(require('./bds/package.json').version).array;
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

function main() {
  const platforms = ['bds', 'paper', 'fabric'].filter((p) => flags.has(p));

  if (flags.has('help') || flags.has('h')) {
    usage();
    return;
  }
  if (platforms.length === 0) {
    usage();
    fail('Specify exactly one platform: --bds, --paper, or --fabric');
  }
  if (platforms.length > 1) {
    fail('Specify only one platform at a time (--bds | --paper | --fabric)');
  }

  const platform = platforms[0];
  const env = loadEnv();
  const release = flags.has('release');
  const skipBuild = flags.has('no-build');
  const noNet = flags.has('no-net');

  if (noNet && platform !== 'bds') {
    console.warn('[dev-server] --no-net only applies to --bds; ignoring for this platform');
  }

  if (platform === 'bds') {
    const bdsRoot = requireEnv(env, 'BDS_SERVER_PATH');
    if (!skipBuild) buildAndDeployBds(bdsRoot, noNet);
    runBds(bdsRoot);
    return;
  }

  const props = parseKeyValueFile(path.join(javaDir, 'gradle.properties'));
  const base = props.archivesBaseName;
  const ver = props.modVersion;

  if (platform === 'paper') {
    const paperRoot = requireEnv(env, 'PAPER_SERVER_PATH');
    if (!skipBuild) {
      runGradle(javaDir, ['buildRustLibrary', ':common:copyNativeWindows', ':paper:shadowJar'], release);
      const jar = path.join(javaDir, 'paper', 'build', 'libs', `${base}-paper-${ver}.jar`);
      deployJar(jar, path.join(paperRoot, 'plugins'), `${base}-paper-`);
    }
    const jarName = findServerJar(paperRoot, /^paper-.*\.jar$/i, 'Paper');
    runJavaServer(env, paperRoot, jarName);
    return;
  }

  if (platform === 'fabric') {
    const fabricRoot = requireEnv(env, 'FABRIC_SERVER_PATH');
    if (!skipBuild) {
      runGradle(javaDir, ['buildRustLibrary', ':common:copyNativeWindows'], release);
      runGradle(path.join(javaDir, 'fabric'), ['build'], release);
      const jar = path.join(javaDir, 'fabric', 'build', 'libs', `${base}-${ver}.jar`);
      deployJar(jar, path.join(fabricRoot, 'mods'), `${base}-`);
    }
    const jarName = findServerJar(fabricRoot, /^fabric-server-.*\.jar$/i, 'Fabric');
    runJavaServer(env, fabricRoot, jarName);
    return;
  }
}

main();
