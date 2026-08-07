import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MeterProbe } from "../../radial/core/canvas/MeterProbe";

/**
 * The measurement half of the meter's reliability story.
 *
 * The pill has now failed in two different layers on the same phone: a listener the page
 * silently skipped, and a renderer that drew nothing while levels demonstrably arrived. The
 * probe exists so the next broken launch names its own layer — a binding records what it was
 * handed and what it actually drew, and the gap between the two is the verdict.
 */
describe("MeterProbe", () => {
    beforeEach(() => {
        vi.useFakeTimers();
        MeterProbe.reset("self");
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it("reads as unmounted before any binding registers", () => {
        expect(MeterProbe.read("self").mounted).toBe(false);
    });

    it("counts only audible levels, not the silence between them", () => {
        MeterProbe.register("self");
        MeterProbe.level("self", 0);
        MeterProbe.level("self", 0.5);
        MeterProbe.level("self", 0);

        const snap = MeterProbe.read("self");
        expect(snap.levels).toBe(1);
        expect(snap.lastLevel).toBe(0.5);
    });

    it("ages its readings so a stall is distinguishable from activity", () => {
        MeterProbe.register("self");
        MeterProbe.level("self", 0.5);
        MeterProbe.painted("self");

        vi.advanceTimersByTime(3000);
        MeterProbe.level("self", 0.7);

        const snap = MeterProbe.read("self");
        expect(snap.levelAgeMs).toBeLessThan(100);
        expect(snap.paintAgeMs).toBeGreaterThanOrEqual(3000);
        expect(snap.paints).toBe(1);
    });

    it("keeps counts across re-registration, because a remounted pill is the same meter", () => {
        MeterProbe.register("self");
        MeterProbe.level("self", 0.5);
        MeterProbe.register("self");

        expect(MeterProbe.read("self").levels).toBe(1);
    });
});
