import { beforeEach, expect, test } from "vitest";
import DiagnosticsCopy from "../../../js/app/dashboard/DiagnosticsCopy";
import { I18n } from "../../../lib/i18n";
import type { Verdict } from "../../../radial/core/controllers/Diagnostics";
import type { LanguagePack } from "../../../js/bindings/LanguagePack";

/**
 * Russian needs three forms for an integer count where English needs two. The kit used to
 * choose between "player is" and "players are" itself, which is unrepresentable here.
 */
const RUSSIAN = {
    v: 1,
    locale: "ru",
    plural: ["one", "few", "many"],
    m: {
        "Everything looks fine.": "Всё в порядке.",
        "{count} player is muted by you.": [
            "{count} игрок заглушён вами.",
            "{count} игрока заглушены вами.",
            "{count} игроков заглушены вами.",
        ],
        "Packet loss is {percent}%. Audio will break up.": "Потери пакетов {percent}%.",
    },
} as unknown as LanguagePack;

function mutedOthers(count: number): Verdict {
    return { severity: "warn", code: "muted-others", params: { count } };
}

beforeEach(() => {
    I18n.adopt(null);
});

test("with no pack the verdict reads in English", () => {
    expect(DiagnosticsCopy.of({ severity: "ok", code: "fine" })).toBe("Everything looks fine.");
});

test("a loaded pack translates the verdict", () => {
    I18n.adopt(RUSSIAN);

    expect(DiagnosticsCopy.of({ severity: "ok", code: "fine" })).toBe("Всё в порядке.");
});

test("English uses its two plural forms", () => {
    expect(DiagnosticsCopy.of(mutedOthers(1))).toBe("1 player is muted by you.");
    expect(DiagnosticsCopy.of(mutedOthers(4))).toBe("4 players are muted by you.");
});

// The regression this change exists to fix. Under the old code the kit picked between two
// English forms, so 2 and 5 could never read differently in Russian.
test("Russian selects among three forms, which the old two-way choice could not express", () => {
    I18n.adopt(RUSSIAN);

    expect(DiagnosticsCopy.of(mutedOthers(1))).toBe("1 игрок заглушён вами.");
    expect(DiagnosticsCopy.of(mutedOthers(2))).toBe("2 игрока заглушены вами.");
    expect(DiagnosticsCopy.of(mutedOthers(5))).toBe("5 игроков заглушены вами.");
    expect(DiagnosticsCopy.of(mutedOthers(21))).toBe("21 игрок заглушён вами.");
});

test("numbers are interpolated into a translated verdict", () => {
    I18n.adopt(RUSSIAN);

    expect(DiagnosticsCopy.of({ severity: "warn", code: "loss", params: { percent: 7 } })).toBe(
        "Потери пакетов 7%.",
    );
});

test("reconnecting reads differently once there is an attempt to name", () => {
    expect(DiagnosticsCopy.of({ severity: "bad", code: "reconnecting", params: { attempt: 0 } }))
        .toBe("Reconnecting. Nobody can hear you right now.");
    expect(DiagnosticsCopy.of({ severity: "bad", code: "reconnecting", params: { attempt: 3 } }))
        .toBe("Reconnecting — attempt 3. Nobody can hear you right now.");
});

// Every code the kit can emit, with the parameters it emits alongside. A code whose copy
// names a placeholder its verdict never supplies would render the brace to the user, so
// this pairs each one with what `Diagnostics.verdict` actually sends.
const EVERY_VERDICT: readonly Verdict[] = [
    { severity: "bad", code: "reconnecting", params: { attempt: 0 } },
    { severity: "bad", code: "reconnecting", params: { attempt: 2 } },
    { severity: "bad", code: "stalled" },
    { severity: "warn", code: "deafened" },
    { severity: "warn", code: "ptt-idle" },
    { severity: "bad", code: "muted" },
    { severity: "warn", code: "input-rate", params: { kHz: "44.1" } },
    { severity: "warn", code: "concealment", params: { percent: 12 } },
    { severity: "warn", code: "loss", params: { percent: 7 } },
    { severity: "warn", code: "muted-others", params: { count: 3 } },
    { severity: "ok", code: "fine" },
];

test("every verdict renders with no placeholder left showing", () => {
    for (const verdict of EVERY_VERDICT) {
        const text = DiagnosticsCopy.of(verdict);

        expect(text, verdict.code).toBeTruthy();
        expect(text, verdict.code).not.toContain("{");
    }
});
