const archiver = require('archiver');
const fs = require('fs');
const path = require('path');

const STRIPPED_MODULES = new Set(['@minecraft/server-net']);

function loadManifest() {
  const raw = fs.readFileSync(path.join(__dirname, 'bp', 'manifest.json'), 'utf8');
  return JSON.parse(raw);
}

function stripNetDeps(manifest) {
  const next = JSON.parse(JSON.stringify(manifest));
  if (Array.isArray(next.dependencies)) {
    next.dependencies = next.dependencies.filter(
      (dep) => !(dep && typeof dep.module_name === 'string' && STRIPPED_MODULES.has(dep.module_name))
    );
  }
  return next;
}

function createBpZip(outputName, manifestJson) {
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
      cwd: path.join(__dirname, 'bp'),
      ignore: ['manifest.json'],
      dot: false,
    });
    archive.append(JSON.stringify(manifestJson, null, 2) + '\n', { name: 'manifest.json' });
    archive.finalize();
  });
}

function createRpZip(outputName) {
  return new Promise((resolve, reject) => {
    const output = fs.createWriteStream(path.join(__dirname, outputName));
    const archive = archiver('zip', { zlib: { level: 9 } });

    output.on('close', () => {
      console.log(`  ${outputName}: ${archive.pointer()} bytes`);
      resolve();
    });
    archive.on('error', reject);
    archive.pipe(output);
    archive.directory(path.join(__dirname, 'rp'), false);
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

async function bundle() {
  console.log('Creating BDS pack bundles...');

  const fullManifest = loadManifest();
  const noNetManifest = stripNetDeps(fullManifest);

  const bpFull = 'bedrock-voice-chat-bp.mcpack';
  const bpNoNet = 'bedrock-voice-chat-bp-no-net.mcpack';
  const rpPack = 'bedrock-voice-chat-rp.mcpack';

  await createBpZip(bpFull, fullManifest);
  await createBpZip(bpNoNet, noNetManifest);
  await createRpZip(rpPack);

  console.log('Creating mcaddons...');
  await createAddon('bedrock-voice-chat.mcaddon', [bpFull, rpPack]);
  await createAddon('bedrock-voice-chat-no-net.mcaddon', [bpNoNet, rpPack]);

  console.log('Bundle complete.');
}

bundle().catch((err) => {
  console.error('Bundle failed:', err);
  process.exit(1);
});
