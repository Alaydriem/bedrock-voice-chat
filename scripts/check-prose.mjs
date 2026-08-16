/**
 * Flags the mechanical prose tics listed in STYLE.md.
 *
 * Regex only. It catches constructions, not judgement. A page can pass this and
 * still be badly written. A rule that fires on correct prose is a bug in the rule:
 * fix or scope the rule, do not annotate the page.
 *
 * Runs against src/content/docs/, not dist/: the source is what gets edited,
 * and a line number in a built HTML file helps nobody.
 *
 * Exits non-zero only when --strict is passed, so it can be adopted as a report
 * before it becomes a gate.
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join, relative } from 'node:path';

const ROOT = fileURLToPath(new URL('../src/content/docs', import.meta.url));
const STRICT = process.argv.includes('--strict');

const RULES = [
  {
    id: 'justification-tail',
    hint: 'Split into two sentences, or cut the clause.',
    re: /,\s+(so|because)\s+\w/gi,
  },
  {
    id: 'which-means',
    hint: 'State the consequence as its own sentence.',
    re: /\bwhich means\b/gi,
  },
  {
    id: 'rather-than',
    hint: 'State what it is. Drop what it is not.',
    re: /\brather than\b/gi,
  },
  {
    id: 'attention-ranking',
    hint: 'Say the thing without ranking it.',
    re: /\b(worth (knowing|noting|setting|reading)|note that|it is worth|the (one )?thing (to know|worth)|importantly|crucially|of course|obviously)\b/gi,
  },
  {
    id: 'hedge',
    hint: 'Operator pages do not hedge. Give the instruction.',
    re: /\b(you may want to|it is recommended|we recommend|consider (setting|using|turning)|arguably|generally speaking)\b/gi,
  },
  {
    id: 'em-dash-aside',
    hint: 'Use a period or parentheses.',
    // Only mid-sentence, between two ordinary words. `**Term** — definition` is a
    // list idiom, not the aside tic, and flagging it buries the real hits.
    re: /[a-z,)]\s[—–]\s[a-z]/g,
    paragraphOnly: true,
  },
  {
    id: 'meta-documentation',
    hint: 'Delete. The reader did not ask how the page was made.',
    // "This page is for <audience>" is an audience gate, not a remark about how
    // the page was written. Player and platform pages open with one on purpose.
    // The rule still catches prose about the documentation's construction.
    re: /\b(this page(?! is for\b)|what follows|the (section|page) below|as (mentioned|described) above|restate[sd]?|generated from (the )?source)\b/gi,
  },
  {
    id: 'moralizing',
    hint: 'Give the instruction directly.',
    // `be sure to` survives inside an aside. A :::caution or :::tip exists to
    // press a point, and the flat imperative reads wrong once it is in one.
    // Running prose still has to give the instruction directly.
    re: /\b(you are responsible for|make sure you|do not forget|remember to)\b/gi,
    proseOnly: true,
  },
  {
    id: 'moralizing-strong',
    hint: 'Give the instruction directly, or move it into an aside.',
    re: /\b(be sure to)\b/gi,
    proseOnly: true,
  },
];

/**
 * Section titles on Arch-voice pages must be a noun phrase or a task, never a claim
 * about the product. "TLS is required" becomes "TLS".
 *
 * Player and start pages are exempt wholesale. Apple Support voice titles a section
 * with the reader's problem — "The record button does nothing", "I cannot sign in" —
 * and those are the target style there, not defects.
 *
 * A question is always exempt. An FAQ heading is supposed to be one.
 */
const ARCH_VOICE = /^wiki\/(server|reference|creator|platforms)\//;
const CLAIM_HEADING =
  /^#{2,4}\s+(?!.*\?\s*$).*\b(is|are|was|were|does|do|decides|means|belongs|cannot|can't|will|won't|should)\b/i;

const files = [];
(function walk(dir) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) walk(full);
    else if (entry.endsWith('.md') || entry.endsWith('.mdx')) files.push(full);
  }
})(ROOT);

const findings = [];

for (const file of files.sort()) {
  const rel = relative(ROOT, file).replace(/\\/g, '/');
  const lines = readFileSync(file, 'utf8').split(/\r?\n/);

  let inFence = false;
  let inFrontmatter = false;
  let inAside = false;

  lines.forEach((raw, index) => {
    const lineNo = index + 1;

    if (lineNo === 1 && raw.trim() === '---') {
      inFrontmatter = true;
      return;
    }
    if (inFrontmatter) {
      if (raw.trim() === '---') inFrontmatter = false;
      return;
    }
    if (/^\s*```/.test(raw)) {
      inFence = !inFence;
      return;
    }
    if (inFence) return;

    // A `:::type[Title]` opens an aside and a bare `:::` closes it. Some rules
    // are suspended inside one: an aside exists to press a point, and the flat
    // imperative the rule asks for reads wrong once the box is already shouting.
    if (/^\s*:::/.test(raw)) {
      inAside = !/^\s*:::\s*$/.test(raw);
      return;
    }

    // Inline code and link targets carry names we do not control.
    const line = raw.replace(/`[^`]*`/g, '``').replace(/\]\([^)]*\)/g, ']()');

    // A table row or a list item is structure. Some rules only mean anything in
    // running prose, where a writer had the choice of a second sentence.
    const isParagraph = !/^\s*([-*+]|\d+\.|\||>|#)/.test(raw);

    for (const rule of RULES) {
      if (rule.paragraphOnly && !isParagraph) continue;
      if (rule.proseOnly && inAside) continue;
      rule.re.lastIndex = 0;
      const match = rule.re.exec(line);
      if (match) {
        findings.push({ rel, lineNo, id: rule.id, hint: rule.hint, text: raw.trim() });
      }
    }

    if (ARCH_VOICE.test(rel) && CLAIM_HEADING.test(line)) {
      findings.push({
        rel,
        lineNo,
        id: 'claim-heading',
        hint: 'Retitle as a noun or a task.',
        text: raw.trim(),
      });
    }
  });
}

const byFile = new Map();
for (const f of findings) {
  if (!byFile.has(f.rel)) byFile.set(f.rel, []);
  byFile.get(f.rel).push(f);
}

const ranked = [...byFile.entries()].sort((a, b) => b[1].length - a[1].length);

for (const [rel, items] of ranked) {
  console.log(`\n${rel}  (${items.length})`);
  for (const item of items) {
    const text = item.text.length > 96 ? item.text.slice(0, 96) + '…' : item.text;
    console.log(`  ${String(item.lineNo).padStart(4)}  ${item.id.padEnd(20)} ${text}`);
  }
}

const counts = new Map();
for (const f of findings) counts.set(f.id, (counts.get(f.id) ?? 0) + 1);

console.log('\nby rule');
for (const [id, n] of [...counts.entries()].sort((a, b) => b[1] - a[1])) {
  console.log(`  ${String(n).padStart(4)}  ${id}`);
}
console.log(`\n${findings.length} findings across ${byFile.size} of ${files.length} pages`);

if (STRICT && findings.length > 0) process.exit(1);
