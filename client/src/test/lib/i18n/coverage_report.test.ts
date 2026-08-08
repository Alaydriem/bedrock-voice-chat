import { expect, test } from "vitest";
import CoverageReport, {
  type LocaleCoverage,
  type MarkingTotals,
} from "../../../lib/i18n/CoverageReport.ts";

const SVELTE = [
  '<script lang="ts">',
  '  import { I18n } from "$lib/i18n";',
  "</script>",
  "<h1>Connect to a server</h1>",
  '<button title="Sign in again">{I18n.t("Sign In")}</button>',
  "<span>{value}</span>",
  "<span>{I18n.tn(\"{count} player\", \"{count} players\", n)}</span>",
].join("\n");

function totals(marked: number, unmarked: number): MarkingTotals {
  return { marked, unmarked, files: [] };
}

test("marked call sites are counted across every marker form", () => {
  const marking = CoverageReport.markingOf("SignInScreen.svelte", SVELTE);

  expect(marking.marked).toBe(2);
});

test("visible text and copy attributes are counted as not yet marked", () => {
  const marking = CoverageReport.markingOf("SignInScreen.svelte", SVELTE);

  expect(marking.unmarked).toBe(2);
});

test("an interpolated value is not mistaken for copy", () => {
  const marking = CoverageReport.markingOf("x.svelte", "<span>{value}</span>");

  expect(marking.unmarked).toBe(0);
});

test("copy held in a TypeScript catalog is counted", () => {
  const source = [
    "static readonly DEFINITIONS = {",
    '  AUTH01: { title: "Connection Refused", message: "The server refused this." },',
    "};",
  ].join("\n");

  expect(CoverageReport.markingOf("FaultCatalog.ts", source).unmarked).toBe(2);
});

// Without this the counter reports no progress for a catalog that was just migrated: the
// argument to a marker is still a capitalised multi-word literal.
test("a literal already inside a marker is not also counted as unmarked", () => {
  const source = 'title: I18n.t("Connection Refused"), message: I18n.t("The server refused this.")';

  const marking = CoverageReport.markingOf("FaultCatalog.ts", source);
  expect(marking.marked).toBe(2);
  expect(marking.unmarked).toBe(0);
});

test("a context marker consumes both of its literals", () => {
  const source = 'caption: I18n.tc("client", "TOO OLD"), other: "Still Unmarked Copy"';

  const marking = CoverageReport.markingOf("FaultCatalog.ts", source);
  expect(marking.marked).toBe(1);
  expect(marking.unmarked).toBe(1);
});

// Ruled N on 2026-08-08: translating a product name yields text matching nothing the
// reader can find, and a log line reaches a file rather than a person.
test("product and platform names are not counted as outstanding work", () => {
  const markup = [
    "<span>Windows · macOS · Linux</span>",
    "<span>Java + Geyser & Floodgate</span>",
    "<span>TLS 1.3 · mTLS</span>",
    "<span>Bedrock Voice Chat</span>",
  ].join("\n");

  expect(CoverageReport.markingOf("x.svelte", markup).unmarked).toBe(0);
});

test("a sentence containing a product name is still counted", () => {
  const markup = "<span>Connect to Minecraft Realms and start talking</span>";

  expect(CoverageReport.markingOf("x.svelte", markup).unmarked).toBe(1);
});

// `() => Promise<boolean>` in a props interface reads as the text node `> Promise<`.
test("a type annotation in a script block is not markup", () => {
  const source = [
    '<script lang="ts">',
    "  interface Props { ontest?: () => Promise<boolean> }",
    "</script>",
    "<p>Real copy here</p>",
  ].join("\n");

  expect(CoverageReport.markingOf("x.svelte", source).unmarked).toBe(1);
});

test("prose inside a comment is documentation, not interface copy", () => {
  const source = [
    '// Leaving them deafened means the fix for "I can\'t hear anyone" is a restart.',
    "/**",
    ' * Every group was called "New group", so a user who declined to rename ended up lost.',
    " */",
    'const shown = "Really user facing copy";',
  ].join("\n");

  expect(CoverageReport.markingOf("x.ts", source).unmarked).toBe(1);
});

