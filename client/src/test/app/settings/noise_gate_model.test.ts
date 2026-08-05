import { describe, expect, it } from "vitest";
import { NoiseGateModel } from "../../../js/app/settings/NoiseGateModel";

describe("NoiseGateModel.apply", () => {
    // A close threshold above the open one is a gate that never shuts. Nothing in the audio
    // path refuses it, so it presents as the feature not working.
    it("pushes the close threshold down when the open threshold comes to meet it", () => {
        const next = NoiseGateModel.apply(NoiseGateModel.DEFAULTS, "open_threshold", -55);
        expect(next.open_threshold).toBe(-55);
        expect(next.close_threshold).toBeLessThanOrEqual(-55 - NoiseGateModel.HYSTERESIS);
    });

    // Whichever one the reader is holding wins; the other yields.
    it("pushes the open threshold up when the close threshold comes to meet it", () => {
        const next = NoiseGateModel.apply(NoiseGateModel.DEFAULTS, "close_threshold", -20);
        expect(next.close_threshold).toBe(-20);
        expect(next.open_threshold).toBeGreaterThanOrEqual(-20 + NoiseGateModel.HYSTERESIS);
    });

    it("leaves a legal pair alone", () => {
        const next = NoiseGateModel.apply(NoiseGateModel.DEFAULTS, "open_threshold", -30);
        expect(next.close_threshold).toBe(NoiseGateModel.DEFAULTS.close_threshold);
    });

    it("does not let a threshold leave its own range", () => {
        expect(NoiseGateModel.apply(NoiseGateModel.DEFAULTS, "open_threshold", 40).open_threshold).toBe(0);
        expect(
            NoiseGateModel.apply(NoiseGateModel.DEFAULTS, "attack_rate", 9_000).attack_rate,
        ).toBe(250);
    });

    // Clamping the pair must not push the other one out of its own range either.
    it("keeps the yielding threshold inside its range", () => {
        const next = NoiseGateModel.apply(NoiseGateModel.DEFAULTS, "close_threshold", 0);
        expect(next.open_threshold).toBeLessThanOrEqual(0);
    });

    it("changes only the timings it was asked to change", () => {
        const next = NoiseGateModel.apply(NoiseGateModel.DEFAULTS, "hold_time", 120);
        expect(next.hold_time).toBe(120);
        expect(next.attack_rate).toBe(NoiseGateModel.DEFAULTS.attack_rate);
        expect(next.release_rate).toBe(NoiseGateModel.DEFAULTS.release_rate);
    });

    // The settings object is persisted and shared, so a change returns a new one.
    it("does not mutate what it was given", () => {
        const before = { ...NoiseGateModel.DEFAULTS };
        NoiseGateModel.apply(before, "open_threshold", -70);
        expect(before).toEqual(NoiseGateModel.DEFAULTS);
    });
});

describe("NoiseGateModel.hydrate", () => {
    // A config written by an older build may be missing a field, and a missing number would
    // render an empty slider rather than a default.
    it("fills in whatever a stored config is missing", () => {
        expect(NoiseGateModel.hydrate({ open_threshold: -30 })).toEqual({
            ...NoiseGateModel.DEFAULTS,
            open_threshold: -30,
        });
    });

    it("falls back entirely when there is nothing stored", () => {
        expect(NoiseGateModel.hydrate(null)).toEqual(NoiseGateModel.DEFAULTS);
    });

    it("pulls a stored value that is out of range back into it", () => {
        expect(NoiseGateModel.hydrate({ hold_time: 4_000 }).hold_time).toBe(250);
    });
});

describe("NoiseGateModel.format", () => {
    it("names the unit, because a bare number here means nothing", () => {
        const open = NoiseGateModel.KNOBS.find((k) => k.id === "open_threshold");
        expect(NoiseGateModel.format(open!, -40)).toBe("-40 dBFS");
        const hold = NoiseGateModel.KNOBS.find((k) => k.id === "hold_time");
        expect(NoiseGateModel.format(hold!, 50)).toBe("50 ms");
    });
});
