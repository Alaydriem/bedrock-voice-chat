const archiver = require('archiver');
const fs = require('fs');
const path = require('path');

const output = fs.createWriteStream(path.join(__dirname, 'bedrock-voice-chat.zip'));
const archive = archiver('zip', { zlib: { level: 9 } });

console.log('Creating bedrock-voice-chat.zip...');

output.on('close', () => {
  console.log(`Bundle created successfully: ${archive.pointer()} total bytes`);
});

archive.on('error', (err) => {
  throw err;
});

archive.pipe(output);

// Add only the files we want in the bundle
// Add main.js (bundled with all dependencies)
archive.file(path.join(__dirname, 'scripts', 'main.js'), { name: 'scripts/main.js' });

// Add texts directory
archive.directory(path.join(__dirname, 'texts'), 'texts');

// Add manifest and icon
archive.file(path.join(__dirname, 'manifest.json'), { name: 'manifest.json' });
archive.file(path.join(__dirname, 'pack_icon.png'), { name: 'pack_icon.png' });

// Add README if it exists
const readmePath = path.join(__dirname, 'README.md');
if (fs.existsSync(readmePath)) {
  archive.file(readmePath, { name: 'README.md' });
}

archive.finalize();
