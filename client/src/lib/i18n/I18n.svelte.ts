import type { LanguagePack } from "../../js/bindings/LanguagePack";
import { CONTEXT_SEPARATOR } from "./Contract.ts";

const ENGLISH = "en";

// Module-scoped rather than a field, which is what makes the static methods reactive: a
// component that calls I18n.t() during render reads this and re-renders when it changes.
let pack = $state<LanguagePack | null>(null);

/**
 * The application's translation surface.
 *
 * Message ids are the English source strings, so with no pack loaded every method already
 * returns correct English. A missing translation therefore degrades one string rather than
 * one screen.
 */
export default class I18n {
  static get locale(): string {
    return pack?.locale ?? ENGLISH;
  }

  static adopt(next: LanguagePack | null): void {
    pack = next;
  }

  static t(msgid: string): string {
    return I18n.#one(msgid) ?? msgid;
  }

  static tc(context: string, msgid: string): string {
    return I18n.#one(`${context}${CONTEXT_SEPARATOR}${msgid}`) ?? msgid;
  }

  static tn(singular: string, plural: string, n: number): string {
    const category = new Intl.PluralRules(I18n.#bcp47()).select(n);
    const entry = pack?.m?.[singular];

    if (entry === undefined) {
      return category === "one" ? singular : plural;
    }
    if (typeof entry === "string") {
      return entry;
    }

    const declared = pack?.plural ?? [];
    const known = declared.indexOf(category);
    const index = known === -1 ? entry.length - 1 : Math.min(known, entry.length - 1);
    return entry[index] ?? plural;
  }

  static tf(msgid: string, params: Record<string, string | number>): string {
    return I18n.t(msgid).replace(/\{(\w+)\}/g, (whole, name: string) =>
      name in params ? String(params[name]) : whole,
    );
  }

  static #one(key: string): string | undefined {
    const entry = pack?.m?.[key];
    if (entry === undefined) return undefined;
    return typeof entry === "string" ? entry : entry[0];
  }

  static #bcp47(): string {
    return I18n.locale.replace("_", "-");
  }
}
