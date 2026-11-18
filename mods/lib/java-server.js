// Provisioning shared by the Paper and Fabric dev servers: server directory, EULA, and the
// downloaded server jar and mods recorded in a manifest.
//
// Errors throw rather than exit, as in `bds.js`.

const { existsSync, mkdirSync, readFileSync, writeFileSync, rmSync } = require('fs');
const path = require('path');

const { Download } = require('./download');

const MANIFEST = '.bvc-dev.json';

const modsDir = path.join(__dirname, '..');

class JavaServer {
  // Server directory: the env variable `key`, else mods/.dev/<name>. Created when missing.
  static serverDir(env, key, name) {
    const dir = env[key] || path.join(modsDir, '.dev', name);
    mkdirSync(dir, { recursive: true });
    return dir;
  }

  // A dev server is for local testing only, so the Minecraft EULA is accepted on the operator's
  // behalf rather than stopping the first start.
  static acceptEula(serverDir) {
    const file = path.join(serverDir, 'eula.txt');
    if (existsSync(file) && /^eula=true$/m.test(readFileSync(file, 'utf8'))) return;
    writeFileSync(
      file,
      '# https://aka.ms/MinecraftEULA, accepted by mods/dev-server.js\neula=true\n'
    );
    console.log('[dev-server] accepted the Minecraft EULA in eula.txt');
  }

  // Installs the artifacts `resolve()` returns, as { key: { name, url, filename, hash, dir } }.
  // A manifest records what was installed, so later runs make no network calls. `update`
  // resolves again and replaces anything that changed.
  static async provision(serverDir, minecraft, resolve, update) {
    const manifestFile = path.join(serverDir, MANIFEST);
    const manifest = existsSync(manifestFile)
      ? JSON.parse(readFileSync(manifestFile, 'utf8'))
      : { artifacts: {} };

    const installed = Object.values(manifest.artifacts);
    const complete =
      manifest.minecraft === minecraft &&
      installed.length > 0 &&
      installed.every((entry) => existsSync(path.join(serverDir, entry.path)));
    if (complete && !update) {
      console.log(`[dev-server] ${serverDir} is provisioned for Minecraft ${minecraft}`);
      return;
    }

    console.log(`[dev-server] resolving Minecraft ${minecraft} server and mods`);
    const artifacts = await resolve();
    const next = { minecraft, artifacts: {} };
    for (const [key, a] of Object.entries(artifacts)) {
      const rel = path.join(a.dir, a.filename);
      const dest = path.join(serverDir, rel);
      const old = manifest.artifacts[key];
      mkdirSync(path.dirname(dest), { recursive: true });
      // Some jars carry no version in their filename (Geyser-Spigot.jar), so a file at the same
      // path is current only when the manifest recorded the same build there.
      if (existsSync(dest) && old && old.path === rel && old.name === a.name) {
        console.log(`[dev-server] ${a.name}: ${rel} present`);
      } else {
        await Download.file(a.url, dest, a.hash);
        console.log(`[dev-server] ${a.name}: installed ${rel}`);
      }
      if (old && old.path !== rel && existsSync(path.join(serverDir, old.path))) {
        rmSync(path.join(serverDir, old.path));
        console.log(`[dev-server] removed ${old.path}`);
      }
      next.artifacts[key] = { path: rel, name: a.name };
    }
    writeFileSync(manifestFile, `${JSON.stringify(next, null, 2)}\n`);
  }
}

module.exports = { JavaServer };
