import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { glob } from "node:fs/promises";
import { basename, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import gettextParser from "gettext-parser";
import CoverageReport from "../../src/lib/i18n/CoverageReport.ts";
import Sources from "../../src/lib/i18n/Sources.ts";
import PackCompiler from "../../src/lib/i18n/PackCompiler.ts";

const CLIENT = fileURLToPath(new URL("../..", import.meta.url));


function flag(name) {
  const index = process.argv.indexOf(name);
  return index === -1 ? null : process.argv[index + 1];
}


async function marking(clientDir) {
  const files = [];

  for await (const absolute of glob(join(clientDir, Sources.GLOB))) {
    const path = relative(clientDir, absolute).split("\\").join("/");
    if (Sources.isIgnored(path)) continue;

    files.push(CoverageReport.markingOf(path, readFileSync(absolute, "utf8")));
  }

  return CoverageReport.totals(files);
}

function locales(clientDir) {
  const directory = join(clientDir, "locales");
  if (!existsSync(directory)) return [];

  return readdirSync(directory)
    .filter((name) => name.endsWith(".po"))
    .map((name) => {
      const po = gettextParser.po.parse(readFileSync(join(directory, name)));
      const total = Object.values(po.translations)
        .flatMap((messages) => Object.values(messages))
        .filter((message) => message.msgid !== "").length;

      return {
        locale: basename(name, ".po"),
        translated: Math.round(PackCompiler.coverage(po) * total),
        total,
      };
    });
}

async function measure(clientDir) {
  return { marking: await marking(clientDir), locales: locales(clientDir) };
}

const current = await measure(CLIENT);

// The pull request's base commit, checked out beside the workspace by the workflow. Absent
// on a push build, where a delta would have nothing to be relative to.
const baselineDir = flag("--baseline");
const baseline =
  baselineDir !== null && existsSync(join(baselineDir, "src"))
    ? await measure(baselineDir)
    : undefined;

const body = CoverageReport.render(current.marking, current.locales, baseline);

const out = flag("--out");
if (out !== null) writeFileSync(out, body);

process.stdout.write(`${body}\n`);
