import { expect, test } from "vitest";
import PackCompiler, { type ParsedPo } from "../../../lib/i18n/PackCompiler.ts";
import { CONTEXT_SEPARATOR } from "../../../lib/i18n/Contract.ts";

const RUSSIAN_HEADER =
  "nplurals=3; plural=(n%10==1 && n%100!=11 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2);";

function russianPo(): ParsedPo {
  return {
    headers: { "plural-forms": RUSSIAN_HEADER },
    translations: {
      "": {
        "Sign In Again": { msgid: "Sign In Again", msgstr: ["Войти снова"] },
        "Never Translated": { msgid: "Never Translated", msgstr: [""] },
        "{count} player nearby": {
          msgid: "{count} player nearby",
          msgid_plural: "{count} players nearby",
          msgstr: ["игрок", "игрока", "игроков"],
        },
      },
      audio: {
        Output: { msgid: "Output", msgstr: ["Выход"] },
      },
    },
  };
}

test("plural categories are resolved onto the pack", () => {
  const pack = PackCompiler.compile("ru", russianPo());
  expect(pack.plural).toEqual(["one", "few", "many"]);
});

test("a singular message compiles to a bare string", () => {
  const pack = PackCompiler.compile("ru", russianPo());
  expect(pack.m["Sign In Again"]).toBe("Войти снова");
});

test("a plural message compiles to the ordered form array", () => {
  const pack = PackCompiler.compile("ru", russianPo());
  expect(pack.m["{count} player nearby"]).toEqual(["игрок", "игрока", "игроков"]);
});

test("context is folded into the key with the EOT separator", () => {
  const pack = PackCompiler.compile("ru", russianPo());
  expect(pack.m[`audio${CONTEXT_SEPARATOR}Output`]).toBe("Выход");
});

test("an untranslated message is omitted so lookup falls through to English", () => {
  const pack = PackCompiler.compile("ru", russianPo());
  expect("Never Translated" in pack.m).toBe(false);
});

test("the empty header entry is never emitted as a message", () => {
  const po = russianPo();
  po.translations[""][""] = { msgid: "", msgstr: ["Project-Id-Version: bvc"] };
  const pack = PackCompiler.compile("ru", po);
  expect("" in pack.m).toBe(false);
});

test("coverage counts translated messages against the total", () => {
  expect(PackCompiler.coverage(russianPo())).toBeCloseTo(3 / 4);
});

test("a plural entry missing one of its forms does not count as translated", () => {
  const po = russianPo();
  po.translations[""]["{count} player nearby"].msgstr = ["игрок", "", "игроков"];
  expect(PackCompiler.coverage(po)).toBeCloseTo(2 / 4);
});
