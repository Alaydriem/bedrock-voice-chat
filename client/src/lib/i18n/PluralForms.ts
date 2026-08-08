import PluralExpression from "./PluralExpression.ts";

/**
 * Resolves a gettext `Plural-Forms` header into the CLDR plural categories a runtime will
 * ask for, ordered by the msgstr index each selects.
 *
 * Gettext orders forms by an expression; CLDR names them. The two are reconciled by
 * evaluating the expression over integers and asking `Intl.PluralRules` what each integer
 * is called. Categories a locale uses only for fractions never appear, which is correct:
 * a gettext catalogue has no form for them.
 */
export default class PluralForms {
  // Far enough to reach every residue class real Plural-Forms expressions test, which
  // bottom out at n%100.
  static readonly #PROBE_LIMIT = 200;

  static categoriesFor(locale: string, header: string): string[] {
    const { count, expression } = PluralForms.#parseHeader(header);
    const rules = new Intl.PluralRules(PluralForms.toBcp47(locale));
    const categories: (string | undefined)[] = new Array(count).fill(undefined);

    for (let n = 0; n <= PluralForms.#PROBE_LIMIT; n += 1) {
      const index = PluralExpression.evaluate(expression, n);

      if (!Number.isInteger(index) || index < 0 || index >= count) {
        throw new Error(
          `Plural expression for ${locale} selected form ${index}, outside 0..${count - 1}`,
        );
      }

      const category = rules.select(n);
      const known = categories[index];

      if (known === undefined) {
        categories[index] = category;
      } else if (known !== category) {
        throw new Error(
          `Plural form ${index} for ${locale} maps to both "${known}" and "${category}"`,
        );
      }
    }

    const unreachable = categories.indexOf(undefined);
    if (unreachable !== -1) {
      throw new Error(`Plural form ${unreachable} for ${locale} is never selected`);
    }

    return categories as string[];
  }

  static toBcp47(locale: string): string {
    return locale.replace("_", "-");
  }

  static #parseHeader(header: string): { count: number; expression: string } {
    const count = /nplurals\s*=\s*(\d+)/.exec(header);
    const expression = /plural\s*=\s*(.+?)\s*;?\s*$/s.exec(header);

    if (count === null || expression === null) {
      throw new Error(`Unparseable Plural-Forms header: ${header}`);
    }

    return { count: Number(count[1]), expression: expression[1] };
  }
}