test("log arguments are not counted as copy", () => {
  const source = [
    'warn("Server returned 403 Forbidden");',
    'info("DiscordCallbackHandler: processing callback");',
    'reportError("Device code expired. Please try again.");',
  ].join("\n");

  // Only the third reaches a reader.
  expect(CoverageReport.markingOf("x.ts", source).unmarked).toBe(1);
});

test("single-word literals are treated as keys rather than copy", () => {
  const source = 'const k = "Account"; emit("StreamEvent"); cls("rad-btn");';

  expect(CoverageReport.markingOf("x.ts", source).unmarked).toBe(0);
});

test("SVG path data is not counted as copy", () => {
  const source = 'const d = "M5.5 11.5a6.5 6.5 0 0 0 13 0"; const e = "M12 18v3.5";';

  expect(CoverageReport.markingOf("Icons.ts", source).unmarked).toBe(0);
});

test("markup rules do not apply to a script file", () => {
  expect(CoverageReport.markingOf("x.ts", "<h1>Not markup here</h1>").unmarked).toBe(0);
});

test("totals rank files by how much work is left in them", () => {
  const marking = CoverageReport.totals([
    { path: "small.svelte", marked: 0, unmarked: 2 },
    { path: "big.svelte", marked: 1, unmarked: 30 },
    { path: "done.svelte", marked: 5, unmarked: 0 },
  ]);

  expect(marking.marked).toBe(6);
  expect(marking.unmarked).toBe(32);
  expect(marking.files.map((file) => file.path)).toEqual(["big.svelte", "small.svelte"]);
});

test("a fully marked tree reports complete rather than dividing by zero", () => {
  expect(CoverageReport.markedPercent(totals(0, 0))).toBe(100);
});

test("the report names the drop when coverage falls", () => {
  const body = CoverageReport.render(totals(10, 90), [], {
    marking: totals(20, 80),
    locales: [],
  });

  expect(body).toContain("Coverage fell 10 points");
  expect(body).toContain("20% → 10%");
});

test("the report says so when coverage rises", () => {
  const body = CoverageReport.render(totals(30, 70), [], {
    marking: totals(20, 80),
    locales: [],
  });

  expect(body).toContain("Coverage rose 10 points");
});

// A localization feature reporting "1 points" in its own output is a poor advertisement.
test("a one point move is singular", () => {
  const body = CoverageReport.render(totals(21, 79), [], {
    marking: totals(20, 80),
    locales: [],
  });

  expect(body).toContain("rose 1 point (");
});

test("a single added string is singular too", () => {
  const body = CoverageReport.render(totals(20, 81), [], {
    marking: totals(21, 80),
    locales: [],
  });

  expect(body).toContain("1 more string is unmarked");
});

test("with no baseline the report states the position without a delta", () => {
  const body = CoverageReport.render(totals(6, 594), []);

  expect(body).toContain("**1%** of 600");
  expect(body).not.toContain("Coverage fell");
  expect(body).not.toContain("Coverage rose");
});

test("a locale at or above the gate is marked as shipping", () => {
  const locales: LocaleCoverage[] = [
    { locale: "de", translated: 95, total: 100 },
    { locale: "ru", translated: 50, total: 100 },
  ];
  const body = CoverageReport.render(totals(100, 0), locales);

  expect(body).toContain("| `de` | 95 / 100 | 95% | yes |");
  expect(body).toContain("| `ru` | 50 / 100 | 50% | no |");
});

test("a locale that lost ground against the base branch is flagged", () => {
  const body = CoverageReport.render(
    totals(100, 0),
    [{ locale: "de", translated: 80, total: 100 }],
    { marking: totals(100, 0), locales: [{ locale: "de", translated: 95, total: 100 }] },
  );

  expect(body).toContain("`de` ⚠️");
});

test("an empty catalog explains the fallback rather than showing an empty table", () => {
  const body = CoverageReport.render(totals(6, 594), []);

  expect(body).toContain("No `.po` files yet");
});
