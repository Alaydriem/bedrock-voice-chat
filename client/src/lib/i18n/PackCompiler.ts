import { CONTEXT_SEPARATOR } from "./Contract.ts";
import PluralForms from "./PluralForms.ts";

export interface PoMessage {
  msgid: string;
  msgid_plural?: string;
  msgstr: string[];
}

export interface ParsedPo {
  headers: Record<string, string>;
  translations: Record<string, Record<string, PoMessage>>;
}

export interface CompiledPack {
  v: 1;
  locale: string;
  plural: string[];
  m: Record<string, string | string[]>;
}

export default class PackCompiler {
  static compile(locale: string, po: ParsedPo): CompiledPack {
    const plural = PluralForms.categoriesFor(locale, PackCompiler.pluralHeader(po));
    const m: Record<string, string | string[]> = {};

    for (const [context, messages] of Object.entries(po.translations)) {
      for (const message of Object.values(messages)) {
        if (message.msgid === "") continue;
        if (!PackCompiler.#isTranslated(message)) continue;

        const key = PackCompiler.key(context, message.msgid);
        m[key] = message.msgid_plural === undefined ? message.msgstr[0] : [...message.msgstr];
      }
    }

    return { v: 1, locale, plural, m };
  }

  static coverage(po: ParsedPo): number {
    let total = 0;
    let translated = 0;

    for (const messages of Object.values(po.translations)) {
      for (const message of Object.values(messages)) {
        if (message.msgid === "") continue;
        total += 1;
        if (PackCompiler.#isTranslated(message)) translated += 1;
      }
    }

    return total === 0 ? 1 : translated / total;
  }

  static key(context: string, msgid: string): string {
    return context === "" ? msgid : `${context}${CONTEXT_SEPARATOR}${msgid}`;
  }

  static pluralHeader(po: ParsedPo): string {
    const header = po.headers["plural-forms"] ?? po.headers["Plural-Forms"];
    if (header === undefined) {
      throw new Error("Catalogue has no Plural-Forms header");
    }
    return header;
  }

  // A plural message counts only when every form is filled; a partially translated plural
  // renders a blank string for whichever count hits the empty form.
  static #isTranslated(message: PoMessage): boolean {
    return message.msgstr.length > 0 && message.msgstr.every((form) => form !== "");
  }
}
