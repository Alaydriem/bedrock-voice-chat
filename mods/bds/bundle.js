const archiver = require('archiver');
const fs = require('fs');
const path = require('path');
const { VersionEncoder } = require('../../.github/scripts/lib/encode-version');
const { ManifestBuilder } = require('./manifest');
const manifestConfig = require('./manifest.config');

function packDir(name) {
  return path.join(__dirname, name);
}

function createPackZip(outputName, sourceDir, manifestJson) {
  return new Promise((resolve, reject) => {
    const output = fs.createWriteStream(path.join(__dirname, outputName));
    const archive = archiver('zip', { zlib: { level: 9 } });

    output.on('close', () => {
      console.log(`  ${outputName}: ${archive.pointer()} bytes`);
      resolve();
    });
    archive.on('error', reject);
    archive.pipe(output);

    archive.glob('**/*', {
      cwd: packDir(sourceDir),
      ignore: ['manifest.json'],
      dot: false,
    });
    archive.append(JSON.stringify(manifestJson, null, 2) + '\n', { name: 'manifest.json' });
    archive.finalize();
  });
}

function createAddon(addonName, mcpackFiles) {
  return new Promise((resolve, reject) => {
    const output = fs.createWriteStream(path.join(__dirname, addonName));
    const archive = archiver('zip', { zlib: { level: 9 } });

    output.on('close', () => {
      console.log(`  ${addonName}: ${archive.pointer()} bytes`);
      resolve();
    });
    archive.on('error', reject);
    archive.pipe(output);

    for (const file of mcpackFiles) {
      archive.file(path.join(__dirname, file), { name: file });
    }
    archive.finalize();
  });
}

function collectUuids(manifest) {
  return [manifest.header.uuid, ...manifest.modules.map((m) => m.uuid)];
}

function packDependency(manifest) {
  return manifest.dependencies.find((dep) => dep.uuid && !dep.module_name);
}

function verify(manifests, scriptPath) {
  const errors = [];

  const allUuids = [
    ...collectUuids(manifests.full.bp),
    ...collectUuids(manifests.full.rp),
    ...collectUuids(manifests['no-net'].bp),
    ...collectUuids(manifests['no-net'].rp),
  ];
  const seen = new Set();
  for (const uuid of allUuids) {
    if (seen.has(uuid)) errors.push(`Duplicate UUID across packs: ${uuid}`);
    seen.add(uuid);
  }

  const noNetBpModules = manifests['no-net'].bp.dependencies
    .filter((dep) => dep.module_name)
    .map((dep) => dep.module_name);
  for (const stripped of manifestConfig.VARIANTS['no-net'].stripModules) {
    if (noNetBpModules.includes(stripped)) {
      errors.push(`no-net BP manifest still declares stripped module: ${stripped}`);
    }
  }

  for (const variant of ['full', 'no-net']) {
    const bpDepOnRp = packDependency(manifests[variant].bp);
    if (bpDepOnRp.uuid !== manifests[variant].rp.header.uuid) {
      errors.push(`${variant} BP->RP dependency UUID mismatch`);
    }
    const rpDepOnBp = packDependency(manifests[variant].rp);
    if (rpDepOnBp.uuid !== manifests[variant].bp.header.uuid) {
      errors.push(`${variant} RP->BP dependency UUID mismatch`);
    }
  }

  const bundleSource = fs.readFileSync(scriptPath, 'utf8');
  const staticImport = /^\s*import\b[^\n]*from\s*['"]@minecraft\/server-(net|admin)['"]/m;
  if (staticImport.test(bundleSource)) {
    errors.push(
      'Bundled main.js contains a static import of @minecraft/server-net or server-admin; ' +
        'these must be loaded via dynamic import() so the shared bundle runs on no-net servers'
    );
  }

  if (errors.length > 0) {
    throw new Error('Manifest verification failed:\n  - ' + errors.join('\n  - '));
  }
  console.log('  Verification passed.');
}

async function bundle() {
  console.log('Creating BDS pack bundles...');

  const semver = require('./package.json').version;
  const encoded = VersionEncoder.encode(semver);
  console.log(`  Version: ${semver} -> [${encoded.array.join(', ')}]`);

  const builder = new ManifestBuilder(encoded, semver);
  const manifests = {
    full: { bp: builder.bp('full'), rp: builder.rp('full') },
    'no-net': { bp: builder.bp('no-net'), rp: builder.rp('no-net') },
  };

  const scriptPath = path.join(packDir('bp'), 'scripts', 'main.js');
  verify(manifests, scriptPath);

  const files = {
    fullBp: 'bedrock-voice-chat-bp.mcpack',
    noNetBp: 'bedrock-voice-chat-bp-no-net.mcpack',
    fullRp: 'bedrock-voice-chat-rp.mcpack',
    noNetRp: 'bedrock-voice-chat-rp-no-net.mcpack',
  };

  await createPackZip(files.fullBp, 'bp', manifests.full.bp);
  await createPackZip(files.noNetBp, 'bp', manifests['no-net'].bp);
  await createPackZip(files.fullRp, 'rp', manifests.full.rp);
  await createPackZip(files.noNetRp, 'rp', manifests['no-net'].rp);

  console.log('Creating mcaddons...');
  await createAddon('bedrock-voice-chat.mcaddon', [files.fullBp, files.fullRp]);
  await createAddon('bedrock-voice-chat-no-net.mcaddon', [files.noNetBp, files.noNetRp]);

  console.log('Bundle complete.');
}

bundle().catch((err) => {
  console.error('Bundle failed:', err);
  process.exit(1);
});
