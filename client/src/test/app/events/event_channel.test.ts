import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke } from "../../tauri";

/** A WebSocket the test drives, standing in for the app's own loopback listener. */
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

    drop(code = 1006): void {
        this.readyState = 3;
        this.onclose?.({ code });
    }

    close(): void {
        this.closed = true;
        this.readyState = 3;
    }
}

vi.stubGlobal("WebSocket", FakeSocket);

const { EventChannel } = await import("../../../js/app/events/EventChannel");

function endpoint(): void {
    mockInvoke({ websocket_internal_endpoint: () => ({ port: 41234, token: "tok" }) });
}

async function settle(): Promise<void> {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
}

beforeEach(() => {
    FakeSocket.instances = [];
    vi.useFakeTimers();
    endpoint();
    EventChannel.shared().resetForTest();
});

afterEach(() => {
    vi.useRealTimers();
});

describe("EventChannel", () => {
    it("dials the literal loopback address with the process token", async () => {
        EventChannel.shared().subscribe("levels", () => {});
        await settle();

        expect(FakeSocket.instances).toHaveLength(1);
        expect(FakeSocket.instances[0].url).toBe("ws://127.0.0.1:41234/events?key=tok");
    });

    it("delivers a frame only to the subscribers of its type", async () => {
        const levels: unknown[] = [];
        const health: unknown[] = [];
        EventChannel.shared().subscribe("levels", (d) => levels.push(d));
        EventChannel.shared().subscribe("health", (d) => health.push(d));
        await settle();

        const socket = FakeSocket.instances[0];
        socket.open();
        socket.deliver({ type: "levels", data: { own: { speaking: true, loudness: 4 }, peers: {} } });

        expect(levels).toHaveLength(1);
        expect(health).toHaveLength(0);
    });

    // One sink throwing must not stop the others being fed. A handler that failed and a frame
    // that never came are different faults with different fixes.
    it("isolates a sink that throws", async () => {
        const seen: unknown[] = [];
        EventChannel.shared().subscribe("levels", () => {
            throw new Error("sink failed");
        });
        EventChannel.shared().subscribe("levels", (d) => seen.push(d));
        await settle();

        FakeSocket.instances[0].open();
        FakeSocket.instances[0].deliver({ type: "levels", data: {} });

        expect(seen).toHaveLength(1);
    });

    it("reconnects with backoff after the socket drops", async () => {
        EventChannel.shared().subscribe("levels", () => {});
        await settle();
        FakeSocket.instances[0].open();
        FakeSocket.instances[0].drop();

        expect(FakeSocket.instances).toHaveLength(1);
        await vi.advanceTimersByTimeAsync(250);
        await settle();
        expect(FakeSocket.instances).toHaveLength(2);
    });

    // Every payload on this channel is absolute state, so a reconnect re-seeds rather than
    // resuming. A meter left at its last value across a gap reads as somebody still talking.
    it("reports disconnected until a socket opens again", async () => {
        EventChannel.shared().subscribe("levels", () => {});
        await settle();
        FakeSocket.instances[0].open();
        expect(EventChannel.shared().connected).toBe(true);

        FakeSocket.instances[0].drop();
        expect(EventChannel.shared().connected).toBe(false);
    });

    it("closes the socket when the last subscriber leaves", async () => {
        const off = EventChannel.shared().subscribe("levels", () => {});
        await settle();
        FakeSocket.instances[0].open();
        off();

        expect(FakeSocket.instances[0].closed).toBe(true);
    });
});
