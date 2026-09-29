/**
 * Builds the published screenshot formats from the originals.
 *
 * Reads screenshots/wiki/*.png and writes AVIF, WebP and JPEG to
 * public/assets/wiki/. Nothing in screenshots/ is published, and nothing in
 * public/assets/wiki/ is committed: the originals are the source of truth.
 *
 * Pages reference the JPEG (/assets/wiki/name.jpg). The rehype plugin in
 * src/plugins/rehype-wiki-picture.mjs wraps it in a <picture> so a browser takes
 * AVIF, then WebP, then the JPEG. The JPEG path also keeps the raw-markdown and
 * llms.txt copies of a page pointing at a file that exists.
 *
 * Also writes src/data/wiki-images.json, the output size of each image, so the
 * plugin can set width and height and the page does not shift as images load.
 *
 * Skips an image whose outputs are newer than its original. Pass --force after
 * changing the settings below.
 */
import { existsSync, mkdirSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join, parse } from 'node:path';
import sharp from 'sharp';

const SRC = fileURLToPath(new URL('../screenshots/wiki', import.meta.url));
const OUT = fileURLToPath(new URL('../public/assets/wiki', import.meta.url));
const MANIFEST = fileURLToPath(new URL('../src/data/wiki-images.json', import.meta.url));
const FORCE = process.argv.includes('--force');

// The wiki column is under 800 CSS px wide. 1600 covers a 2x display.
const MAX_WIDTH = 1600;

const FORMATS = {
  avif: (image) => image.avif({ quality: 50, effort: 6 }),
  webp: (image) => image.webp({ quality: 76, effort: 6 }),
  jpg: (image) => image.jpeg({ quality: 72, mozjpeg: true }),
};

const isFresh = (source, outputs) =>
  outputs.every((file) => existsSync(file) && statSync(file).mtimeMs >= statSync(source).mtimeMs);

const sizeOf = (file) => statSync(file).size;

async function build(name) {
  const source = join(SRC, `${name}.png`);
  const outputs = Object.keys(FORMATS).map((ext) => join(OUT, `${name}.${ext}`));
  const { width, height } = await sharp(source).metadata();
  const scale = Math.min(1, MAX_WIDTH / width);
  const size = { width: Math.round(width * scale), height: Math.round(height * scale) };

  if (!FORCE && isFresh(source, outputs)) return { name, size, built: false };

  await Promise.all(
    Object.entries(FORMATS).map(([ext, encode]) =>
      encode(sharp(source).resize({ width: MAX_WIDTH, withoutEnlargement: true })).toFile(
        join(OUT, `${name}.${ext}`),
      ),
    ),
  );
  return { name, size, built: true };
}

function removeOrphans(names) {
  const wanted = new Set(names);
  for (const file of readdirSync(OUT)) {
    const { name, ext } = parse(file);
    if (!(ext.slice(1) in FORMATS) || wanted.has(name)) continue;
    rmSync(join(OUT, file));
    console.log(`images: removed ${file}, no original`);
  }
}

if (!existsSync(SRC)) {
  console.error(`images: ${SRC} does not exist`);
  process.exit(1);
}

mkdirSync(OUT, { recursive: true });
mkdirSync(parse(MANIFEST).dir, { recursive: true });

const names = readdirSync(SRC)
  .filter((file) => file.endsWith('.png'))
  .map((file) => parse(file).name)
  .sort();

const results = [];
for (const name of names) results.push(await build(name));

removeOrphans(names);
writeFileSync(
  MANIFEST,
  `${JSON.stringify(Object.fromEntries(results.map((r) => [r.name, r.size])), null, 2)}\n`,
);

const built = results.filter((r) => r.built).length;
const originals = names.reduce((sum, name) => sum + sizeOf(join(SRC, `${name}.png`)), 0);
const published = (ext) =>
  names.reduce((sum, name) => sum + sizeOf(join(OUT, `${name}.${ext}`)), 0);
const mb = (bytes) => `${(bytes / 1048576).toFixed(1)} MB`;

console.log(
  `images: ${names.length} originals (${mb(originals)}), ${built} rebuilt. ` +
    `avif ${mb(published('avif'))}, webp ${mb(published('webp'))}, jpg ${mb(published('jpg'))}`,
);
