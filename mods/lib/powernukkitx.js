// PowerNukkitX dev-server helpers, used by `dev-server.js --powernukkitx`.
//
// Errors throw rather than exit, as in `bds.js`.

const { execFileSync } = require('child_process');
const { X509Certificate } = require('crypto');
const {
  existsSync,
  mkdirSync,
  copyFileSync,
  readFileSync,
  writeFileSync,
  createWriteStream,
  renameSync,
} = require('fs');
const path = require('path');
const { Readable } = require('stream');
const { pipeline } = require('stream/promises');

const PNX_VERSION = '3.0.6';
const PNX_JAR_URL = `https://github.com/PowerNukkitX/PowerNukkitX/releases/download/${PNX_VERSION}/powernukkitx.jar`;
const PNX_PORT = 19132;
const DEFAULT_EMBEDDED_PORT = 28280;

const modsDir = path.join(__dirname, '..');
const repoRoot = path.join(modsDir, '..');

class PowerNukkitX {
  // Server directory: PNX_SERVER_PATH, else mods/.dev/powernukkitx. Created when missing.
  static serverDir(env) {
    const dir = env.PNX_SERVER_PATH || path.join(modsDir, '.dev', 'powernukkitx');
    mkdirSync(dir, { recursive: true });
    return dir;
  }

  static async ensureJar(serverDir) {
    const jar = path.join(serverDir, 'powernukkitx.jar');
    if (existsSync(jar)) return jar;

    console.log(`[dev-server] downloading PowerNukkitX ${PNX_VERSION}\n  ${PNX_JAR_URL}`);
    const res = await fetch(PNX_JAR_URL);
    if (res.status !== 200 || !res.body) {
      throw new Error(`PowerNukkitX download failed: HTTP ${res.status} from ${PNX_JAR_URL}`);
    }
    // A partial download must not be mistaken for a finished one on the next run.
    const partial = `${jar}.part`;
    await pipeline(Readable.fromWeb(res.body), createWriteStream(partial));
    renameSync(partial, jar);
    console.log('[dev-server] downloaded powernukkitx.jar');
    return jar;
  }

  static certsDir(env) {
    const base =
      env.BVC_TLS_CERTS_PATH || path.join(repoRoot, 'server', 'server', 'certificates');
    return path.join(base, 'acme');
  }

  // Copies the ACME pair on every run so a renewed certificate is picked up.
  static copyCertificates(env, pluginDir) {
    const src = PowerNukkitX.certsDir(env);
    const cert = path.join(src, 'cert.pem');
    const key = path.join(src, 'key.pem');
    if (!existsSync(cert) || !existsSync(key)) {
      throw new Error(
        'No ACME certificate in server/server/certificates/acme. ' +
          'Run `mise run dev-server` once to obtain it.'
      );
    }
    const tlsDir = path.join(pluginDir, 'tls');
    mkdirSync(tlsDir, { recursive: true });
    copyFileSync(cert, path.join(tlsDir, 'cert.pem'));
    copyFileSync(key, path.join(tlsDir, 'key.pem'));
    console.log(`[dev-server] copied ACME certificate from ${src}`);
    return {
      certificate: path.join(tlsDir, 'cert.pem'),
      key: path.join(tlsDir, 'key.pem'),
      names: PowerNukkitX.certificateNames(cert),
    };
  }

  // The DNS names a certificate is issued for, which the server requires as tls.names.
  static certificateNames(certFile) {
    const san = new X509Certificate(readFileSync(certFile)).subjectAltName || '';
    const names = san
      .split(',')
      .map((entry) => entry.trim())
      .filter((entry) => entry.startsWith('DNS:'))
      .map((entry) => entry.slice('DNS:'.length));
    if (names.length === 0) {
      throw new Error(`${certFile} carries no DNS subject alternative names`);
    }
    return names;
  }

  // The `port` of the top-level `server` block in the local dev server's config.hcl.
  static embeddedPort() {
    const file = path.join(repoRoot, 'server', 'server', 'config.hcl');
    if (!existsSync(file)) {
      console.log(
        `[dev-server] server/server/config.hcl not found; using port ${DEFAULT_EMBEDDED_PORT}`
      );
      return DEFAULT_EMBEDDED_PORT;
    }
    const text = readFileSync(file, 'utf8');
    const block = /^server\s*\{([\s\S]*?)^\}/m.exec(text);
    const body = block ? block[1] : text;
    // Only a `port` assignment at the block's own nesting depth counts.
    let depth = 0;
    for (const raw of body.split(/\r?\n/)) {
      const line = raw.trim();
      const m = depth === 0 && /^port\s*=\s*(\d+)\s*$/.exec(line);
      if (m) return Number(m[1]);
      depth += (line.match(/\{/g) || []).length - (line.match(/\}/g) || []).length;
    }
    console.log(`[dev-server] no server port in config.hcl; using ${DEFAULT_EMBEDDED_PORT}`);
    return DEFAULT_EMBEDDED_PORT;
  }

  // Writes the plugin config once; an existing file keeps the operator's edits.
  static seedConfig(pluginDir, certs) {
    const file = path.join(pluginDir, 'config.yml');
    if (existsSync(file)) {
      console.log('[dev-server] plugins/BedrockVoiceChat/config.yml exists; leaving it as is');
      return;
    }
    const yamlPath = (p) => JSON.stringify(p.replace(/\\/g, '/'));
    const body = [
      'use-embedded-server: true',
      'telemetry: false',
      'embedded-config:',
      '  server:',
      `    port: ${PowerNukkitX.embeddedPort()}`,
      '    features:',
      '      telemetry: false',
      '    tls:',
      `      certificate: ${yamlPath(certs.certificate)}`,
      `      key: ${yamlPath(certs.key)}`,
      '      names:',
      ...certs.names.map((name) => `        - ${JSON.stringify(name)}`),
      '',
    ].join('\n');
    writeFileSync(file, body);
    console.log(`[dev-server] wrote ${file}`);
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
