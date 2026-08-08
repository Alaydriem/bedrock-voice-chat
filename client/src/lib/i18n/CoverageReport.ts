import Sources from "./Sources.ts";

export interface FileMarking {
  path: string;
  marked: number;
  unmarked: number;
}

export interface MarkingTotals {
  marked: number;
  unmarked: number;
  files: FileMarking[];
}

export interface LocaleCoverage {
  locale: string;
  translated: number;
  total: number;
}

/**
 * Counts how much of the interface has been marked for translation.
 *
 * This is the progress meter for the migration, and it exists because the failure it
 * measures is invisible: an unmarked string renders as flawless English, so nothing in a
 * build, a test run, or a screenshot distinguishes "not translated yet" from "done".
 */
export default class CoverageReport {
  // `>Some words<` in markup, and copy-bearing attributes. Both require a capital and
  // three following characters, which is what separates a sentence from `{value}`, a CSS
  // class, or a single-letter label.
  // Both capture their text, so the proper-noun rule can read it.
  static readonly #TEXT_NODE = />(\s*[A-Z][a-zA-Z0-9 ,.'!?:·—–&-]{3,}?)</g;
  static readonly #COPY_ATTRIBUTE =
    /(?:label|title|placeholder|note|aria-label)="([A-Z][^"]{3,})"/g;
  static readonly #MARKER = /I18n\.t[cfn]?\(/g;

  // Copy held in TypeScript catalogs — FaultCatalog's titles and messages, pane names.
  // Two words minimum, because a one-word literal is far more often a key, an event name
  // or a CSS class than a sentence.
  static readonly #SCRIPT_COPY = /"[A-Z][a-zA-Z0-9,.'!?:-]*(?: [a-zA-Z0-9,.'!?:-]+){1,}"/g;

  // `"M5.5 11.5a6.5 6.5 0 0 0 13 0"` satisfies every rule above — capital, spaces, several
  // words — and appears 37 times in the icon set alone. Command letters are interleaved
  // with the coordinates rather than only leading, so they are permitted throughout.
  static readonly #SVG_PATH = /^"[MLHVCSQTAZ][MLHVCSQTAZmlhvcsqtaz\d.,\s-]*"$/;

  static markingOf(path: string, source: string): FileMarking {
    const marked = CoverageReport.#count(source, CoverageReport.#MARKER);
    const unmarked = path.endsWith(".svelte")
      ? CoverageReport.#markup(source)
      : CoverageReport.#scriptCopy(source);

    return { path, marked, unmarked };
  }

  // A marker and the one or two literals it takes. Removed before copy is counted: the
  // argument to `I18n.t("Connection Refused")` is still a capitalised multi-word literal,
  // so a catalog that had just been fully migrated would report no progress at all.
  static readonly #MARKED_CALL = /I18n\.t[cfn]?\(\s*(?:"(?:[^"\\]|\\.)*"\s*,?\s*){1,2}/g;

  // Prose in a comment is documentation, not interface. `"I can't hear anyone"` explaining
  // why deafen ranks above mute is not a string anybody will ever read on screen.
  static readonly #COMMENT = /\/\*[\s\S]*?\*\/|\/\/[^\n]*/g;

  static #scriptCopy(source: string): number {
    const unmarked = source
      .replace(CoverageReport.#COMMENT, (whole) => " ".repeat(whole.length))
      .replace(CoverageReport.#MARKED_CALL, (whole) => " ".repeat(whole.length));
    let count = 0;

    for (const match of unmarked.matchAll(CoverageReport.#SCRIPT_COPY)) {
      if (CoverageReport.#SVG_PATH.test(match[0])) continue;
      if (Sources.isProperNoun(match[0].slice(1, -1))) continue;
      // A literal opening a log call goes to a file, not to a reader.
      if (Sources.LOG_CALL.test(unmarked.slice(0, match.index))) continue;
      count += 1;
    }

    return count;
  }

  static totals(files: FileMarking[]): MarkingTotals {
    return {
      marked: files.reduce((sum, file) => sum + file.marked, 0),
      unmarked: files.reduce((sum, file) => sum + file.unmarked, 0),
      files: files.filter((file) => file.unmarked > 0).sort((a, b) => b.unmarked - a.unmarked),
    };
  }

  static percent(part: number, whole: number): number {
    return whole === 0 ? 100 : Math.round((part / whole) * 1000) / 10;
  }

  static markedPercent(totals: MarkingTotals): number {
    return CoverageReport.percent(totals.marked, totals.marked + totals.unmarked);
  }

  /**
   * Renders the comment body.
   *
   * `baseline` is the same measurement taken on the pull request's base commit. Absent on
   * a push build, where there is nothing to compare against.
   */
  static render(
    marking: MarkingTotals,
    locales: LocaleCoverage[],
    baseline?: { marking: MarkingTotals; locales: LocaleCoverage[] },
  ): string {
    const lines: string[] = ["## Translation coverage", ""];

    lines.push(...CoverageReport.#markingSection(marking, baseline?.marking));
    lines.push("", ...CoverageReport.#localeSection(locales, baseline?.locales));

    if (marking.files.length > 0) {
      lines.push("", "<details><summary>Largest unmarked files</summary>", "");
      lines.push("| File | Unmarked |", "|---|---:|");
      for (const file of marking.files.slice(0, 15)) {
        lines.push(`| \`${file.path}\` | ${file.unmarked} |`);
      }
      lines.push("", "</details>");
    }

    return lines.join("\n");
  }

  static #markingSection(current: MarkingTotals, baseline?: MarkingTotals): string[] {
    const total = current.marked + current.unmarked;
    const percent = CoverageReport.markedPercent(current);
    const lines = [
      "### Strings marked for translation",
      "",
      "| | Count |",
      "|---|---:|",
      `| Marked | ${current.marked} |`,
      `| Not yet marked | ${current.unmarked} |`,
      `| **Coverage** | **${percent}%** of ${total} |`,
      "",
      "_Markup counts are exact. TypeScript counts are a close estimate — a two-word " +
        "literal is usually copy, but some are log lines._",
    ];

    if (baseline === undefined) return lines;

    const before = CoverageReport.markedPercent(baseline);
    const delta = Math.round((percent - before) * 10) / 10;

    lines.push("");
    if (delta < 0) {
      const added = current.unmarked - baseline.unmarked;
      lines.push(
        `> **Coverage fell ${CoverageReport.#points(Math.abs(delta))}** ` +
          `(${before}% → ${percent}%). ${added} more ` +
          `${added === 1 ? "string is" : "strings are"} unmarked than on the base branch.`,
      );
    } else if (delta > 0) {
      lines.push(
        `> Coverage rose ${CoverageReport.#points(delta)} (${before}% → ${percent}%).`,
      );
    } else {
      lines.push(`> Coverage unchanged at ${percent}%.`);
    }

    return lines;
  }

  static #points(value: number): string {
    return `${value} ${value === 1 ? "point" : "points"}`;
  }

  static #localeSection(current: LocaleCoverage[], baseline?: LocaleCoverage[]): string[] {
    if (current.length === 0) {
      return [
        "### Locales",
        "",
        "No `.po` files yet. Every string falls back to its English source.",
      ];
    }

    const lines = ["### Locales", "", "| Locale | Translated | Coverage | Ships |", "|---|---:|---:|:-:|"];

    for (const locale of current) {
      const percent = CoverageReport.percent(locale.translated, locale.total);
      const was = baseline?.find((entry) => entry.locale === locale.locale);
      const drop =
        was !== undefined && percent < CoverageReport.percent(was.translated, was.total);

      lines.push(
        `| \`${locale.locale}\`${drop ? " ⚠️" : ""} | ${locale.translated} / ${locale.total} | ` +
          `${percent}% | ${percent >= 90 ? "yes" : "no"} |`,
      );
    }

    lines.push("", "_Ships at 90% or above. A half-translated locale reads as a broken app rather than an English one._");
    return lines;
  }

  static #markup(source: string): number {
    let count = 0;

    // Script blocks are excluded, not merely deprioritised: `() => Promise<boolean>` reads
    // as the text node `> Promise<` and would be counted as untranslated copy forever.
    for (const part of source.split(/(<script[\s\S]*?<\/script>|<style[\s\S]*?<\/style>)/)) {
      if (part.startsWith("<script") || part.startsWith("<style")) continue;

      for (const pattern of [CoverageReport.#TEXT_NODE, CoverageReport.#COPY_ATTRIBUTE]) {
        for (const match of part.matchAll(new RegExp(pattern.source, pattern.flags))) {
          const text = (match[1] ?? "").trim();
          if (text.length >= 4 && !Sources.isProperNoun(text)) count += 1;
        }
      }
    }

    return count;
  }

  static #count(source: string, pattern: RegExp): number {
    return source.match(new RegExp(pattern.source, pattern.flags))?.length ?? 0;
  }
}
