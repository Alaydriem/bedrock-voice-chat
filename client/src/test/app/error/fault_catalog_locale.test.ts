import { beforeEach, expect, test } from "vitest";
import FaultCatalog from "../../../js/app/error/FaultCatalog";
import { I18n, CONTEXT_SEPARATOR } from "../../../lib/i18n";
import type { LanguagePack } from "../../../js/bindings/LanguagePack";

const RUSSIAN = {
    v: 1,
    locale: "ru",
    plural: ["one", "few", "many"],
    m: {
        "Connection Refused": "Соединение отклонено",
        "Sign In Again": "Войти снова",
        "Downloading the update…": "Загрузка обновления…",
        [`client${CONTEXT_SEPARATOR}TOO OLD`]: "УСТАРЕЛ",
        [`server${CONTEXT_SEPARATOR}TOO OLD`]: "УСТАРЕЛА",
        "A new version (v{version}) of Bedrock Voice Chat is available. This will download and install the update immediately.":
            "Доступна новая версия (v{version}).",
    },
} as unknown as LanguagePack;

beforeEach(() => {
    I18n.adopt(null);
});

test("with no pack the catalog reads in English", () => {
    expect(FaultCatalog.resolve("AUTH01").title).toBe("Connection Refused");
});

/**
 * The regression this whole migration exists to prevent.
 *
 * `DEFINITIONS` used to be an object literal on the class body, evaluated once at import —
 * before any pack loads. Every string was therefore captured in English permanently, and
 * nothing failed: the screen simply never changed language.
 */
test("a pack adopted after the module loaded still reaches the catalog", () => {
    I18n.adopt(RUSSIAN);

    expect(FaultCatalog.resolve("AUTH01").title).toBe("Соединение отклонено");
});

test("dropping the pack returns the catalog to English", () => {
    I18n.adopt(RUSSIAN);
    I18n.adopt(null);

    expect(FaultCatalog.resolve("AUTH01").title).toBe("Connection Refused");
});

test("action labels are translated too, not just the headline", () => {
    I18n.adopt(RUSSIAN);

    expect(FaultCatalog.resolve("AUTH01").primaryAction.label).toBe("Войти снова");
});

test("the update phrases follow the language", () => {
    I18n.adopt(RUSSIAN);

    expect(FaultCatalog.UPDATE_PHRASES[0]).toBe("Загрузка обновления…");
});

// Same two English words on both version faults. Separated by context so a language that
// inflects the adjective to agree with client or server can say each differently.
test("the two version captions are separately translatable", () => {
    I18n.adopt(RUSSIAN);

    expect(FaultCatalog.resolve("VER01").caption).toBe("УСТАРЕЛ");
    expect(FaultCatalog.resolve("VER02").caption).toBe("УСТАРЕЛА");
});

test("the version is interpolated into the translated update message", () => {
    I18n.adopt(RUSSIAN);
    const updated = FaultCatalog.withVersion(FaultCatalog.resolve("UPD01"), "1.2.3");

    expect(updated.message).toBe("Доступна новая версия (v1.2.3).");
    expect(updated.caption).toBe("v1.2.3");
});

test("an unrecognised code keeps its own code on the translated catch-all", () => {
    I18n.adopt(RUSSIAN);
    const resolved = FaultCatalog.resolve("WAT99");

    expect(resolved.code).toBe("WAT99");
    expect(resolved.title).toBe("Something Went Wrong");
});

test("removing the server switch leaves a usable way out", () => {
    const trimmed = FaultCatalog.withoutServerSwitch(FaultCatalog.resolve("VER02"));

    expect(trimmed.primaryAction.url).toBe("/dashboard");
    expect(trimmed.secondaryAction).toBeUndefined();
});

// `forScreen` is what the error route derives from, so these cover the state flow the
// route no longer holds itself.
test("with several servers configured the switch offer is kept", () => {
    const shown = FaultCatalog.forScreen("VER02", null, false);

    expect(shown.primaryAction.url).toBe("/");
});

test("with one server configured the switch offer is removed", () => {
    const shown = FaultCatalog.forScreen("VER02", null, true);

    expect(shown.primaryAction.url).toBe("/dashboard");
});

test("the update keeps both actions even with one server, because neither switches", () => {
    const shown = FaultCatalog.forScreen("UPD01", null, true);

    expect(shown.secondaryAction?.url).toBe("/");
});

test("a version reaches the screen's copy and caption", () => {
    const shown = FaultCatalog.forScreen("UPD01", "9.9.9", false);

    expect(shown.message).toContain("v9.9.9");
    expect(shown.caption).toBe("v9.9.9");
});

test("no code resolves to the catch-all rather than throwing", () => {
    expect(FaultCatalog.forScreen(null, null, false).code).toBe("ERROR");
});

test("the screen follows a language change, because nothing is stored", () => {
    expect(FaultCatalog.forScreen("AUTH01", null, false).title).toBe("Connection Refused");

    I18n.adopt(RUSSIAN);

    expect(FaultCatalog.forScreen("AUTH01", null, false).title).toBe("Соединение отклонено");
});

test("every definition still carries the fields the screen renders", () => {
    I18n.adopt(RUSSIAN);

    for (const [code, definition] of Object.entries(FaultCatalog.DEFINITIONS)) {
        expect(definition.title, code).toBeTruthy();
        expect(definition.message, code).toBeTruthy();
        expect(definition.caption, code).toBeTruthy();
        expect(definition.label, code).toBeTruthy();
        expect(definition.hint, code).toBeTruthy();
        expect(definition.primaryAction.label, code).toBeTruthy();
    }
});
