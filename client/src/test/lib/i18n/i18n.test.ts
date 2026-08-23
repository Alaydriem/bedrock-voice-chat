import { beforeEach, expect, test } from "vitest";
import { I18n, CONTEXT_SEPARATOR } from "../../../lib/i18n";
import type { LanguagePack } from "../../../js/bindings/LanguagePack";

const RUSSIAN = {
  v: 1,
  locale: "ru",
  plural: ["one", "few", "many"],
  m: {
    "Sign In Again": "Войти снова",
    [`audio${CONTEXT_SEPARATOR}Output`]: "Выход",
    "{count} player nearby": ["{count} игрок", "{count} игрока", "{count} игроков"],
    "Signed in as {name}": "Вы вошли как {name}",
  },
} as unknown as LanguagePack;

beforeEach(() => {
  I18n.adopt(null);
});

test("with no pack a message id is its own English translation", () => {
  expect(I18n.t("Sign In Again")).toBe("Sign In Again");
});

test("a loaded pack translates a known message", () => {
  I18n.adopt(RUSSIAN);
  expect(I18n.t("Sign In Again")).toBe("Войти снова");
});

test("an untranslated message falls through to English rather than blank", () => {
  I18n.adopt(RUSSIAN);
  expect(I18n.t("Never Translated")).toBe("Never Translated");
});

test("context lookups use the separator the compiler emits", () => {
  I18n.adopt(RUSSIAN);
  expect(I18n.tc("audio", "Output")).toBe("Выход");
});

test("a context lookup with no entry falls through to the bare message", () => {
  I18n.adopt(RUSSIAN);
  expect(I18n.tc("logs", "Output")).toBe("Output");
});

test("plurals select the Russian form for each count", () => {
  I18n.adopt(RUSSIAN);
  expect(I18n.tn("{count} player nearby", "{count} players nearby", 1)).toBe("{count} игрок");
  expect(I18n.tn("{count} player nearby", "{count} players nearby", 3)).toBe("{count} игрока");
  expect(I18n.tn("{count} player nearby", "{count} players nearby", 7)).toBe("{count} игроков");
});

test("a CLDR category the catalog has no form for falls to the last form", () => {
  I18n.adopt(RUSSIAN);
  expect(I18n.tn("{count} player nearby", "{count} players nearby", 1.5)).toBe(
    "{count} игроков",
  );
});

test("with no pack plurals use English rules against the two English forms", () => {
  expect(I18n.tn("{count} player nearby", "{count} players nearby", 1)).toBe(
    "{count} player nearby",
  );
  expect(I18n.tn("{count} player nearby", "{count} players nearby", 4)).toBe(
    "{count} players nearby",
  );
});

test("named placeholders are substituted", () => {
  I18n.adopt(RUSSIAN);
  expect(I18n.tf("Signed in as {name}", { name: "Steve" })).toBe("Вы вошли как Steve");
});

test("a placeholder with no matching parameter is left intact rather than blanked", () => {
  expect(I18n.tf("Signed in as {name}", {})).toBe("Signed in as {name}");
});

test("substituted values are never treated as markup", () => {
  expect(I18n.tf("Signed in as {name}", { name: "<b>x</b>" })).toBe("Signed in as <b>x</b>");
});

test("adopting null returns the app to English", () => {
  I18n.adopt(RUSSIAN);
  I18n.adopt(null);

  expect(I18n.t("Sign In Again")).toBe("Sign In Again");
  expect(I18n.locale).toBe("en");
});
