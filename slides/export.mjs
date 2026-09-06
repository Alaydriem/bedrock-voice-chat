#!/usr/bin/env node
/**
 * Renders every slide in deck.html to a PNG.
 *
 * The stage is always 1920x1080. Resolution comes from the browser's device
 * scale factor, so 4K is one flag and nothing in the deck changes:
 *
 *   node slides/export.mjs              3840 x 2160  (default)
 *   node slides/export.mjs --scale=1    1920 x 1080
 *   node slides/export.mjs --scale=4    7680 x 4320
 *
 *   --clean       remove any earlier export first
 *   --out=DIR     write somewhere other than slides/out
 *   --only=6,7    export just those slide numbers
 *   --chrome      keep the footer and progress bar (off by default: a still
 *                 dropped on a timeline usually wants a clean plate)
 *
 * Uses whatever Chromium build is already installed. No download, no
 * node_modules, no Playwright.
 */

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const DECK = join(HERE, 'deck.html');

/* ---- arguments --------------------------------------------------------- */

const argv = process.argv.slice(2);
const flag = (name, fallback) => {
  const hit = argv.find((a) => a.startsWith(`--${name}=`));
  return hit ? hit.slice(name.length + 3) : fallback;
};
const has = (name) => argv.includes(`--${name}`);

const scale = Number(flag('scale', 2));
const outDir = resolve(flag('out', join(HERE, 'out')));
const only = flag('only', '')
  .split(',')
  .map((n) => Number(n.trim()))
  .filter(Boolean);

if (!Number.isFinite(scale) || scale < 1 || scale > 4) {
  console.error('--scale must be between 1 and 4');
  process.exit(1);
}

/* ---- the browser ------------------------------------------------------- *
 * Any Chromium takes --headless and --screenshot. Edge ships with Windows,
 * so on this machine there is nothing to install.                          */

const CANDIDATES = [
  process.env.BVC_SLIDES_BROWSER,
  'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
  'C:/Program Files/Microsoft/Edge/Application/msedge.exe',
  'C:/Program Files/Google/Chrome/Application/chrome.exe',
  'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
  '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/usr/bin/microsoft-edge',
  '/usr/bin/google-chrome',
  '/usr/bin/chromium',
  '/usr/bin/chromium-browser',
].filter(Boolean);

const browser = CANDIDATES.find((p) => existsSync(p));

if (!browser) {
  console.error('No Chromium-based browser found. Looked in:');
  CANDIDATES.forEach((p) => console.error('  ' + p));
  console.error('\nSet BVC_SLIDES_BROWSER to the executable and run again.');
  process.exit(1);
}

/* ---- the slides -------------------------------------------------------- *
 * data-name on each <section> becomes the filename, so a renamed or
 * reordered slide exports under a name that still means something.        */

const html = readFileSync(DECK, 'utf8');
const names = [...html.matchAll(/<section class="slide"[^>]*data-name="([^"]+)"/g)].map(
  (m) => m[1],
);

if (names.length === 0) {
  console.error('No slides found in deck.html. Every section needs data-name.');
  process.exit(1);
}

/* ---- render ------------------------------------------------------------ */

if (has('clean') && existsSync(outDir)) rmSync(outDir, { recursive: true, force: true });
mkdirSync(outDir, { recursive: true });

// A throwaway profile. Without it the launch can attach to an already running
// Edge and exit without writing anything.
const profile = mkdtempSync(join(tmpdir(), 'bvc-slides-'));

const deckUrl = pathToFileURL(DECK).href;
const pad = (n) => String(n).padStart(2, '0');
const wanted = (n) => only.length === 0 || only.includes(n);

/**
 * msedge.exe hands the work to a child and returns, so the launcher exits
 * roughly a second before the PNG is on disk. Wait for the file to appear and
 * for its size to settle, rather than trusting the exit.
 */
const waitForFile = (file, timeoutMs = 30000) => {
  const deadline = Date.now() + timeoutMs;
  let lastSize = -1;
  let stableFor = 0;

  while (Date.now() < deadline) {
    if (existsSync(file)) {
      const { size } = statSync(file);
      if (size > 0 && size === lastSize) {
        stableFor += 1;
        if (stableFor >= 2) return true;
      } else {
        stableFor = 0;
      }
      lastSize = size;
    }
    // Synchronous sleep: this script is a straight line and has nothing else
    // to do while the browser paints.
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 150);
  }

  return false;
};

console.log(`${browser}`);
console.log(`${1920 * scale} x ${1080 * scale}  ->  ${outDir}\n`);

let written = 0;

try {
  for (const [i, name] of names.entries()) {
    const n = i + 1;
    if (!wanted(n)) continue;

    const file = join(outDir, `${pad(n)}-${name}.png`);
    const url = `${deckUrl}?s=${n}${has('chrome') ? '' : '&chrome=0'}`;

    // A stale file from an earlier run would make the wait below pass instantly.
    if (existsSync(file)) rmSync(file);

    execFileSync(
      browser,
      [
        '--headless=new',
        '--disable-gpu',
        '--hide-scrollbars',
        '--no-first-run',
        '--no-default-browser-check',
        `--user-data-dir=${profile}`,
        `--force-device-scale-factor=${scale}`,
        '--window-size=1920,1080',
        '--virtual-time-budget=3000',
        `--screenshot=${file}`,
        url,
      ],
      { stdio: 'ignore' },
    );

    if (!waitForFile(file)) throw new Error(`browser wrote nothing for slide ${n}`);

    console.log(`  ${pad(n)}  ${name}`);
    written += 1;
  }
} finally {
  rmSync(profile, { recursive: true, force: true });
}

console.log(`\n${written} slide${written === 1 ? '' : 's'} exported.`);
