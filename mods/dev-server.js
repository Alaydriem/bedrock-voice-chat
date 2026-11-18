const { execSync, spawnSync } = require('child_process');
const { existsSync, mkdirSync, copyFileSync, readdirSync, rmSync } = require('fs');
const path = require('path');

const {
  parseKeyValueFile,
  loadEnv,
  requireEnv,
  buildAndDeployBds,
} = require('./lib/bds');

const { EmbeddedConfig } = require('./lib/embedded-config');
const { Fabric } = require('./lib/fabric');
const { JavaServer } = require('./lib/java-server');
const { Paper } = require('./lib/paper');
const { PowerNukkitX } = require('./lib/powernukkitx');

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
Usage: yarn dev-server <--bds | --paper | --fabric | --powernukkitx> [options]

  Builds the selected mod, deploys it to your local test server (paths read
  from mods/.env), then starts that server attached to this terminal so you
  can type console commands. Ctrl+C stops it; re-run to rebuild and restart.

Platforms (pick exactly one):
  --bds       Bedrock Dedicated Server
  --paper     Paper server with Geyser, Floodgate and Simple Voice Chat
  --fabric    Fabric server with Fabric API, Geyser, Floodgate and Simple
              Voice Chat
  --powernukkitx
              PowerNukkitX server

  Paper, Fabric and PowerNukkitX are downloaded and seeded on first run under
  mods/.dev/<platform>; their *_SERVER_PATH variables are optional. Paper and
  Fabric mirror the PowerNukkitX dev server's BVC config and certificates when
  it has them.

Options:
  --no-net    (BDS only) Deploy the no-net pack variant and point the world at it
  --release   Build the Rust native library / mods in release mode
  --no-build  Skip build + deploy; just (re)start the server
  --update    (Paper, Fabric) Resolve the newest server and mod builds again
              and replace any that changed

