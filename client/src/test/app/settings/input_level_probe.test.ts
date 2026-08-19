import { get } from "svelte/store";
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

/** What a stream this probe started publishes: the unquantised amplitude. */
function emitRaw(rms: number, gateOpen = false): void {
    socket()?.deliver({ type: "input_level", data: { rms, gate_open: gateOpen } });
}

/** What a live session publishes: a quantised step and a speaking flag. */
function emitSession(loudness: number, speaking = true): void {
    socket()?.deliver({ type: "levels", data: { own: { speaking, loudness }, peers: {} } });
}

/**
 * Let the channel's socket land.
 *
 * `subscribe` opens it without awaiting — the caller gets a sink immediately and the socket
 * follows — so a test that emits on the next line emits into nothing.
 */
async function settle(): Promise<void> {
    await new Promise((resolve) => setTimeout(resolve, 0));
}

function meterCalls(): number {
    return invokeCalls().filter((c) => c.cmd === "start_input_meter").length;
}

/**
 * Probes are stopped between tests.
 *
 * `LevelFeed` and `EventChannel` are both shared, so a probe left subscribed leaves the next
 * test's feed holding a subscription against a socket this file has already thrown away.
 */
const opened: Array<{ stop: () => Promise<void> }> = [];

function probing(): InstanceType<typeof InputLevelProbe> {
    const probe = new InputLevelProbe();
    opened.push(probe);
    return probe;
}

