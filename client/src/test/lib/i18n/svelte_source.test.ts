import { expect, test } from "vitest";
import SvelteSource from "../../../lib/i18n/SvelteSource.ts";

function lineOf(output: string, needle: string): number {
  const index = output.split("\n").findIndex((line) => line.includes(needle));
  return index + 1;
}

test("script contents keep their original line numbers", () => {
  const source = ['<script lang="ts">', '  const a = I18n.t("Alpha");', "</script>"].join("\n");
  expect(lineOf(SvelteSource.toTypeScript(source), "Alpha")).toBe(2);
});

test("markup expressions keep their original line numbers", () => {
  const source = ["<div>", "", '  <p>{I18n.t("Beta")}</p>', "</div>"].join("\n");
  expect(lineOf(SvelteSource.toTypeScript(source), "Beta")).toBe(3);
});

test("expressions inside attributes are recovered", () => {
  const source = '<button title={I18n.t("Gamma")}>x</button>';
  expect(SvelteSource.toTypeScript(source)).toContain("Gamma");
});

test("expressions inside each blocks are recovered", () => {
  const source = ["{#each rows as row}", '  <li>{I18n.t("Delta")}</li>', "{/each}"].join("\n");
  expect(SvelteSource.toTypeScript(source)).toContain("Delta");
});

test("both script blocks are recovered", () => {
  const source = [
    '<script module lang="ts">',
    '  const m = I18n.t("Module");',
    "</script>",
    '<script lang="ts">',
    '  const i = I18n.t("Instance");',
    "</script>",
  ].join("\n");
  const output = SvelteSource.toTypeScript(source);

  expect(output).toContain("Module");
  expect(output).toContain("Instance");
});

test("markup with no expressions yields no statements", () => {
  expect(SvelteSource.toTypeScript("<p>Plain</p>").trim()).toBe("");
});

test("the output parses as TypeScript rather than as fragments", () => {
  const source = ["<div>", '  <p>{I18n.t("Epsilon")}</p>', "</div>"].join("\n");
  expect(SvelteSource.toTypeScript(source)).toContain(';(I18n.t("Epsilon"));');
});
