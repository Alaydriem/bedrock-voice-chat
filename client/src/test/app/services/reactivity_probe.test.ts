import { flushSync, tick } from "svelte";
import { describe, expect, it } from "vitest";
import { ReactivityProbe } from "../../../js/app/services/ReactivityProbe.svelte";

describe("ReactivityProbe", () => {
    it("starts settled", () => {
        const probe = new ReactivityProbe();
        expect(probe.settled).toBe(true);
        probe.cleanup();
    });

    // The window this probe exists to see: a state write is pending and the scheduler has
    // not applied it yet. Synchronously after the pulse, a healthy scheduler has not run
    // either — settled only distinguishes health once the flush has had its chance.
    it("is unsettled between a pulse and the flush", () => {
        const probe = new ReactivityProbe();
        probe.pulse();
        expect(probe.settled).toBe(false);
        flushSync();
        expect(probe.settled).toBe(true);
        probe.cleanup();
    });

    it("settles on its own when the scheduler is healthy", async () => {
        const probe = new ReactivityProbe();
        probe.pulse();
        await tick();
        expect(probe.settled).toBe(true);
        probe.cleanup();
    });

    it("keeps settling across repeated pulses", async () => {
        const probe = new ReactivityProbe();
        for (let round = 0; round < 3; round++) {
            probe.pulse();
            await tick();
            expect(probe.settled).toBe(true);
        }
        probe.cleanup();
    });

    it("stops reacting after cleanup", async () => {
        const probe = new ReactivityProbe();
        probe.cleanup();
        probe.pulse();
        await tick();
        expect(probe.settled).toBe(false);
    });
});
