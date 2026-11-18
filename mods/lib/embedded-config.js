// Seeds the BVC mod's embedded-server configuration for a local dev server, shared by every
// Java-side platform.
//
// Errors throw rather than exit, as in `bds.js`.

const { X509Certificate } = require('crypto');
const {
  existsSync,
  mkdirSync,
  copyFileSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} = require('fs');
const path = require('path');
const YAML = require('yaml');

const DEFAULT_EMBEDDED_PORT = 28280;
const DATABASE = 'bvc.sqlite3';

const repoRoot = path.join(__dirname, '..', '..');

class EmbeddedConfig {
  static certsDir(env) {
    const base =
      env.BVC_TLS_CERTS_PATH || path.join(repoRoot, 'server', 'server', 'certificates');
    return path.join(base, 'acme');
  }

  // Copies the ACME pair into `<dataDir>/tls` on every run so a renewed certificate is picked up.
  static copyCertificates(env, dataDir) {
    const src = EmbeddedConfig.certsDir(env);
    const cert = path.join(src, 'cert.pem');
    const key = path.join(src, 'key.pem');
    if (!existsSync(cert) || !existsSync(key)) {
      throw new Error(
        'No ACME certificate in server/server/certificates/acme. ' +
          'Run `mise run dev-server` once to obtain it.'
      );
    }
    const tlsDir = path.join(dataDir, 'tls');
    mkdirSync(tlsDir, { recursive: true });
    copyFileSync(cert, path.join(tlsDir, 'cert.pem'));
    copyFileSync(key, path.join(tlsDir, 'key.pem'));
    console.log(`[dev-server] copied ACME certificate from ${src}`);
    return {
      certificate: path.join(tlsDir, 'cert.pem'),
      key: path.join(tlsDir, 'key.pem'),
      names: EmbeddedConfig.certificateNames(cert),
    };
  }

  // Re-copies the ACME pair into a `<dataDir>/tls` that an earlier seed created, so a renewed
  // certificate is picked up. Does nothing when either side is missing.
  static refreshCertificates(env, dataDir) {
    const src = EmbeddedConfig.certsDir(env);
    const tlsDir = path.join(dataDir, 'tls');
    if (!existsSync(path.join(src, 'cert.pem')) || !existsSync(tlsDir)) return;
    EmbeddedConfig.copyCertificates(env, dataDir);
  }

  // Seeds `configFile` for a platform with no config to mirror: the dev server's ACME pair and
  // an embedded config that names it. `format` is 'yaml' or 'json'.
  static seed(env, dataDir, configFile, format) {
    if (existsSync(configFile)) {
      EmbeddedConfig.refreshCertificates(env, dataDir);
      return;
    }
    const certs = EmbeddedConfig.copyCertificates(env, dataDir);
    if (format === 'json') EmbeddedConfig.seedJson(configFile, certs);
    else EmbeddedConfig.seedYaml(configFile, certs);
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

  // The mod configuration as a plain object: embedded mode, telemetry off, the copied certificate.
  static document(certs) {
    const slash = (p) => p.replace(/\\/g, '/');
    return {
      'use-embedded-server': true,
      telemetry: false,
      'embedded-config': {
        server: {
          port: EmbeddedConfig.embeddedPort(),
          features: { telemetry: false },
          tls: {
            certificate: slash(certs.certificate),
            key: slash(certs.key),
            names: certs.names,
          },
        },
      },
    };
  }

  // Writes the YAML plugin config (Paper, PowerNukkitX) once; an existing file keeps the
  // operator's edits.
  static seedYaml(file, certs) {
    if (EmbeddedConfig.exists(file)) return;
    const doc = EmbeddedConfig.document(certs);
    const server = doc['embedded-config'].server;
    const body = [
      'use-embedded-server: true',
      'telemetry: false',
      'embedded-config:',
      '  server:',
      `    port: ${server.port}`,
      '    features:',
      '      telemetry: false',
      '    tls:',
      `      certificate: ${JSON.stringify(server.tls.certificate)}`,
      `      key: ${JSON.stringify(server.tls.key)}`,
      '      names:',
      ...server.tls.names.map((name) => `        - ${JSON.stringify(name)}`),
      '',
    ].join('\n');
    EmbeddedConfig.write(file, body);
  }

  // Writes the JSON mod config (Fabric) once; an existing file keeps the operator's edits.
  static seedJson(file, certs) {
    if (EmbeddedConfig.exists(file)) return;
    EmbeddedConfig.write(file, `${JSON.stringify(EmbeddedConfig.document(certs), null, 2)}\n`);
  }

  // Mirrors another dev server's BVC data directory into `targetDataDir`: the database, the
  // `certificates` directory and the mod config. The ACME certificate, its account and the server
  // CA are stored in the database, and `certificates/` is only materialised from it, so the
  // database is what keeps the target from issuing its own certificate. It is copied only while
  // the target has none, so a server's own data is never replaced. A certificate file is copied
  // when it is missing or the source is newer. The config is written only when missing; `format`
  // is 'yaml' or 'json'. Returns false when the source has no config to mirror.
  static mirror(sourceDataDir, targetDataDir, targetConfigFile, format) {
    const sourceConfig = path.join(sourceDataDir, 'config.yml');
    if (!existsSync(sourceConfig)) return false;

    const db = path.join(sourceDataDir, DATABASE);
    if (existsSync(db) && !existsSync(path.join(targetDataDir, DATABASE))) {
      mkdirSync(targetDataDir, { recursive: true });
      for (const suffix of ['', '-wal', '-shm']) {
        if (existsSync(db + suffix)) {
          copyFileSync(db + suffix, path.join(targetDataDir, DATABASE + suffix));
        }
      }
      console.log(`[dev-server] mirrored ${db}`);
    }

    // Uploaded audio is a file per database row, so it travels with the database.
    const assets = path.join(sourceDataDir, 'assets');
    if (existsSync(assets)) {
      const copied = EmbeddedConfig.copyNewer(assets, path.join(targetDataDir, 'assets'));
      console.log(`[dev-server] mirrored ${copied} audio asset file(s) from ${assets}`);
    }

    const certs = path.join(sourceDataDir, 'certificates');
    if (existsSync(certs)) {
      const copied = EmbeddedConfig.copyNewer(certs, path.join(targetDataDir, 'certificates'));
      console.log(`[dev-server] mirrored ${copied} certificate file(s) from ${certs}`);
    }

    if (!EmbeddedConfig.exists(targetConfigFile)) {
      const text = readFileSync(sourceConfig, 'utf8');
      const body =
        format === 'json' ? `${JSON.stringify(YAML.parse(text), null, 2)}\n` : text;
      EmbeddedConfig.write(targetConfigFile, body);
      console.log(`[dev-server] config mirrored from ${sourceConfig}`);
    }
    return true;
  }

  static copyNewer(src, dest) {
    mkdirSync(dest, { recursive: true });
    let copied = 0;
    for (const entry of readdirSync(src, { withFileTypes: true })) {
      const from = path.join(src, entry.name);
      const to = path.join(dest, entry.name);
      if (entry.isDirectory()) {
        copied += EmbeddedConfig.copyNewer(from, to);
      } else if (!existsSync(to) || statSync(from).mtimeMs > statSync(to).mtimeMs) {
        copyFileSync(from, to);
        copied += 1;
      }
    }
    return copied;
  }

  static exists(file) {
    if (!existsSync(file)) return false;
    console.log(`[dev-server] ${file} exists; leaving it as is`);
    return true;
  }

  static write(file, body) {
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, body);
    console.log(`[dev-server] wrote ${file}`);
  }
}

module.exports = { EmbeddedConfig };
