/**
 * Regenerates the Bedrock version support table from the protocol matrix.
 *
 * The matrix is emitted by bedrock-protocol-rs:
 *
 *     cargo run --example version-matrix > src/data/version-matrix.json
 *
 * Runs as a prebuild step, so the published table always matches the JSON and
 * a stale hand-edit cannot survive a build. Everything outside the marked
 * region of the page is hand-written and left alone.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const ROOT = new URL('../', import.meta.url);
const DATA = fileURLToPath(new URL('src/data/version-matrix.json', ROOT));
const PAGE = fileURLToPath(
  new URL('src/content/docs/wiki/platforms/version-support.md', ROOT),
);

const START = '<!-- generated:version-table start -->';
const END = '<!-- generated:version-table end -->';

const matrix = JSON.parse(readFileSync(DATA, 'utf8'));

/** Versions a player could actually be running: public release, codec present. */
const supported = matrix.versions.filter((v) => v.released && v.has_codec);
/** Named, public, but no codec — the crate cannot speak these. */
const unsupported = matrix.versions.filter((v) => v.released && !v.has_codec);
/** Codec exists but the build is not public yet. */
const preview = matrix.versions.filter((v) => !v.released && v.client_version);

const newest = matrix.versions.find((v) => v.protocol === matrix.released_latest);

const rows = (list) =>
  list
    .map((v) => `| ${v.client_version} | \`v${v.protocol}\` |`)
    .join('\n');

const table = [
  START,
  '',
  '## Supported Bedrock versions',
  '',
  '| Minecraft Bedrock | Protocol |',
  '|---|---|',
  rows(supported),
  '',
  `**${newest.client_version} is the newest supported release.**`,
  '',
  '## Not supported',
  '',
  'These protocols are recognised but this build has no codec for them. The',
  'no-net Addon and Bedrock Voice Chat Connect will not work against a server running one.',
  '',
  '| Minecraft Bedrock | Protocol |',
  '|---|---|',
  rows(unsupported),
  ...(preview.length
    ? [
        '',
        '## Preview builds',
        '',
        'A codec exists, but no public Bedrock release speaks these yet.',
        '',
        '| Minecraft Bedrock | Protocol |',
        '|---|---|',
        rows(preview),
      ]
    : []),
  '',
  END,
].join('\n');

const page = readFileSync(PAGE, 'utf8');
const from = page.indexOf(START);
const to = page.indexOf(END);
if (from === -1 || to === -1) {
  throw new Error(`${PAGE} is missing the generated-table markers`);
}

const next = page.slice(0, from) + table + page.slice(to + END.length);
if (next !== page) {
  writeFileSync(PAGE, next);
  console.log('version-support.md: table regenerated');
} else {
  console.log('version-support.md: already current');
}
