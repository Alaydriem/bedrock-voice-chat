// PowerNukkitX dev-server helpers, used by `dev-server.js --powernukkitx`.
//
// Errors throw rather than exit, as in `bds.js`.

const { execFileSync } = require('child_process');
const { existsSync, mkdirSync } = require('fs');
const path = require('path');

const { Download } = require('./download');

const PNX_VERSION = '3.0.6';
const PNX_JAR_URL = `https://github.com/PowerNukkitX/PowerNukkitX/releases/download/${PNX_VERSION}/powernukkitx.jar`;
const PNX_PORT = 19132;

const modsDir = path.join(__dirname, '..');

class PowerNukkitX {
  // Server directory: PNX_SERVER_PATH, else mods/.dev/powernukkitx. Created when missing.
  static serverDir(env) {
    const dir = env.PNX_SERVER_PATH || path.join(modsDir, '.dev', 'powernukkitx');
    mkdirSync(dir, { recursive: true });
    return dir;
  }

  // The plugin's data directory, which holds config.yml and the embedded server's certificates.
  static dataDir(serverDir) {
    return path.join(serverDir, 'plugins', 'BedrockVoiceChat');
  }

  static async ensureJar(serverDir) {
    const jar = path.join(serverDir, 'powernukkitx.jar');
    if (existsSync(jar)) return jar;

    console.log(`[dev-server] downloading PowerNukkitX ${PNX_VERSION}`);
    await Download.file(PNX_JAR_URL, jar, null);
    console.log('[dev-server] downloaded powernukkitx.jar');
    return jar;
  }

  static javaBin(env) {
    if (env.PNX_JAVA_BIN) return env.PNX_JAVA_BIN;
    const exe = process.platform === 'win32' ? 'java.exe' : 'java';
    const where = () =>
      execFileSync('mise', ['where', 'java@temurin-21'], { encoding: 'utf8' }).trim();
    let home;
    try {
      home = where();
    } catch {
      console.log('[dev-server] installing java@temurin-21 for PowerNukkitX');
      execFileSync('mise', ['install', 'java@temurin-21'], { stdio: 'inherit' });
      home = where();
    }
    return path.join(home, 'bin', exe);
  }

  // PowerNukkitX compiles event executors with ClassLoader.defineClass, which needs java.lang
  // opened to the unnamed module. The jar's manifest does not declare it.
  static launchArgs() {
    const opens = ['java.lang', 'java.io', 'java.util', 'java.nio', 'java.net'].flatMap((p) => [
      '--add-opens',
      `java.base/${p}=ALL-UNNAMED`,
    ]);
    return [
      ...opens,
      '-jar',
      'powernukkitx.jar',
      '--language',
      'eng',
      '--skip-setup',
      '--accept-license',
      '--port',
      String(PNX_PORT),
    ];
  }
}

module.exports = { PowerNukkitX, PNX_VERSION };
