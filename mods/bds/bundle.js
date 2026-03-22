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

async function bundle() {
  console.log('Creating BDS pack bundles...');

  await createZip('bedrock-voice-chat-bp.zip', 'bp');
  await createZip('bedrock-voice-chat-rp.zip', 'rp');

  console.log('Bundle complete.');
}

bundle().catch((err) => {
  console.error('Bundle failed:', err);
  process.exit(1);
});
