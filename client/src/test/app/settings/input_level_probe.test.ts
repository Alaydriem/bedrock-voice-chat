import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../tauri";

/** The push channel, under the test's control, so "a session is capturing" is expressible. */
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

const { InputLevelProbe } = await import("../../../js/app/settings/InputLevelProbe");
const { LevelFeed } = await import("../../../js/app/dashboard/LevelFeed");
const { EventChannel } = await import("../../../js/app/events/EventChannel");

const ENDPOINT = { port: 5555, token: "tok" };

function socket(): FakeSocket | undefined {
    return FakeSocket.instances.find((s) => !s.closed);
}

/** What a capture publishes: a quantised step and a gate verdict, for everyone at once. */
function emitSession(loudness: number, speaking = true): void {
    socket()?.deliver({ type: "levels", data: { own: { speaking, loudness }, peers: {} } });
}

async function settle(): Promise<void> {
    await new Promise((resolve) => setTimeout(resolve, 0));
}

const opened: Array<{ stop: () => Promise<void> }> = [];

function probing(): InstanceType<typeof InputLevelProbe> {
    const probe = new InputLevelProbe();
    opened.push(probe);
    return probe;
}

/**
 * The pane reads the capture the pill draws, and starts nothing.
 *
 * It used to claim a stream of its own when it believed nothing was capturing.
 * `start_input_meter` runs `AudioStreamManager::init`, which replaces the input stream with one
 * that meters without transmitting, and the matching stop left nothing capturing at all — so a
 * visit to this pane took the microphone off the air and left every dashboard meter flat for the
 * rest of the session, while the pane's own meter kept working because each visit built it a
 * fresh capture. Reading the shared feed makes that structurally impossible.
 */
describe("InputLevelProbe", () => {
    beforeEach(() => {
        FakeSocket.instances = [];
        EventChannel.shared().resetForTest();
        mockInvoke({ websocket_internal_endpoint: () => ENDPOINT });
    });

    afterEach(async () => {
        while (opened.length > 0) await opened.pop()?.stop();
    });

    // The invariant the class now keeps by construction rather than by asking.
    it("never asks the backend for a capture", async () => {
        const probe = probing();
        await probe.start();
        await settle();
        emitSession(4);
        await probe.stop();

        expect(invokeCalls().map((c) => c.cmd)).not.toContain("start_input_meter");
        expect(invokeCalls().map((c) => c.cmd)).not.toContain("stop_input_meter");
    });

    it("drives its meter source from the shared feed", async () => {
        const probe = probing();
        const seen: number[] = [];
        probe.source.subscribe((level) => seen.push(level));
        await probe.start();
        await settle();

        emitSession(4);

        expect(probe.source.level).toBeGreaterThan(0);
        expect(seen.at(-1)).toBe(probe.source.level);
    });

    it("opens no socket of its own", async () => {
        const probe = probing();
        await probe.start();
        await settle();

        // One shared socket at most, whoever opened it.
        expect(FakeSocket.instances).toHaveLength(1);
    });

    /**
     * A capture that dies mid-word leaves the last amplitude standing, and a mark held at
     * half-height reads as somebody still talking — the one thing a meter must never say.
     */
    it("returns the meter to rest when the levels stop", async () => {
        vi.useFakeTimers();
        try {
            const probe = probing();
            await probe.start();
            await vi.advanceTimersByTimeAsync(0);
            emitSession(6);
            expect(probe.source.level).toBeGreaterThan(0);

            await vi.advanceTimersByTimeAsync(InputLevelProbe.SILENCE_MS * 2 + 100);

            expect(probe.source.level).toBe(0);
        } finally {
            vi.useRealTimers();
        }
    });

    // A quiet room is not a dead capture, and levels are published on change — so an arriving
    // level has to hold the meter open rather than merely move it once.
    it("holds a speaking level while levels keep arriving", async () => {
        vi.useFakeTimers();
        try {
            const probe = probing();
            await probe.start();
            await vi.advanceTimersByTimeAsync(0);

            for (let i = 0; i < 4; i += 1) {
                await vi.advanceTimersByTimeAsync(InputLevelProbe.SILENCE_MS - 500);
                emitSession(6);
            }

            expect(probe.source.level).toBeGreaterThan(0);
        } finally {
            vi.useRealTimers();
        }
    });

    // Closing the pane must not take the dashboard's levels with it.
    it("leaves the feed delivering after the pane closes", async () => {
        const seen: unknown[] = [];
        const off = LevelFeed.shared().subscribe((s) => seen.push(s));
        await settle();

        const probe = probing();
        await probe.start();
        await settle();
        await probe.stop();

        emitSession(5);
        expect(seen.length).toBeGreaterThan(0);
        off();
    });
});