Configure paths in mods/.env (copy mods/.env.example to get started).
`);
}

// `-Pbundled` is not optional for a local run. Without it the build produces the
// skinny jar, whose manifest pins the release it was built against; a local build
// has no release, so every library resolves to a URL that does not exist. A dev
// machine is the offline case the bundled jar is for.
function runGradle(cwd, tasks, release) {
  const cmd = [gradlew, ...tasks, '-Pbundled'];
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

// JAVA_BIN is optional: the mise task runs under java@temurin-25, so `java` on PATH is right.
function runJavaServer(env, serverDir, jarName) {
  const javaBin = env.JAVA_BIN || 'java';
  const xms = env.JVM_XMS || '4G';
  const xmx = env.JVM_XMX || '4G';
  const jvmArgs = [`-Xms${xms}`, `-Xmx${xmx}`, '-jar', jarName, '--nogui'];
  console.log(`\n[dev-server] starting ${jarName} (Ctrl+C to stop)\n`);
  const res = spawnSync(javaBin, jvmArgs, { cwd: serverDir, stdio: 'inherit' });
  process.exit(res.status ?? 0);
}

function runPnxServer(env, serverDir) {
  const javaBin = PowerNukkitX.javaBin(env);
  console.log(`\n[dev-server] starting powernukkitx.jar (Ctrl+C to stop)\n`);
  const res = spawnSync(javaBin, PowerNukkitX.launchArgs(), { cwd: serverDir, stdio: 'inherit' });
  process.exit(res.status ?? 0);
}

async function runPowerNukkitX(env, release, skipBuild) {
  const serverDir = PowerNukkitX.serverDir(env);
  await PowerNukkitX.ensureJar(serverDir);

  const pluginDir = PowerNukkitX.dataDir(serverDir);
  EmbeddedConfig.seed(env, pluginDir, path.join(pluginDir, 'config.yml'), 'yaml');

  if (!skipBuild) {
    const props = parseKeyValueFile(path.join(javaDir, 'gradle.properties'));
    const base = props.archivesBaseName;
    const ver = props.modVersion;
    runGradle(
      javaDir,
      ['buildRustLibrary', ':common:copyNativeWindows', ':powernukkitx:shadowJar'],
      release
    );
    const jar = path.join(javaDir, 'powernukkitx', 'build', 'libs', `${base}-powernukkitx-${ver}.jar`);
    deployJar(jar, path.join(serverDir, 'plugins'), `${base}-powernukkitx-`);
  }
  runPnxServer(env, serverDir);
}

// Paper and Fabric run the same embedded server as the PowerNukkitX dev server, so they take its
// config and its issued certificates rather than obtaining their own. Without a PowerNukkitX
// config, they are seeded from the dev server's ACME pair as PowerNukkitX is.
function configureEmbedded(env, dataDir, configFile, format) {
  const pnxData = PowerNukkitX.dataDir(PowerNukkitX.serverDir(env));
  if (!EmbeddedConfig.mirror(pnxData, dataDir, configFile, format)) {
    EmbeddedConfig.seed(env, dataDir, configFile, format);
  }
}

async function runPaper(env, release, skipBuild, update) {
  const serverDir = JavaServer.serverDir(env, 'PAPER_SERVER_PATH', 'paper');
  await Paper.provision(serverDir, env, update);

  const dataDir = path.join(serverDir, 'plugins', 'BedrockVoiceChat');
  configureEmbedded(env, dataDir, path.join(dataDir, 'config.yml'), 'yaml');

  if (!skipBuild) {
    const props = parseKeyValueFile(path.join(javaDir, 'gradle.properties'));
    const base = props.archivesBaseName;
    const ver = props.modVersion;
    runGradle(javaDir, ['buildRustLibrary', ':common:copyNativeWindows', ':paper:shadowJar'], release);
    const jar = path.join(javaDir, 'paper', 'build', 'libs', `${base}-paper-${ver}.jar`);
    deployJar(jar, path.join(serverDir, 'plugins'), `${base}-paper-`);
  }
  const jarName = findServerJar(serverDir, /^paper-.*\.jar$/i, 'Paper');
  runJavaServer(env, serverDir, jarName);
}

async function runFabric(env, release, skipBuild, update) {
  const serverDir = JavaServer.serverDir(env, 'FABRIC_SERVER_PATH', 'fabric');
  await Fabric.provision(serverDir, env, update);

  const configDir = path.join(serverDir, 'config');
  configureEmbedded(
    env,
    path.join(configDir, 'bedrock-voice-chat'),
    path.join(configDir, 'bedrock-voice-chat.json'),
    'json'
  );

  if (!skipBuild) {
    const props = parseKeyValueFile(path.join(javaDir, 'gradle.properties'));
    const base = props.archivesBaseName;
    const ver = props.modVersion;
    runGradle(javaDir, ['buildRustLibrary', ':common:copyNativeWindows'], release);
    runGradle(path.join(javaDir, 'fabric'), ['build'], release);
    const jar = path.join(javaDir, 'fabric', 'build', 'libs', `${base}-${ver}.jar`);
    deployJar(jar, path.join(serverDir, 'mods'), `${base}-`);
  }
  const jarName = findServerJar(serverDir, /^fabric-server-.*\.jar$/i, 'Fabric');
  runJavaServer(env, serverDir, jarName);
}

function runBds(bdsRoot) {
  const exe = path.join(bdsRoot, 'bedrock_server.exe');
  if (!existsSync(exe)) fail(`bedrock_server.exe not found in ${bdsRoot}`);
  console.log('\n[dev-server] starting BDS (Ctrl+C to stop)\n');
  const res = spawnSync(exe, [], { cwd: bdsRoot, stdio: 'inherit' });
  process.exit(res.status ?? 0);
}

async function main() {
  const platforms = ['bds', 'paper', 'fabric', 'powernukkitx'].filter((p) => flags.has(p));

  if (flags.has('help') || flags.has('h')) {
    usage();
    return;
  }
  if (platforms.length === 0) {
    usage();
    fail('Specify exactly one platform: --bds, --paper, --fabric, or --powernukkitx');
  }
  if (platforms.length > 1) {
    fail('Specify only one platform at a time (--bds | --paper | --fabric | --powernukkitx)');
  }

  const platform = platforms[0];
  // Only BDS needs mods/.env; every Paper, Fabric and PowerNukkitX variable is optional.
  // A process environment variable wins over the file.
  const env = {
    ...loadEnv(platform !== 'bds'),
    ...Object.fromEntries(
      [
        'JAVA_BIN',
        'PAPER_SERVER_PATH',
        'PAPER_MC_VERSION',
        'FABRIC_SERVER_PATH',
        'FABRIC_MC_VERSION',
        'PNX_SERVER_PATH',
        'PNX_JAVA_BIN',
        'BVC_TLS_CERTS_PATH',
      ]
        .filter((k) => process.env[k])
        .map((k) => [k, process.env[k]])
    ),
  };
  const release = flags.has('release');
  const skipBuild = flags.has('no-build');
  const noNet = flags.has('no-net');
  const update = flags.has('update');

  if (noNet && platform !== 'bds') {
    console.warn('[dev-server] --no-net only applies to --bds; ignoring for this platform');
  }
  if (update && platform !== 'paper' && platform !== 'fabric') {
    console.warn('[dev-server] --update only applies to --paper and --fabric; ignoring');
  }

  if (platform === 'powernukkitx') {
    await runPowerNukkitX(env, release, skipBuild);
    return;
  }

  if (platform === 'bds') {
    const bdsRoot = requireEnv(env, 'BDS_SERVER_PATH');
    if (!skipBuild) buildAndDeployBds(bdsRoot, noNet);
    runBds(bdsRoot);
    return;
  }

  if (platform === 'paper') {
    await runPaper(env, release, skipBuild, update);
    return;
  }

  if (platform === 'fabric') {
    await runFabric(env, release, skipBuild, update);
  }
}

// The shared BDS helpers throw rather than exit, so a missing path still prints one line here
// instead of a stack trace.
main().catch((e) => fail(e.message ?? String(e)));
