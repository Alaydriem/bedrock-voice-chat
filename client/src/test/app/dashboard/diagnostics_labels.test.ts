import { beforeEach, expect, test } from "vitest";
import DiagnosticsCopy from "../../../js/app/dashboard/DiagnosticsCopy";
import { DIAGNOSTICS_EN, Diagnostics } from "../../../radial/core/controllers/Diagnostics";
import { I18n } from "../../../lib/i18n";
import type { LanguagePack } from "../../../js/bindings/LanguagePack";

const RUSSIAN = {
    v: 1,
    locale: "ru",
    plural: ["one", "few", "many"],
    m: { "Your mic": "Ваш микрофон", "Round trip": "Задержка", none: "нет" },
} as unknown as LanguagePack;

beforeEach(() => {
    I18n.adopt(null);
});

/**
 * The kit holds a default English set and the app holds a translated one. Two copies of the
 * same words drift, and the drift is silent — the panel keeps rendering, in English, for
 * whichever label the app forgot.
 */
test("the app supplies exactly the labels the kit declares", () => {
    expect(Object.keys(DiagnosticsCopy.labels()).sort()).toEqual(Object.keys(DIAGNOSTICS_EN).sort());
});

test("untranslated, the app's labels read identically to the kit's defaults", () => {
    expect(DiagnosticsCopy.labels()).toEqual(DIAGNOSTICS_EN);
});

test("a loaded pack reaches the panel's labels", () => {
    I18n.adopt(RUSSIAN);
    const labels = DiagnosticsCopy.labels();

    expect(labels.yourMic).toBe("Ваш микрофон");
    expect(labels.roundTrip).toBe("Задержка");
});

test("an untranslated label falls back to English rather than blank", () => {
    I18n.adopt(RUSSIAN);

    expect(DiagnosticsCopy.labels().jitterBuffer).toBe("Jitter buffer");
});

test("the kit renders its groups with whatever labels it is given", () => {
    I18n.adopt(RUSSIAN);
    const groups = Diagnostics.groups(input(), DiagnosticsCopy.labels());

    expect(groups[0].title).toBe("Ваш микрофон");
    expect(groups.find((g) => g.rows.some(([label]) => label === "Задержка"))).toBeTruthy();
});

test("with no labels supplied the kit still renders English on its own", () => {
    expect(Diagnostics.groups(input())[0].title).toBe("Your mic");
});

function input() {
    return {
        rtt: 30, lossPercent: 0, jitterMs: 20, jitterDrops: 0, datagramsIn: 50,
        datagramsOut: 50, capturing: 50, inputDevice: "Mic", inputRate: 48000,
        outputDevice: "Speakers", outputRate: 48000, quicPort: 443, protocol: "3.0.0",
        rangeMetres: 48, falloff: "linear", server: "bvc.example.com", uptimeSeconds: 60,
        reconnecting: false, muted: false, noiseGate: "Open" as const, deafened: false,
        pttIdle: false, mutedOthers: 0, visiblePlayers: 4,
    };
}
