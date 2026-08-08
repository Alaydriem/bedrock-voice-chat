import { expect, test } from "vitest";
import PluralForms from "../../../lib/i18n/PluralForms";

const HEADERS: Record<string, string> = {
  en: "nplurals=2; plural=(n != 1);",
  de: "nplurals=2; plural=(n != 1);",
  ja: "nplurals=1; plural=0;",
  ru: "nplurals=3; plural=(n%10==1 && n%100!=11 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2);",
  pl: "nplurals=3; plural=(n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2);",
  fr: "nplurals=2; plural=(n > 1);",
};

test("English maps the two gettext forms onto CLDR one/other", () => {
  expect(PluralForms.categoriesFor("en", HEADERS.en)).toEqual(["one", "other"]);
});

test("Japanese has a single form and it is CLDR other", () => {
  expect(PluralForms.categoriesFor("ja", HEADERS.ja)).toEqual(["other"]);
});

test("Russian resolves three gettext forms even though CLDR names four categories", () => {
  expect(PluralForms.categoriesFor("ru", HEADERS.ru)).toEqual(["one", "few", "many"]);
});

test("Polish resolves in gettext index order, not CLDR declaration order", () => {
  expect(PluralForms.categoriesFor("pl", HEADERS.pl)).toEqual(["one", "few", "many"]);
});

test("French treats one as singular despite its zero-is-singular rule", () => {
  expect(PluralForms.categoriesFor("fr", HEADERS.fr)).toEqual(["one", "other"]);
});

test("underscored catalogue locales become BCP-47 for Intl", () => {
  expect(PluralForms.toBcp47("pt_BR")).toBe("pt-BR");
  expect(PluralForms.toBcp47("zh_CN")).toBe("zh-CN");
  expect(PluralForms.toBcp47("ru")).toBe("ru");
});

test("an expression disagreeing with the locale's real rules is rejected", () => {
  const wrong = "nplurals=3; plural=(n%3);";
  expect(() => PluralForms.categoriesFor("en", wrong)).toThrow(/maps to both/);
});

test("a header declaring more forms than the expression produces is rejected", () => {
  const unreachable = "nplurals=4; plural=(n != 1);";
  expect(() => PluralForms.categoriesFor("en", unreachable)).toThrow(/never selected/);
});

test("an unparseable header is rejected rather than guessed at", () => {
  expect(() => PluralForms.categoriesFor("en", "garbage")).toThrow(/Plural-Forms/);
});
