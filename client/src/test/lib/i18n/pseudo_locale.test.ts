import { expect, test } from "vitest";
import PseudoLocale from "../../../lib/i18n/PseudoLocale.ts";
import type { ParsedPo } from "../../../lib/i18n/PackCompiler.ts";

function pot(): ParsedPo {
  return {
    headers: { "plural-forms": "nplurals=2; plural=(n != 1);" },
    translations: {
      "": {
        "Sign In": { msgid: "Sign In", msgstr: [""] },
        "{count} player": {
          msgid: "{count} player",
          msgid_plural: "{count} players",
          msgstr: ["", ""],
        },
      },
    },
  };
}

test("every message is bracketed so an unextracted string is visible", () => {
  const pack = PseudoLocale.build(pot());
  expect(pack.m["Sign In"]).toMatch(/^\[.*\]$/);
});

test("the text is expanded so tight layouts fail here rather than in German", () => {
  const pack = PseudoLocale.build(pot());
  expect((pack.m["Sign In"] as string).length).toBeGreaterThan("Sign In".length * 1.3);
});

test("placeholders survive untouched so substitution still works", () => {
  const pack = PseudoLocale.build(pot());
  const forms = pack.m["{count} player"] as string[];

  expect(forms).toHaveLength(2);
  for (const form of forms) {
    expect(form).toContain("{count}");
  }
});

test("the accenting reaches the words around a placeholder", () => {
  const pack = PseudoLocale.build(pot());
  const [singular] = pack.m["{count} player"] as string[];

  expect(singular).not.toContain("player");
  expect(singular).toContain("{count}");
});

test("plural messages get one pseudo form per category", () => {
  const pack = PseudoLocale.build(pot());
  expect(Array.isArray(pack.m["{count} player"])).toBe(true);
  expect((pack.m["{count} player"] as string[]).length).toBe(pack.plural.length);
});

test("the locale is en_XA so it never collides with a real one", () => {
  expect(PseudoLocale.build(pot()).locale).toBe("en_XA");
});
