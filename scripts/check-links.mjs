/**
 * Fails the build on an internal link that does not resolve to a built page.
 *
 * Starlight does not validate links, so a renamed or not-yet-written page is
 * otherwise only found by clicking it. Run after `astro build`, against dist/.
 *
 * Pages still to be written are expected to fail here — that is the point of
 * the PLANNED list: an unwritten page is a known gap, a typo is not, and the
 * two should not look the same.
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join, relative } from 'node:path';

const DIST = fileURLToPath(new URL('../dist', import.meta.url));

/** Links to pages not yet authored. Remove an entry as its page lands. */
const PLANNED = new Set([]);

const html = [];
(function walk(dir) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) walk(full);
    else if (entry.endsWith('.html')) html.push(full);
  }
})(DIST);

/** Every URL the build actually produced. */
const pages = new Set(
  html.map((f) => {
    const rel = relative(DIST, f).replace(/\\/g, '/');
    return '/' + rel.replace(/index\.html$/, '').replace(/\.html$/, '/');
  }),
);

const broken = new Map();
const stale = new Set();

for (const file of html) {
  const from = '/' + relative(DIST, file).replace(/\\/g, '/').replace(/index\.html$/, '');
  for (const m of readFileSync(file, 'utf8').matchAll(/href="(\/[^"#]*)(#[^"]*)?"/g)) {
    let href = m[1];
    if (!href.startsWith('/wiki/')) continue;
    if (!href.endsWith('/')) href += '/';
    if (pages.has(href)) {
      if (PLANNED.has(href)) stale.add(href);
      continue;
    }
    if (PLANNED.has(href)) continue;
    if (!broken.has(href)) broken.set(href, new Set());
    broken.get(href).add(from);
  }
}

for (const href of [...stale].sort()) {
  console.log(`note: ${href} now exists — drop it from PLANNED`);
}

if (broken.size) {
  console.error(`\n${broken.size} broken internal link(s):\n`);
  for (const [href, sources] of [...broken].sort()) {
    console.error(`  ${href}`);
    for (const s of [...sources].sort()) console.error(`      from ${s}`);
  }
  process.exit(1);
}

console.log(
  `links ok — ${pages.size} pages, ${PLANNED.size} planned pages not yet written`,
);
