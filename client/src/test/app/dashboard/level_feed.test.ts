import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke } from "../../tauri";
import type { LevelSnapshot } from "../../../js/bindings/LevelSnapshot";

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

const { LevelFeed } = await import("../../../js/app/dashboard/LevelFeed");
const { EventChannel } = await import("../../../js/app/events/EventChannel");

const SILENT = { own: { speaking: false, loudness: 0 }, peers: {} };

async function settle(): Promise<void> {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
}

// The feed is a shared object, so a test that leaves a sink subscribed leaves the next one
// holding an already-open channel subscription and no new socket to drive.
const held: Array<() => void> = [];

function subscribe(sink: (snapshot: LevelSnapshot) => void): () => void {
    const off = LevelFeed.shared().subscribe(sink);
    held.push(off);
    return off;
}

beforeEach(() => {
    FakeSocket.instances = [];
    mockInvoke({ websocket_internal_endpoint: () => ({ port: 5555, token: "tok" }) });
    EventChannel.shared().resetForTest();
});

afterEach(() => {
    while (held.length > 0) held.pop()?.();
});

describe("LevelFeed", () => {
    it("feeds every sink from one socket", async () => {
        const first: unknown[] = [];
        const second: unknown[] = [];
        subscribe((s) => first.push(s));
        subscribe((s) => second.push(s));
        await settle();

        expect(FakeSocket.instances).toHaveLength(1);
        FakeSocket.instances[0].open();
        FakeSocket.instances[0].deliver({ type: "levels", data: SILENT });

        expect(first).toEqual([SILENT]);
        expect(second).toEqual([SILENT]);
    });

    it("counts what it delivered", async () => {
        subscribe(() => {});
        await settle();
        FakeSocket.instances[0].open();
        FakeSocket.instances[0].deliver({ type: "levels", data: SILENT });
        FakeSocket.instances[0].deliver({ type: "levels", data: SILENT });

        expect(LevelFeed.shared().received).toBe(2);
    });

    // A frame of another type on the same socket must not be counted as a level or pushed to a
    // meter, or the diagnostics event count stops meaning what its label says.
    it("ignores frames of other types", async () => {
        const seen: unknown[] = [];
        subscribe((s) => seen.push(s));
        await settle();
        FakeSocket.instances[0].open();
        FakeSocket.instances[0].deliver({ type: "health", data: { status: "Connected" } });

        expect(seen).toHaveLength(0);
        expect(LevelFeed.shared().received).toBe(0);
    });

    it("releases the socket when the last sink leaves", async () => {
        const off = subscribe(() => {});
        await settle();
        FakeSocket.instances[0].open();
        expect(LevelFeed.shared().attached).toBe(true);

        off();
        expect(FakeSocket.instances[0].closed).toBe(true);
        expect(LevelFeed.shared().attached).toBe(false);
    });
});
