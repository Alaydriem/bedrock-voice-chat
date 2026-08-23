import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { glob } from "node:fs/promises";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { GettextExtractor, JsExtractors } from "gettext-extractor";
import SvelteSource from "../../src/lib/i18n/SvelteSource.ts";
import Sources from "../../src/lib/i18n/Sources.ts";

const CLIENT = fileURLToPath(new URL("../..", import.meta.url));
const POT = join(CLIENT, "locales", "bvc.pot");


const HEADERS = {
  "Project-Id-Version": "bvc",
  "MIME-Version": "1.0",
  "Content-Type": "text/plain; charset=UTF-8",
  "Content-Transfer-Encoding": "8bit",
  "Plural-Forms": "nplurals=2; plural=(n != 1);",
};

const checking = process.argv.includes("--check");

const extractor = new GettextExtractor();
const parser = extractor.createJsParser([
  JsExtractors.callExpression("I18n.t", { arguments: { text: 0 } }),
  JsExtractors.callExpression("I18n.tf", { arguments: { text: 0 } }),
  JsExtractors.callExpression("I18n.tc", { arguments: { context: 0, text: 1 } }),
  JsExtractors.callExpression("I18n.tn", { arguments: { text: 0, textPlural: 1 } }),
]);


for await (const absolute of glob(join(CLIENT, Sources.GLOB))) {
  const reference = relative(CLIENT, absolute).split("\\").join("/");
  if (Sources.isIgnored(reference)) continue;

  const source = readFileSync(absolute, "utf8");

  if (reference.endsWith(".svelte")) {
    parser.parseString(SvelteSource.toTypeScript(source), reference);
  } else {
    parser.parseString(source, reference);
  }
}

const generated = extractor.getPotString(HEADERS);

// Compared in process rather than through `git diff`, which reports nothing for a file
// that is untracked and so would pass the gate on a catalog that was never committed.
if (checking) {
  const current = existsSync(POT) ? readFileSync(POT, "utf8") : null;

  if (current !== generated) {
    const reason = current === null ? "does not exist" : "is out of date";
    process.stderr.write(`i18n: locales/bvc.pot ${reason}. Run \`yarn i18n:extract\`.\n`);
    process.exit(1);
  }

  process.stdout.write("i18n: catalog is current\n");
} else {
  writeFileSync(POT, generated);
  extractor.printStats();
}
