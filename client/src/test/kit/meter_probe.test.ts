import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MeterProbe } from "../../radial/core/canvas/MeterProbe";
import { PushLevelSource } from "../../radial/core/sources/LevelSource";

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

/**
 * How many canvases the ledger is speaking for.
 *
 * The counts are merged by name, and the dashboard mounts the pill twice — a capsule in the
 * stage for a phone and a floating one for desktop — with CSS deciding which is seen. "It is
 * painting" is then a claim about whichever of them painted, not about the one being looked
 * at, and nothing in the readout said so.
 */
describe("the bindings a name is shared by", () => {
    // The ledger is static and deliberately survives a remount, so a case that does not clear
    // it first counts the registrations every case before it made.
    beforeEach(() => {
        MeterProbe.reset("self");
    });

    it("counts one for a single binding", () => {
        MeterProbe.register("self");
        expect(MeterProbe.read("self").bindings).toBe(1);
    });

    it("counts every binding that holds the name at once", () => {
        MeterProbe.register("self");
        MeterProbe.register("self");
        expect(MeterProbe.read("self").bindings).toBe(2);
    });

    // A binding that went away must stop being counted, or a screen that remounts a few times
    // reads as a screen with a dozen meters on it.
    it("stops counting a binding that was released", () => {
        MeterProbe.register("self");
        MeterProbe.register("self");
        MeterProbe.release("self");

        expect(MeterProbe.read("self").bindings).toBe(1);
    });

    // Releasing the last one ends the ledger's claim to be describing anything on screen,
    // but the history it collected is the evidence — zeroing it would erase the stall.
    it("keeps its counts after the last binding goes", () => {
        MeterProbe.register("self");
        MeterProbe.level("self", 0.5);
        MeterProbe.release("self");

        const snap = MeterProbe.read("self");
        expect(snap.bindings).toBe(0);
        expect(snap.levels).toBe(1);
    });

    it("never counts below zero, however the releases fall", () => {
        MeterProbe.register("self");
        MeterProbe.release("self");
        MeterProbe.release("self");

        expect(MeterProbe.read("self").bindings).toBe(0);
    });
});

/**
 * A push source counts what is listening to it.
 *
 * The pill is handed `PlayerLevelSources.own()` at mount and keeps that object for the life of
 * the binding. Nothing in the window can see whether the object it kept is still the one being
 * pushed to — a meter bound to an orphaned source and a meter nobody is speaking into look
 * identical from every counter there was.
 */
describe("PushLevelSource listener counting", () => {
    it("counts nothing before anything subscribes", () => {
        expect(new PushLevelSource().listeners).toBe(0);
    });

    it("counts each subscriber, and forgets one that unsubscribed", () => {
        const source = new PushLevelSource();
        const off = source.subscribe(() => {});
        source.subscribe(() => {});
        expect(source.listeners).toBe(2);

        off();
        expect(source.listeners).toBe(1);
    });

    it("counts nothing after close drops them all", () => {
        const source = new PushLevelSource();
        source.subscribe(() => {});
        source.close();
        expect(source.listeners).toBe(0);
    });
});
