import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke } from "../../tauri";

/**
 * The event bridge, with the fault this suite exists for built in.
 *
 * On Android a `listen()` can resolve while its page-side dispatch entry was never written —
 * the registration eval is fire-and-forget — leaving a listener the backend dispatches to and
 * the page silently skips. That phantom cannot be observed from inside the page, so the feed
 * verifies every registration with a probed round trip. This registry models both kinds of
 * registration: live ones receive what `emitLevels` sends, phantoms receive nothing, and both
 * report success to the caller.
 */
interface Registration {
    readonly event: string;
    readonly run: (e: { payload: unknown }) => void;
    readonly phantom: boolean;
    dropped: boolean;
}

const registrations: Registration[] = [];
let phantomsAhead = 0;
let unlistenThrowsOnPhantom = false;

vi.mock("@tauri-apps/api/event", () => ({
    listen: async (event: string, run: (e: { payload: unknown }) => void) => {
        const registration: Registration = {
            event,
            run,
            phantom: phantomsAhead > 0,
            dropped: false,
        };
        if (phantomsAhead > 0) phantomsAhead -= 1;
        registrations.push(registration);
        return async () => {
            registration.dropped = true;
            // Tauri's injected unregister helper does `listeners[eventId].handlerId` without
            // a guard, so tearing down exactly the registration that never materialised throws.
            if (unlistenThrowsOnPhantom && registration.phantom) {
                throw new TypeError("Cannot read properties of undefined (reading 'handlerId')");
            }
        };
    },
}));

const { LevelFeed } = await import("../../../js/app/dashboard/LevelFeed");

const SILENT = { own: { speaking: false, loudness: 0 }, peers: {} };

function emitLevels(payload: unknown = SILENT): void {
    for (const registration of registrations) {
        if (registration.event !== "audio-levels") continue;
        if (registration.phantom || registration.dropped) continue;
        registration.run({ payload });
    }
}

function healthyProbe(): void {
    mockInvoke({ probe_audio_levels: () => emitLevels() });
}

async function flush(): Promise<void> {
    await vi.advanceTimersByTimeAsync(0);
}

describe("LevelFeed", () => {
    beforeEach(() => {
        vi.useFakeTimers();
        registrations.length = 0;
        phantomsAhead = 0;
        unlistenThrowsOnPhantom = false;
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it("verifies a healthy registration with one probe and delivers levels", async () => {
        healthyProbe();
        const feed = new LevelFeed();
        const seen: unknown[] = [];

        feed.subscribe((snapshot) => seen.push(snapshot));
        await flush();

        expect(feed.attached).toBe(true);
        expect(registrations).toHaveLength(1);

        const speaking = { own: { speaking: true, loudness: 3 }, peers: {} };
        emitLevels(speaking);
        expect(seen).toContain(speaking);
    });

    it("re-registers when the probe goes unanswered", async () => {
        healthyProbe();
        phantomsAhead = 1;
        const feed = new LevelFeed();
        const seen: unknown[] = [];

        feed.subscribe((snapshot) => seen.push(snapshot));
        // First attempt: probe answered by nothing (the registration is a phantom), judged
        // dead after the timeout; one backoff later the second registration verifies.
        await vi.advanceTimersByTimeAsync(
            LevelFeed.PROBE_TIMEOUT_MS + LevelFeed.RETRY_BASE_MS,
        );

        expect(registrations).toHaveLength(2);
        expect(registrations[0].dropped).toBe(true);
        expect(registrations[1].dropped).toBe(false);
        expect(feed.attached).toBe(true);

        emitLevels();
        expect(seen).toHaveLength(2);
    });

    it("survives the unlisten throw a phantom registration produces", async () => {
        healthyProbe();
        phantomsAhead = 1;
        unlistenThrowsOnPhantom = true;
        const feed = new LevelFeed();
        const seen: unknown[] = [];

        feed.subscribe((snapshot) => seen.push(snapshot));
        await vi.advanceTimersByTimeAsync(
            LevelFeed.PROBE_TIMEOUT_MS + LevelFeed.RETRY_BASE_MS,
        );

        expect(registrations).toHaveLength(2);
        expect(feed.attached).toBe(true);

        emitLevels();
        expect(seen).toHaveLength(2);
    });

    it("keeps the last registration when no attempt ever verifies", async () => {
        // The probe sends but nothing arrives — a bridge dead in a way retrying cannot fix.
        mockInvoke({ probe_audio_levels: () => undefined });
        const feed = new LevelFeed();

        feed.subscribe(() => undefined);
        await vi.advanceTimersByTimeAsync(60_000);

        expect(registrations).toHaveLength(LevelFeed.MAX_ATTEMPTS);
        // A false-negative probe on a busy main thread must not cost the one registration
        // that might still be working, so the last one stays.
        expect(registrations[LevelFeed.MAX_ATTEMPTS - 1].dropped).toBe(false);
        expect(feed.attached).toBe(true);
    });

    it("treats an unanswerable probe as verified rather than churning", async () => {
        // The command failing says nothing about the listener, and re-registering
        // cannot fix a probe that will fail identically every time.
        mockInvoke({});
        const feed = new LevelFeed();

        feed.subscribe(() => undefined);
        await vi.advanceTimersByTimeAsync(LevelFeed.PROBE_TIMEOUT_MS * 2);

        expect(registrations).toHaveLength(1);
        expect(registrations[0].dropped).toBe(false);
        expect(feed.attached).toBe(true);
    });

    it("closes the registration when the last sink leaves", async () => {
        healthyProbe();
        const feed = new LevelFeed();

        const off = feed.subscribe(() => undefined);
        await flush();
        off();
        await flush();

        expect(registrations[0].dropped).toBe(true);
        expect(feed.attached).toBe(false);
    });

    it("does not leave a fresh subscriber on a registration torn down for an empty audience", async () => {
        healthyProbe();
        phantomsAhead = 1;
        const feed = new LevelFeed();
        const seen: unknown[] = [];

        // The first audience leaves while its phantom registration is still being verified.
        const off = feed.subscribe(() => undefined);
        await flush();
        off();

        // A new audience arrives during the same open. It must end on a live registration.
        feed.subscribe((snapshot) => seen.push(snapshot));
        await vi.advanceTimersByTimeAsync(
            (LevelFeed.PROBE_TIMEOUT_MS + LevelFeed.RETRY_BASE_MS * 16) * LevelFeed.MAX_ATTEMPTS,
        );

        expect(feed.attached).toBe(true);
        emitLevels();
        expect(seen.length).toBeGreaterThan(0);
    });
});
