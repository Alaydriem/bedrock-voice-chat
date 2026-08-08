import { describe, expect, test } from "vitest";
import PluralExpression from "../../../lib/i18n/PluralExpression";

const ENGLISH = "n != 1";
const RUSSIAN =
  "(n%10==1 && n%100!=11 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2)";
const POLISH = "(n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2)";
const JAPANESE = "0";

describe("English", () => {
  test("selects form 0 for exactly one", () => {
    expect(PluralExpression.evaluate(ENGLISH, 1)).toBe(0);
  });

  test("selects form 1 for zero and for many", () => {
    expect(PluralExpression.evaluate(ENGLISH, 0)).toBe(1);
    expect(PluralExpression.evaluate(ENGLISH, 2)).toBe(1);
  });
});

describe("Russian", () => {
  test.each([
    [1, 0],
    [21, 0],
    [2, 1],
    [22, 1],
    [5, 2],
    [11, 2],
    [0, 2],
  ])("n=%i selects form %i", (n, expected) => {
    expect(PluralExpression.evaluate(RUSSIAN, n)).toBe(expected);
  });
});

describe("Polish", () => {
  test.each([
    [1, 0],
    [2, 1],
    [22, 1],
    [5, 2],
    [0, 2],
  ])("n=%i selects form %i", (n, expected) => {
    expect(PluralExpression.evaluate(POLISH, n)).toBe(expected);
  });
});

test("a constant expression always selects the only form", () => {
  expect(PluralExpression.evaluate(JAPANESE, 7)).toBe(0);
});

test("a trailing semicolon from the header is tolerated", () => {
  expect(PluralExpression.evaluate("n != 1;", 1)).toBe(0);
});

describe("rejects anything outside the grammar", () => {
  test.each([
    ["process.exit(1)"],
    ["require('fs')"],
    ["n + 1"],
    ["globalThis"],
    ["n ? 0 : (1"],
  ])("throws on %s", (source) => {
    expect(() => PluralExpression.evaluate(source, 1)).toThrow();
  });
});
