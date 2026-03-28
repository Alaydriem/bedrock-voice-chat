const archiver = require('archiver');
const fs = require('fs');
const path = require('path');

function createZip(name, sourceDir) {
  return new Promise((resolve, reject) => {
    const output = fs.createWriteStream(path.join(__dirname, name));
    const archive = archiver('zip', { zlib: { level: 9 } });

    output.on('close', () => {
      console.log(`  ${name}: ${archive.pointer()} bytes`);
      resolve();
    });

    archive.on('error', reject);
    archive.pipe(output);

    // Add all contents at root level (not nested under the source dir name)
    archive.directory(path.join(__dirname, sourceDir), false);
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

  const bpPack = 'bedrock-voice-chat-bp.mcpack';
  const rpPack = 'bedrock-voice-chat-rp.mcpack';

  await createZip(bpPack, 'bp');
  await createZip(rpPack, 'rp');

  console.log('Creating mcaddon...');
  await createAddon('bedrock-voice-chat.mcaddon', [bpPack, rpPack]);

  console.log('Bundle complete.');
}

bundle().catch((err) => {
  console.error('Bundle failed:', err);
  process.exit(1);
});