describe("InputLevelProbe", () => {
    beforeEach(() => {
        FakeSocket.instances = [];
        EventChannel.shared().resetForTest();
        mockInvoke({
            websocket_internal_endpoint: () => ENDPOINT,
            input_capture_active: () => false,
            start_input_meter: () => null,
            stop_input_meter: () => null,
        });
    });

    afterEach(async () => {
        while (opened.length > 0) await opened.pop()?.stop();
    });

    /**
     * The invariant the whole class exists for. `start_input_meter` runs
     * `AudioStreamManager::init`, which stops and replaces the input stream with one that meters
     * without transmitting — so calling it while a session is capturing takes the microphone off
     * the air because somebody opened a settings pane.
     */
    it("does not start a stream when a session is already capturing", async () => {
        mockInvoke({
            websocket_internal_endpoint: () => ENDPOINT,
            input_capture_active: () => true,
            stop_input_meter: () => null,
        });

        const probe = probing();
        await probe.start();

        expect(meterCalls()).toBe(0);

        // And nothing it did not start is stopped on the way out.
        await probe.stop();
        expect(invokeCalls().some((c) => c.cmd === "stop_input_meter")).toBe(false);
    });

    /**
     * The regression that made asking necessary.
     *
     * This used to wait to see whether level events arrived and claim a stream when none did.
     * Levels are now published only when they change, so a room where nobody is speaking
     * produces no events at all — indistinguishable, under the old rule, from a dead capture.
     * Opening the audio pane in silence tore down a working session every time.
     */
    it("does not start a stream over a silent session, however long it stays silent", async () => {
        vi.useFakeTimers();
        try {
            mockInvoke({
                websocket_internal_endpoint: () => ENDPOINT,
                input_capture_active: () => true,
                stop_input_meter: () => null,
            });

            const probe = probing();
            await probe.start();

            // Nothing published at all: the session is up and the room is quiet.
            await vi.advanceTimersByTimeAsync(60_000);

            expect(meterCalls()).toBe(0);
        } finally {
            vi.useRealTimers();
        }
    });

    // The pane reached before connecting, where nothing is capturing and the meter would sit
    // flat forever — which reads as a dead microphone rather than as no session.
    it("starts its own stream when nothing is capturing, and stops that one", async () => {
        const probe = probing();
        await probe.start();

        expect(meterCalls()).toBe(1);

        await probe.stop();
        expect(invokeCalls().some((c) => c.cmd === "stop_input_meter")).toBe(true);
    });

    it("shows a live session's level without owning a stream", async () => {
        mockInvoke({
            websocket_internal_endpoint: () => ENDPOINT,
            input_capture_active: () => true,
            stop_input_meter: () => null,
        });

        const probe = probing();
        await probe.start();
        await settle();

        emitSession(4);
        expect(get(probe.rms)).toBeGreaterThan(0);
        expect(get(probe.gateOpen)).toBe(true);
        await probe.stop();
    });

    it("shows the unquantised amplitude from a stream it started", async () => {
        const probe = probing();
        await probe.start();
        await settle();

        emitRaw(0.4, true);
        expect(get(probe.rms)).toBe(0.4);
        expect(get(probe.gateOpen)).toBe(true);
    });

    /**
     * Being unable to ask is not evidence that nothing is capturing.
     *
     * Wrong in this direction the meter stays empty on a screen that has other things on it.
     * Wrong the other way a working microphone goes off the air, so the failure has to answer
     * "something is capturing" and leave the session alone.
     */
    it("leaves a session alone when it cannot find out whether one is running", async () => {
        mockInvoke({
            websocket_internal_endpoint: () => ENDPOINT,
            input_capture_active: () => {
                throw new Error("no audio manager");
            },
            stop_input_meter: () => null,
        });

        const probe = probing();
        await probe.start();

        expect(meterCalls()).toBe(0);
    });

    // A pane opened and closed while the question was still in flight must not leave a capture
    // stream behind it, running for a screen that is gone.
    it("does not start a stream for a pane that closed while it was asking", async () => {
        // Built before the probe runs, so the resolver exists whichever order the awaits inside
        // `start` happen to settle in.
        let answer!: (value: boolean) => void;
        const pending = new Promise<boolean>((resolve) => {
            answer = resolve;
        });
        mockInvoke({
            websocket_internal_endpoint: () => ENDPOINT,
            input_capture_active: () => pending,
            stop_input_meter: () => null,
        });

        const probe = probing();
        const starting = probe.start();
        await probe.stop();
        answer(false);
        await starting;

        expect(meterCalls()).toBe(0);
    });

    // A meter that could not start looks exactly like a microphone picking up nothing, and
    // telling those two apart is the only reason the ring is on the screen.
    it("reports a refused stream as unreadable rather than as silence", async () => {
        mockInvoke({
            websocket_internal_endpoint: () => ENDPOINT,
            input_capture_active: () => false,
            start_input_meter: () => {
                throw new Error("device is held by another application");
            },
        });

        const probe = probing();
        await probe.start();

        expect(get(probe.available)).toBe(false);
    });
});

/**
 * The pane must not open a second socket.
 *
 * It used to `listen` for itself, so a window with settings open held two registrations for one
 * event and tore one of them down on the way out. The dashboard's was a singleton opened once at
 * boot; the pane's was opened and dropped on every visit, which is why the pane's meter always
 * worked and the pill did not. Going through the shared feed and the shared channel leaves
 * exactly one socket in the window however many meters are reading it, and closing the pane
 * drops a sink rather than the transport.
 */
describe("the pane's share of the level feed", () => {
    beforeEach(() => {
        FakeSocket.instances = [];
        EventChannel.shared().resetForTest();
        mockInvoke({
            websocket_internal_endpoint: () => ENDPOINT,
            input_capture_active: () => true,
            stop_input_meter: () => null,
        });
    });

    afterEach(async () => {
        while (opened.length > 0) await opened.pop()?.stop();
    });

    it("opens no socket of its own", async () => {
        const probe = probing();
        await probe.start();
        await settle();

        // One shared socket at most, whoever opened it.
        expect(FakeSocket.instances).toHaveLength(1);

        await probe.stop();
    });

    it("still shows a live session's level through the shared feed", async () => {
        const probe = probing();
        await probe.start();
        await settle();

        emitSession(4);

        expect(get(probe.rms)).toBeGreaterThan(0);
        expect(get(probe.gateOpen)).toBe(true);
        await probe.stop();
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
