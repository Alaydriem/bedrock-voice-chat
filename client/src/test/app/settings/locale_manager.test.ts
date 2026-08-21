import { beforeEach, expect, test, vi } from "vitest";
import { get } from "svelte/store";
import LocaleManager from "../../../js/app/managers/settings/LocaleManager";
import { I18n } from "../../../lib/i18n";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
vi.mock("@charlesportwoodii/tauri-plugin-curia", () => ({ error: vi.fn(async () => {}) }));

const RUSSIAN = {
  v: 1,
  locale: "ru",
  plural: ["one", "few", "many"],
  m: { "Sign In Again": "Войти снова" },
};

function fakeStore() {
  const values = new Map<string, unknown>();
  return {
    get: async (key: string) => values.get(key) ?? null,
    set: async (key: string, value: unknown) => void values.set(key, value),
    save: vi.fn(async () => {}),
  };
}

beforeEach(() => {
  I18n.adopt(null);
  invoke.mockReset();
  invoke.mockImplementation(async (command: string) => {
    if (command === "i18n_locales") return ["de", "ru"];
    if (command === "i18n_load") return RUSSIAN;
    return null;
  });
});

test("initialize publishes the locales the backend reports", async () => {
  const manager = new LocaleManager(fakeStore() as never);
  await manager.initialize();

  expect(get(manager.locales)).toEqual(["de", "ru"]);
});

test("with no stored preference the system locale is requested", async () => {
  const manager = new LocaleManager(fakeStore() as never);
  await manager.initialize();

  expect(invoke).toHaveBeenCalledWith("i18n_load", { requested: navigator.language });
});

test("a stored preference is requested instead of the system locale", async () => {
  const store = fakeStore();
  await store.set("locale", "de");

  const manager = new LocaleManager(store as never);
  await manager.initialize();

  expect(invoke).toHaveBeenCalledWith("i18n_load", { requested: "de" });
});

test("a loaded pack reaches the translation surface", async () => {
  const manager = new LocaleManager(fakeStore() as never);
  await manager.initialize();

  expect(I18n.t("Sign In Again")).toBe("Войти снова");
});

test("choosing a locale persists it with an explicit save", async () => {
  const store = fakeStore();
  const manager = new LocaleManager(store as never);
  await manager.initialize();
  await manager.choose("de");

  expect(await store.get("locale")).toBe("de");
  expect(store.save).toHaveBeenCalled();
});

test("choosing auto clears the override and re-detects", async () => {
  const store = fakeStore();
  const manager = new LocaleManager(store as never);
  await manager.initialize();
  await manager.choose("auto");

  expect(invoke).toHaveBeenLastCalledWith("i18n_load", { requested: navigator.language });
  expect(get(manager.active)).toBe("auto");
});

test("a backend failure leaves the app in English rather than throwing", async () => {
  invoke.mockImplementation(async (command: string) => {
    if (command === "i18n_locales") return [];
    throw new Error("resource directory missing");
  });

  const manager = new LocaleManager(fakeStore() as never);

  await expect(manager.initialize()).resolves.toBeUndefined();
  expect(I18n.t("Sign In Again")).toBe("Sign In Again");
});
