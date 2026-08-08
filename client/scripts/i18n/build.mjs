import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { fileURLToPath } from "node:url";
import gettextParser from "gettext-parser";
import PackCompiler from "../../src/lib/i18n/PackCompiler.ts";
import PseudoLocale from "../../src/lib/i18n/PseudoLocale.ts";

const CLIENT = fileURLToPath(new URL("../..", import.meta.url));
const LOCALES = join(CLIENT, "locales");
const OUTPUT = join(CLIENT, "src-tauri", "resources", "i18n");

// A locale below this ships nothing. A half-translated locale reads as a broken app
// rather than an English one, which is the worse of the two failures.
const COVERAGE_THRESHOLD = 0.9;

rmSync(OUTPUT, { recursive: true, force: true });
mkdirSync(OUTPUT, { recursive: true });

const shipped = [];
const skipped = [];

for (const file of readdirSync(LOCALES).filter((name) => name.endsWith(".po"))) {
  const locale = basename(file, ".po");
  const po = gettextParser.po.parse(readFileSync(join(LOCALES, file)));
  const coverage = PackCompiler.coverage(po);

  if (coverage < COVERAGE_THRESHOLD) {
    skipped.push(`${locale} (${Math.round(coverage * 100)}%)`);
    continue;
  }

  writeFileSync(join(OUTPUT, `${locale}.json`), JSON.stringify(PackCompiler.compile(locale, po)));
  shipped.push(`${locale} (${Math.round(coverage * 100)}%)`);
}

const pot = gettextParser.po.parse(readFileSync(join(LOCALES, "bvc.pot")));
writeFileSync(
  join(OUTPUT, `${PseudoLocale.LOCALE}.json`),
  JSON.stringify(PseudoLocale.build(pot)),
);

process.stdout.write(`i18n: shipped ${shipped.join(", ") || "none"}\n`);
if (skipped.length > 0) {
  process.stdout.write(
    `i18n: below ${COVERAGE_THRESHOLD * 100}%, not shipped - ${skipped.join(", ")}\n`,
  );
}
