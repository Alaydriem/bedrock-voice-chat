import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke } from "../../tauri";

class FakeSocket {
    static instances: FakeSocket[] = [];
    onopen: (() => void) | null = null;
    onmessage: ((event: { data: string }) => void) | null = null;
    onclose: ((event: { code: number }) => void) | null = null;
    onerror: (() => void) | null = null;
    readyState = 0;
    closed = false;

    constructor(readonly url: string) {
        FakeSocket.instances.push(this);
    }

    open(): void {
        this.readyState = 1;
        this.onopen?.();
    }

    deliver(frame: unknown): void {
        this.onmessage?.({ data: JSON.stringify(frame) });
    }

    close(): void {
        this.closed = true;
        this.readyState = 3;
    }
}

vi.stubGlobal("WebSocket", FakeSocket);

const { PlayerLevelSources } = await import("../../../js/app/dashboard/PlayerLevelSources");
const { LevelFeed } = await import("../../../js/app/dashboard/LevelFeed");
const { EventChannel } = await import("../../../js/app/events/EventChannel");

const SPEAKING = { own: { speaking: true, loudness: 4 }, peers: {} };

function live(): FakeSocket[] {
    return FakeSocket.instances.filter((s) => !s.closed);
}

function emit(data: unknown = SPEAKING): void {
    for (const socket of live()) {
        socket.open();
        socket.deliver({ type: "levels", data });
    }
}

/**
 * Restarting the fan-out must not leave the feed without an audience.
 *
 * `LevelFeed` closes its one channel subscription when the last sink goes, and opens another
 * for the next. `start()` stops before it subscribes, so a close and an open sit back to back
 * with an `await` inside the open — and a fan-out left holding a subscription nothing feeds
 * reports itself healthy while every meter stays flat.
 */
describe("restarting the level fan-out", () => {
    beforeEach(() => {
        vi.useFakeTimers();
        FakeSocket.instances = [];
        mockInvoke({ websocket_internal_endpoint: () => ({ port: 5555, token: "tok" }) });
        EventChannel.shared().resetForTest();
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it("keeps a live subscription across a restart", async () => {
        const sources = new PlayerLevelSources();
        await sources.start();
        await vi.advanceTimersByTimeAsync(0);
        expect(live()).toHaveLength(1);

        await sources.start();
        await vi.advanceTimersByTimeAsync(0);

        expect(live()).toHaveLength(1);
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

    // The audience really has gone, so the subscription must go with it.
    it("drops the subscription when it is stopped for good", async () => {
        const sources = new PlayerLevelSources();
        await sources.start();
        await vi.advanceTimersByTimeAsync(0);

        sources.stop();

        expect(live()).toHaveLength(0);
        expect(LevelFeed.shared().attached).toBe(false);
    });
});
