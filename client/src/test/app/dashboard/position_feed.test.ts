import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke } from "../../tauri";

/** Every socket the feed has opened, in order, so a redial is observable. */
const sockets: FakeSocket[] = [];

class FakeSocket {
    onopen: (() => void) | null = null;
    onmessage: ((event: { data: string }) => void) | null = null;
    onclose: (() => void) | null = null;
    onerror: (() => void) | null = null;
    closed = false;

    constructor(
        readonly url: string,
        readonly protocols: string[],
    ) {
        sockets.push(this);
    }

    close(): void {
        this.closed = true;
    }
}

vi.stubGlobal("WebSocket", FakeSocket);

const { PositionFeed } = await import("../../../js/app/dashboard/PositionFeed");

/** Runs the feed up to its first socket, which takes a ticket round trip to reach. */
async function started(): Promise<InstanceType<typeof PositionFeed>> {
    const feed = new PositionFeed("https://voice.example.com", () => {});
    const starting = feed.start();
    await vi.advanceTimersByTimeAsync(0);
    await starting;
    return feed;
}

/**
 * Long enough for any scheduled retry to fire.
 *
 * The delay carries jitter, so a test that waits exactly one backoff is a coin flip.
 */
async function pastRetry(backoffMs: number): Promise<void> {
    await vi.advanceTimersByTimeAsync(backoffMs + PositionFeed.BACKOFF_MIN_MS + 100);
}

describe("PositionFeed", () => {
    beforeEach(() => {
        vi.useFakeTimers();
        sockets.length = 0;
        mockInvoke({
            api_websocket_ticket: () => ({ ticket: "abc", expires_in: 60 }),
        });
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    /**
     * A failed socket reports `error` and `close`, but the two are not guaranteed to both
     * arrive — and retrying from `close` alone left the feed holding a socket nothing would
     * ever replace, because `connect` declines to open a second one.
     */
    it("redials after a socket error, without waiting for a close that may not come", async () => {
        const feed = await started();
        expect(sockets).toHaveLength(1);

        sockets[0].onerror!();
        await pastRetry(PositionFeed.BACKOFF_MIN_MS);

        expect(sockets).toHaveLength(2);
        feed.stop();
    });

    /**
     * The shape of a link lost mid-session rather than refused: the connection is discarded in
     * flight, so the socket sits in CONNECTING and reports nothing at all. Nothing here can
     * distinguish that from a slow server, so it is timed out rather than waited on.
     */
    it("abandons a socket that never opens", async () => {
        const feed = await started();

        await vi.advanceTimersByTimeAsync(PositionFeed.OPEN_TIMEOUT_MS + 100);
        expect(sockets[0].closed).toBe(true);

        await pastRetry(PositionFeed.BACKOFF_MIN_MS);
        expect(sockets).toHaveLength(2);
        feed.stop();
    });

    it("backs off, so a server that is down is not redialled at a fixed rate", async () => {
        const feed = await started();

        sockets[0].onclose!();
        await pastRetry(PositionFeed.BACKOFF_MIN_MS);
        expect(sockets).toHaveLength(2);

        sockets[1].onclose!();
        // Only as far as the first delay: the second must be longer than it, or nothing is
        // backing off.
        await vi.advanceTimersByTimeAsync(PositionFeed.BACKOFF_MIN_MS);
        expect(sockets).toHaveLength(2);

        await pastRetry(PositionFeed.BACKOFF_MIN_MS * 2);
        expect(sockets).toHaveLength(3);
        feed.stop();
    });

    /**
     * A feed that reconnected once and dropped again is not in the state its last outage left
     * it in, and inheriting that outage's delay would make a brief second drop take a minute
     * to recover from.
     */
    it("returns to the shortest delay once a socket opens", async () => {
        const feed = await started();

        sockets[0].onclose!();
        await pastRetry(PositionFeed.BACKOFF_MIN_MS);
        sockets[1].onclose!();
        await pastRetry(PositionFeed.BACKOFF_MIN_MS * 2);
        expect(sockets).toHaveLength(3);

        sockets[2].onopen!();
        sockets[2].onclose!();
        await pastRetry(PositionFeed.BACKOFF_MIN_MS);

        expect(sockets).toHaveLength(4);
        feed.stop();
    });

    it("stops redialling once the feed is stopped", async () => {
        const feed = await started();

        sockets[0].onerror!();
        feed.stop();
        await pastRetry(PositionFeed.BACKOFF_MAX_MS);

        expect(sockets).toHaveLength(1);
    });
});
