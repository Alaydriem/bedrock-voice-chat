import { get } from "svelte/store";
import { describe, expect, it, vi } from "vitest";
import { UpdateStatus } from "../../../js/app/settings/UpdateStatus";

describe("UpdateStatus", () => {
    it("starts idle, having checked nothing", () => {
        const state = get(new UpdateStatus(async () => null).state);
        expect(state.kind).toBe("idle");
        expect(state.checkedAt).toBeNull();
    });

    it("reports the waiting version", async () => {
        const status = new UpdateStatus(async () => "1.0.0-beta.9");
        await status.check();
        const state = get(status.state);
        expect(state.kind).toBe("available");
        expect(state.version).toBe("1.0.0-beta.9");
    });

    it("records when it last checked, so the row can say so", async () => {
        const status = new UpdateStatus(async () => null);
        await status.check();
        const state = get(status.state);
        expect(state.kind).toBe("current");
        expect(state.checkedAt).toBeGreaterThan(0);
    });

    // No updater is a build installed from a store, not a failure. "Couldn't check for
    // updates" on an MSIX install sends somebody looking for a problem they do not have.
    it("distinguishes no updater from a failed check", async () => {
        const status = new UpdateStatus(async () => {
            throw new Error("No updater available on this platform");
        });
        await status.check();
        expect(get(status.state).kind).toBe("unavailable");
    });

    it("reports a check that could not reach the update server", async () => {
        const status = new UpdateStatus(async () => {
            throw new Error("error sending request for url");
        });
        await status.check();
        expect(get(status.state).kind).toBe("failed");
    });

    // Two presses must not race to a stale answer.
    it("ignores a second check while one is in flight", async () => {
        const check = vi.fn(async () => null);
        const status = new UpdateStatus(check);
        await Promise.all([status.check(), status.check()]);
        expect(check).toHaveBeenCalledTimes(1);
    });

    // The badge is the only thing in the nav that draws the eye, so it means one thing.
    it("badges the nav only when an update is actually waiting", async () => {
        const waiting = new UpdateStatus(async () => "1.0.0-beta.9");
        await waiting.check();
        expect(get(waiting.badge)).toBe(true);

        const current = new UpdateStatus(async () => null);
        await current.check();
        expect(get(current.badge)).toBe(false);
    });
});
