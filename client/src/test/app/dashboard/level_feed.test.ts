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

/**
 * Measured on a live client: all four `audio-levels` registrations the page had ever made read
 * `callbackAlive: false` at once, with the dashboard still mounted and subscribed. Only
 * `_unlisten` deletes a callback, so the feed had torn its own registration down and never put
 * one back — and nothing noticed, because verification runs when a registration is opened and
 * never again. The meters stayed flat until the page was reloaded.
 *
 * An audience with nothing registered to feed it is the state that must not persist.
 */
describe("a registration lost while the audience remains", () => {
    beforeEach(() => {
        vi.useFakeTimers();
        registrations.length = 0;
        phantomsAhead = 0;
        unlistenThrowsOnPhantom = false;
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it("re-opens without waiting for a new subscriber", async () => {
        healthyProbe();
        const feed = new LevelFeed();
        const seen: unknown[] = [];
        feed.subscribe((snapshot) => seen.push(snapshot));
        await flush();
        expect(feed.attached).toBe(true);

        // What the live client was found in: the audience is still here, the registration is not.
        feed.forgetRegistrationForTest();
        expect(feed.attached).toBe(false);

        await vi.advanceTimersByTimeAsync(LevelFeed.WATCH_MS + 10);

        expect(feed.attached).toBe(true);
        const speaking = { own: { speaking: true, loudness: 3 }, peers: {} };
        emitLevels(speaking);
        expect(seen).toContain(speaking);
    });

    // The audience left for good. Re-opening for nobody is a listener the page pays for and
    // never reads, and the feed is meant to close when the last screen goes.
    it("does not re-open once the last subscriber has gone", async () => {
        healthyProbe();
        const feed = new LevelFeed();
        const off = feed.subscribe(() => {});
        await flush();
        off();

        const before = registrations.length;
        await vi.advanceTimersByTimeAsync(LevelFeed.WATCH_MS * 3);

        expect(registrations).toHaveLength(before);
        expect(feed.attached).toBe(false);
    });

    it("leaves a healthy registration alone", async () => {
        healthyProbe();
        const feed = new LevelFeed();
        feed.subscribe(() => {});
        await flush();

        const before = registrations.length;
        await vi.advanceTimersByTimeAsync(LevelFeed.WATCH_MS * 3);

        expect(registrations).toHaveLength(before);
        expect(feed.attached).toBe(true);
    });
});

/**
 * Unbind and rebind on demand, for a moment the app already knows about.
 *
 * The settings pane registers its own `audio-levels` listener on every mount, and that one
 * always works — a fresh registration is never the broken one. The dashboard's is a singleton
 * opened once at boot, so when it is the one that came loose there is nothing to notice. This
 * gives the screen closing over it the same fresh start the pane gets for free.
 */
describe("resyncing the registration on demand", () => {
    beforeEach(() => {
        vi.useFakeTimers();
        registrations.length = 0;
        phantomsAhead = 0;
        unlistenThrowsOnPhantom = false;
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it("replaces a held registration with a fresh, verified one", async () => {
        healthyProbe();
        const feed = new LevelFeed();
        const seen: unknown[] = [];
        feed.subscribe((snapshot) => seen.push(snapshot));
        await flush();
        const first = registrations[0];

        await feed.resync();
        await flush();

        expect(first.dropped).toBe(true);
        expect(feed.attached).toBe(true);
        expect(registrations.length).toBeGreaterThan(1);

        const speaking = { own: { speaking: true, loudness: 3 }, peers: {} };
        emitLevels(speaking);
        expect(seen).toContain(speaking);
    });

    it("opens one when the registration was already lost", async () => {
        healthyProbe();
        const feed = new LevelFeed();
        feed.subscribe(() => {});
        await flush();
        feed.forgetRegistrationForTest();

        await feed.resync();
        await flush();

        expect(feed.attached).toBe(true);
    });

    // Nothing is listening, so a registration would be one the page pays for and never reads.
    it("does nothing when there is no audience", async () => {
        healthyProbe();
        const feed = new LevelFeed();
        const off = feed.subscribe(() => {});
        await flush();
        off();
        const before = registrations.length;

        await feed.resync();
        await flush();

        expect(registrations).toHaveLength(before);
        expect(feed.attached).toBe(false);
    });
});
