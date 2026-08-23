import { afterEach, describe, expect, it, vi } from "vitest";
import {
    ReactivityWatchdog,
    type SchedulerProbe,
} from "../../../js/app/services/ReactivityWatchdog";

class FakeProbe implements SchedulerProbe {
    pulses = 0;
    settled = true;

    pulse(): void {
        this.pulses++;
    }
}

function build(
    over: { settled?: boolean; heartbeatMs?: number; minGapMs?: number } = {},
) {
    const probe = new FakeProbe();
    probe.settled = over.settled ?? true;
    const flush = vi.fn(() => {
        // flushSync applies the pending write, which is what settles the real probe.
        probe.settled = true;
    });
    const report = vi.fn();
    const watchdog = new ReactivityWatchdog(probe, flush, report, {
        heartbeatMs: over.heartbeatMs,
        minGapMs: over.minGapMs,
    });
    return { probe, flush, report, watchdog };
}

function setVisibility(state: "visible" | "hidden"): void {
    Object.defineProperty(document, "visibilityState", {
        configurable: true,
        value: state,
    });
}

afterEach(() => {
    setVisibility("visible");
});

describe("ReactivityWatchdog.check", () => {
    it("leaves a healthy scheduler alone", async () => {
        const { probe, flush, report, watchdog } = build({ settled: true });
        const healed = await watchdog.check();
        expect(healed).toBe(false);
        expect(probe.pulses).toBe(1);
        expect(flush).not.toHaveBeenCalled();
        expect(report).not.toHaveBeenCalled();
    });

    it("flushes and reports a wedged scheduler", async () => {
        const { flush, report, watchdog } = build({ settled: false });
        const healed = await watchdog.check();
        expect(healed).toBe(true);
        expect(flush).toHaveBeenCalledTimes(1);
        expect(report).toHaveBeenCalledTimes(1);
    });

    // Two resumes in quick succession must not race two probes through the same sentinel.
    it("runs one check at a time", async () => {
        const { probe, watchdog } = build({ settled: true });
        const [first, second] = await Promise.all([watchdog.check(), watchdog.check()]);
        expect(probe.pulses).toBe(1);
        expect(first).toBe(false);
        expect(second).toBe(false);
    });

    // flushSync throwing must not leave the watchdog latched shut — the exact failure
    // shape it exists to guard against.
    it("survives a flush that throws and can check again", async () => {
        const probe = new FakeProbe();
        probe.settled = false;
        const flush = vi.fn(() => {
            throw new Error("flush failed");
        });
        const report = vi.fn();
        const watchdog = new ReactivityWatchdog(probe, flush, report);

        await expect(watchdog.check()).rejects.toThrow("flush failed");
        probe.settled = true;
        await expect(watchdog.check()).resolves.toBe(false);
    });
});

describe("ReactivityWatchdog visibility wiring", () => {
    it("checks when the document becomes visible", async () => {
        const { probe, watchdog } = build({ settled: true });
        watchdog.start();
        setVisibility("visible");
        document.dispatchEvent(new Event("visibilitychange"));
        await vi.waitFor(() => expect(probe.pulses).toBe(1));
        watchdog.cleanup();
    });

    it("ignores the document going hidden", async () => {
        const { probe, watchdog } = build({ settled: true });
        watchdog.start();
        setVisibility("hidden");
        document.dispatchEvent(new Event("visibilitychange"));
        await new Promise((r) => setTimeout(r, 10));
        expect(probe.pulses).toBe(0);
        watchdog.cleanup();
    });

    it("stops listening after cleanup", async () => {
        const { probe, watchdog } = build({ settled: true });
        watchdog.start();
        watchdog.cleanup();
        setVisibility("visible");
        document.dispatchEvent(new Event("visibilitychange"));
        await new Promise((r) => setTimeout(r, 10));
        expect(probe.pulses).toBe(0);
    });
});

describe("ReactivityWatchdog interaction wiring", () => {
    // A wedge that lands while the app is visible shows itself as a tap that does
    // nothing. The tap is also the trigger: checking on pointerdown means the first
    // frozen tap flushes the backlog — including that same tap — one macrotask later.
    it("checks on a pointerdown", async () => {
        const { probe, watchdog } = build({ settled: true });
        watchdog.start();
        document.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
        await vi.waitFor(() => expect(probe.pulses).toBe(1));
        watchdog.cleanup();
    });

    it("throttles pointerdown checks to the minimum gap", async () => {
        const { probe, watchdog } = build({ settled: true, minGapMs: 60_000 });
        watchdog.start();
        document.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
        document.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
        document.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
        await vi.waitFor(() => expect(probe.pulses).toBe(1));
        await new Promise((r) => setTimeout(r, 10));
        expect(probe.pulses).toBe(1);
        watchdog.cleanup();
    });

    it("ignores pointerdown after cleanup", async () => {
        const { probe, watchdog } = build({ settled: true });
        watchdog.start();
        watchdog.cleanup();
        document.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
        await new Promise((r) => setTimeout(r, 10));
        expect(probe.pulses).toBe(0);
    });
});

describe("ReactivityWatchdog heartbeat", () => {
    // A frozen meter with no finger anywhere near the screen: nothing fires pointerdown
    // or visibilitychange, so the heartbeat is what notices.
    it("checks on its own while nothing else triggers", async () => {
        const { probe, watchdog } = build({ settled: true, heartbeatMs: 10, minGapMs: 0 });
        watchdog.start();
        await vi.waitFor(() => expect(probe.pulses).toBeGreaterThanOrEqual(2));
        watchdog.cleanup();
    });

    it("stops the heartbeat on cleanup", async () => {
        const { probe, watchdog } = build({ settled: true, heartbeatMs: 10, minGapMs: 0 });
        watchdog.start();
        await vi.waitFor(() => expect(probe.pulses).toBeGreaterThanOrEqual(1));
        watchdog.cleanup();
        const seen = probe.pulses;
        await new Promise((r) => setTimeout(r, 40));
        expect(probe.pulses).toBe(seen);
    });
});
