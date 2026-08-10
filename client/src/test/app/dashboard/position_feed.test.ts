import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { info, warn } from "@tauri-apps/plugin-log";
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

/**
 * Drives a run of failed redials, following the delay the feed is waiting on.
 *
 * The delay is tracked rather than overshot: advancing well past it would run the replacement
 * socket into its own open timeout, turning one failure into several and making the counts
 * these tests assert on meaningless.
 */
async function failAttempts(count: number): Promise<void> {
    let backoff = PositionFeed.BACKOFF_MIN_MS;
    for (let i = 0; i < count; i++) {
        sockets[sockets.length - 1].onclose!();
        await pastRetry(backoff);
        backoff = Math.min(backoff * 2, PositionFeed.BACKOFF_MAX_MS);
    }
}

describe("PositionFeed", () => {
    beforeEach(() => {
        vi.useFakeTimers();
        sockets.length = 0;
        mockInvoke({
            api_websocket_ticket: () => ({ ticket: "abc", expires_in: 60 }),
        });
        vi.mocked(warn).mockClear();
        vi.mocked(info).mockClear();
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

    /**
     * A drop that the next redial repairs is the feed working. Warning about it spends the
     * breadcrumb budget that a genuine error would have been read through.
     */
    it("says nothing about a drop that the next redial repairs", async () => {
        const feed = await started();

        await failAttempts(1);
        sockets[1].onopen!();

        expect(vi.mocked(warn)).not.toHaveBeenCalled();
        expect(vi.mocked(info)).not.toHaveBeenCalled();
        feed.stop();
    });

    it("announces a sustained outage once, not once per attempt", async () => {
        const feed = await started();

        await failAttempts(PositionFeed.FAILURE_THRESHOLD + 3);

        expect(vi.mocked(warn)).toHaveBeenCalledTimes(1);
        feed.stop();
    });

    /**
     * The half that was missing: without it a reader cannot tell a feed that came back from
     * one that is still down and has merely stopped complaining about it.
     */
    it("reports the reconnection that ends an announced outage", async () => {
        const feed = await started();

        await failAttempts(PositionFeed.FAILURE_THRESHOLD);
        expect(vi.mocked(warn)).toHaveBeenCalledTimes(1);

        sockets[sockets.length - 1].onopen!();

        expect(vi.mocked(info)).toHaveBeenCalledTimes(1);
        expect(vi.mocked(info).mock.calls[0][0]).toContain("reconnected");
        feed.stop();
    });

    /** A second outage is its own event, so the pair must arm again after a recovery. */
    it("announces a later outage after recovering from an earlier one", async () => {
        const feed = await started();

        await failAttempts(PositionFeed.FAILURE_THRESHOLD);
        sockets[sockets.length - 1].onopen!();
        await failAttempts(PositionFeed.FAILURE_THRESHOLD);

        expect(vi.mocked(warn)).toHaveBeenCalledTimes(2);
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
