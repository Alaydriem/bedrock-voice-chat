import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke } from "../../tauri";

/** The event bridge, recording every registration so a dropped one is visible. */
interface Registration {
    readonly event: string;
    readonly run: (e: { payload: unknown }) => void;
    dropped: boolean;
}

const registrations: Registration[] = [];

vi.mock("@tauri-apps/api/event", () => ({
    listen: async (event: string, run: (e: { payload: unknown }) => void) => {
        const registration: Registration = { event, run, dropped: false };
        registrations.push(registration);
        return async () => {
            registration.dropped = true;
        };
    },
}));

const { PlayerLevelSources } = await import("../../../js/app/dashboard/PlayerLevelSources");
const { LevelFeed } = await import("../../../js/app/dashboard/LevelFeed");

const SPEAKING = { own: { speaking: true, loudness: 4 }, peers: {} };

function emit(payload: unknown = SPEAKING): void {
    for (const registration of registrations) {
        if (registration.event !== "audio-levels" || registration.dropped) continue;
        registration.run({ payload });
    }
}

function live(): number {
    return registrations.filter((r) => r.event === "audio-levels" && !r.dropped).length;
}

/**
 * Restarting the fan-out must not leave the feed without an audience, even for an instant.
 *
 * `LevelFeed` closes its one registration when the last sink goes, and re-opens it for the
 * next. Unsubscribing before re-subscribing puts a close and an open back to back with an
 * `await` inside the open, which is the window a teardown can land in — and the attempt that
 * then succeeds is no longer the held one. The sinks stay subscribed, the feed holds nothing,
 * and the meters are flat for the life of the page.
 */
describe("restarting the level fan-out", () => {
    beforeEach(() => {
        vi.useFakeTimers();
        registrations.length = 0;
        mockInvoke({ probe_audio_levels: () => emit() });
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it("keeps a live registration across a restart", async () => {
        const sources = new PlayerLevelSources();
        await sources.start();
        await vi.advanceTimersByTimeAsync(0);
        expect(live()).toBe(1);

        await sources.start();
        await vi.advanceTimersByTimeAsync(0);

        expect(live()).toBe(1);
        expect(LevelFeed.shared().attached).toBe(true);
        sources.stop();
    });

    it("still receives levels after a restart", async () => {
        const sources = new PlayerLevelSources();
        await sources.start();
        await vi.advanceTimersByTimeAsync(0);
        await sources.start();
        await vi.advanceTimersByTimeAsync(0);

        const before = sources.activity.events;
        emit();

        expect(sources.activity.events).toBeGreaterThan(before);
        expect(sources.activity.lastRms).toBeGreaterThan(0);
        sources.stop();
    });

    // The audience really has gone, so the registration must go with it.
    it("drops the registration when it is stopped for good", async () => {
        const sources = new PlayerLevelSources();
        await sources.start();
        await vi.advanceTimersByTimeAsync(0);

        sources.stop();

        expect(live()).toBe(0);
    });
});
