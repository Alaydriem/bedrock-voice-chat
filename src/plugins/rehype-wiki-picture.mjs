/**
 * Wraps each wiki screenshot in a <picture> that offers AVIF and WebP.
 *
 * A page writes the JPEG as raw HTML:
 *
 *   <div class="window"><img src="/assets/wiki/name.jpg" alt="..." /></div>
 *
 * scripts/images.mjs publishes name.avif and name.webp beside it. The JPEG stays
 * the <img> so the raw-markdown copy of the page and any browser without AVIF
 * still get a working image.
 *
 * Raw HTML reaches rehype plugins as `raw` nodes, not elements, so this rewrites
 * the string. A markdown ![](...) image is left as a plain JPEG.
 *
 * Width and height come from src/data/wiki-images.json, written by the same
 * script. Without it the images still render, and the page shifts as they load.
 */
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const MANIFEST = fileURLToPath(new URL('../data/wiki-images.json', import.meta.url));
const WIKI_IMG = /<img\b([^>]*?)\bsrc="\/assets\/wiki\/([a-z0-9-]+)\.jpg"([^>]*)>/g;

const readSizes = () => (existsSync(MANIFEST) ? JSON.parse(readFileSync(MANIFEST, 'utf8')) : {});

const toPicture = (name, before, after, size) => {
  const path = `/assets/wiki/${name}`;
  const dimensions = size ? ` width="${size.width}" height="${size.height}"` : '';
  return (
    `<picture>` +
    `<source srcset="${path}.avif" type="image/avif">` +
    `<source srcset="${path}.webp" type="image/webp">` +
    `<img${before}src="${path}.jpg"${after} loading="lazy" decoding="async"${dimensions}>` +
    `</picture>`
  );
};

const rewrite = (node, sizes) => {
  for (const child of node.children ?? []) {
    if (child.type === 'raw') {
      child.value = child.value.replace(WIKI_IMG, (_, before, name, after) =>
        toPicture(name, before, after, sizes[name]),
      );
    } else {
      rewrite(child, sizes);
    }
  }
};

export default function rehypeWikiPicture() {
  const sizes = readSizes();
  return (tree) => rewrite(tree, sizes);
}
