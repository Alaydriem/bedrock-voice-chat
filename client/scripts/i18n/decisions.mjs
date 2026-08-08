import { readFileSync, writeFileSync } from "node:fs";
import { glob } from "node:fs/promises";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import Sources from "../../src/lib/i18n/Sources.ts";

const CLIENT = fileURLToPath(new URL("../..", import.meta.url));

/**
 * Lists every string still outside the catalog, for a human to rule on.
 *
 * The coverage report counts; this names. Whether a literal is copy, a log line, a product
 * name or a key is a judgement no pattern makes reliably, and guessing wrong is worse in
 * both directions — a translated log line is noise, an untranslated sentence is a bug.
 */
const TEXT_NODE = />(\s*[A-Z][^<>{}]*?)</g;
const ATTRIBUTE = /\b(label|title|placeholder|note|aria-label)="([A-Z][^"{}]*?)"/g;
const SCRIPT_COPY = /"[A-Z][a-zA-Z0-9,.'!?:-]*(?: [a-zA-Z0-9,.'!?:-]+){1,}"/g;
const SVG_PATH = /^"[MLHVCSQTAZ][MLHVCSQTAZmlhvcsqtaz\d.,\s-]*"$/;
const MARKED_CALL = /I18n\.t[cfn]?\(\s*(?:"(?:[^"\\]|\\.)*"\s*,?\s*){1,2}/g;

function scriptBlocks(source) {
  return source.split(/(<script[\s\S]*?<\/script>)/);
}

function lineOf(source, index) {
  return source.slice(0, index).split("\n").length;
}

function reason(path, text) {
  if (/&[a-z]+;|&#\d+;/.test(text)) return "has an HTML entity";
  if (!/[a-z]{2}/.test(text)) return "no lowercase run — may be a key or an acronym";
  return "";
}

/**
 * Already ruled on, and so not a question any more.
 *
 * The counter applies these too. A list that keeps asking about `Discord` after it was
 * answered trains the reader to skim, which is how a real question gets missed.
 */
function settled(path, text, before) {
  if (Sources.isProperNoun(text)) return true;
  if (Sources.LOG_CALL.test(before)) return true;
  return false;
}

const rows = [];

for await (const absolute of glob(join(CLIENT, Sources.GLOB))) {
  const path = relative(CLIENT, absolute).split("\\").join("/");
  if (Sources.isIgnored(path)) continue;

  const source = readFileSync(absolute, "utf8");

  if (path.endsWith(".svelte")) {
    for (const part of scriptBlocks(source)) {
      if (part.startsWith("<script")) continue;
      const base = source.indexOf(part);

      for (const match of part.matchAll(TEXT_NODE)) {
        const text = match[1].trim().replace(/\s+/g, " ");
        if (text.length < 4 || settled(path, text, "")) continue;
        rows.push({ path, line: lineOf(source, base + match.index), text, kind: "markup", reason: reason(path, text) });
      }
      for (const match of part.matchAll(ATTRIBUTE)) {
        const text = match[2].trim().replace(/\s+/g, " ");
        if (text.length < 4 || settled(path, text, "")) continue;
        rows.push({ path, line: lineOf(source, base + match.index), text, kind: `attr ${match[1]}`, reason: reason(path, text) });
      }
    }
    continue;
  }

  // Comments and markers blanked rather than removed, so line numbers stay true.
  const stripped = source
    .replace(/\/\*[\s\S]*?\*\/|\/\/[^\n]*/g, (whole) => whole.replace(/[^\n]/g, " "))
    .replace(MARKED_CALL, (whole) => " ".repeat(whole.length));
  for (const match of stripped.matchAll(SCRIPT_COPY)) {
    if (SVG_PATH.test(match[0])) continue;
    const text = match[0].slice(1, -1);
    if (settled(path, text, stripped.slice(0, match.index))) continue;
    const line = lineOf(source, match.index);
    const context = source.split("\n")[line - 1]?.trim().slice(0, 70) ?? "";
    rows.push({ path, line, text, kind: "script", reason: reason(path, text), context });
  }
}

rows.sort((a, b) => a.path.localeCompare(b.path) || a.line - b.line);

const lines = [
  "# Strings awaiting a decision",
  "",
  `${rows.length} candidates the tooling will not rule on by itself.`,
  "",
  "Mark each **T** (translate), **N** (leave English — a product name, a key, a log line),",
  "or **D** (delete / not user-facing). Anything marked N gets added to the counter's ignore",
  "list so it stops being reported as outstanding work.",
  "",
];

let current = "";
for (const row of rows) {
  if (row.path !== current) {
    current = row.path;
    lines.push("", `## \`${current}\``, "", "| ? | Line | Kind | String | Note |", "|:-:|---:|---|---|---|");
  }
  const text = row.text.replace(/\|/g, "\\|");
  const note = row.reason || (row.context ? `\`${row.context.replace(/\|/g, "\\|")}\`` : "");
  lines.push(`|  | ${row.line} | ${row.kind} | ${text} | ${note} |`);
}

const out = join(CLIENT, "..", "docs", "superpowers", "2026-08-08-string-decisions.md");
writeFileSync(out, lines.join("\n"));
process.stdout.write(`${rows.length} candidates written to ${out}\n`);
