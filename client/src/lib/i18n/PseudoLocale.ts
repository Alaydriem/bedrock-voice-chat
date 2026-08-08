import PackCompiler, { type CompiledPack, type ParsedPo } from "./PackCompiler.ts";

/**
 * Builds `en_XA` from the template catalog.
 *
 * Localization fails silently: an unextracted string renders as flawless English and
 * cannot be found by reading the screen. Bracketing every extracted string makes the
 * unextracted ones the only plain text left, and padding to roughly German length makes a
 * layout that cannot survive translation fail before a translator has spent time on it.
 */
export default class PseudoLocale {
  static readonly LOCALE = "en_XA";

  static readonly #ACCENTS: Record<string, string> = {
    a: "å", b: "ƀ", c: "ç", d: "ð", e: "é", f: "ƒ", g: "ĝ", h: "ĥ", i: "î", j: "ĵ",
    k: "ķ", l: "ļ", m: "ɱ", n: "ñ", o: "ô", p: "þ", q: "ǫ", r: "ŕ", s: "š", t: "ţ",
    u: "û", v: "ṽ", w: "ŵ", x: "ẋ", y: "ý", z: "ž",
    A: "Å", B: "Ɓ", C: "Ç", D: "Ð", E: "É", F: "Ƒ", G: "Ĝ", H: "Ĥ", I: "Ĩ", J: "Ĵ",
    K: "Ķ", L: "Ļ", M: "Ṁ", N: "Ñ", O: "Ô", P: "Þ", Q: "Ǫ", R: "Ŕ", S: "Ṡ", T: "Ţ",
    U: "Û", V: "Ṽ", W: "Ŵ", X: "Ẋ", Y: "Ý", Z: "Ž",
  };

  static readonly #EXPANSION = 0.4;

  static build(pot: ParsedPo): CompiledPack {
    const plural = ["one", "other"];
    const m: Record<string, string | string[]> = {};

    for (const [context, messages] of Object.entries(pot.translations)) {
      for (const message of Object.values(messages)) {
        if (message.msgid === "") continue;

        const key = PackCompiler.key(context, message.msgid);
        m[key] =
          message.msgid_plural === undefined
            ? PseudoLocale.#transform(message.msgid)
            : [
                PseudoLocale.#transform(message.msgid),
                PseudoLocale.#transform(message.msgid_plural),
              ];
      }
    }

    return { v: 1, locale: PseudoLocale.LOCALE, plural, m };
  }

  // Placeholders pass through untouched: accenting `{count}` would break substitution and
  // hide the very layout problem this locale exists to expose.
  static #transform(source: string): string {
    const accented = source
      .split(/(\{[^}]*\})/)
      .map((part) => (part.startsWith("{") ? part : PseudoLocale.#accent(part)))
      .join("");

    const padding = "~".repeat(Math.ceil(source.length * PseudoLocale.#EXPANSION));
    return `[${accented} ${padding}]`;
  }

  static #accent(text: string): string {
    return [...text].map((char) => PseudoLocale.#ACCENTS[char] ?? char).join("");
  }
}
