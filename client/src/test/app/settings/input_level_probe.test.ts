import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../tauri";

/** The app-event bus, under the test's control, so "a session is capturing" is expressible. */
const listeners = new Map<string, (event: { payload: unknown }) => void>();
let unlistened = 0;

vi.mock("@tauri-apps/api/event", () => ({
    listen: async (event: string, run: (e: { payload: unknown }) => void) => {
        listeners.set(event, run);
        return () => {
            unlistened += 1;
            listeners.delete(event);
        };
    },
}));

const { InputLevelProbe } = await import("../../../js/app/settings/InputLevelProbe");
const { LevelFeed } = await import("../../../js/app/dashboard/LevelFeed");

/** What a stream this probe started emits: the unquantised amplitude. */
function emitRaw(rms: number, gateOpen = false): void {
    listeners.get("audio-input-level")?.({ payload: { rms, gate_open: gateOpen } });
}

/** The backend's answer to the feed's verification probe: one snapshot, through the listener. */
function answerProbe(): void {
    listeners.get("audio-levels")?.({
        payload: { own: { speaking: false, loudness: 0 }, peers: {} },
    });
}

/** What a live session publishes: a quantised step and a speaking flag. */
function emitSession(loudness: number, speaking = true): void {
    listeners.get("audio-levels")?.({
        payload: { own: { speaking, loudness }, peers: {} },
    });
}

/**
 * Let the shared feed's registration land.
 *
 * `subscribe` opens it without awaiting — the caller gets a sink immediately and the listener
 * follows — so a test that emits on the next line emits into nothing.
 */
async function settle(): Promise<void> {
    await new Promise((resolve) => setTimeout(resolve, 0));
}

function meterCalls(): number {
    return invokeCalls().filter((c) => c.cmd === "start_input_meter").length;
}

describe("InputLevelProbe", () => {
    beforeEach(() => {
        listeners.clear();
        unlistened = 0;
        LevelFeed.shared().forgetRegistrationForTest();
        mockInvoke({
            input_capture_active: () => false,
            start_input_meter: () => null,
            stop_input_meter: () => null,
            probe_audio_levels: () => answerProbe(),
        });
    });

    /**
     * The invariant the whole class exists for. `start_input_meter` runs
     * `AudioStreamManager::init`, which stops and replaces the input stream with one that meters
     * without transmitting — so calling it while a session is capturing takes the microphone off
     * the air because somebody opened a settings pane.
     */
    it("does not start a stream when a session is already capturing", async () => {
        mockInvoke({ input_capture_active: () => true, stop_input_meter: () => null });

        const probe = new InputLevelProbe();
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
            mockInvoke({ input_capture_active: () => true, stop_input_meter: () => null });

            const probe = new InputLevelProbe();
            await probe.start();

            // Nothing emitted at all: the session is up and the room is quiet.
            await vi.advanceTimersByTimeAsync(60_000);

            expect(meterCalls()).toBe(0);
        } finally {
            vi.useRealTimers();
        }
    });

    // The pane reached before connecting, where nothing is capturing and the meter would sit
    // flat forever — which reads as a dead microphone rather than as no session.
    it("starts its own stream when nothing is capturing, and stops that one", async () => {
        const probe = new InputLevelProbe();
        await probe.start();

        expect(meterCalls()).toBe(1);

        await probe.stop();
        expect(invokeCalls().some((c) => c.cmd === "stop_input_meter")).toBe(true);
    });

    it("shows a live session's level without owning a stream", async () => {
        mockInvoke({
            input_capture_active: () => true,
            stop_input_meter: () => null,
            probe_audio_levels: () => answerProbe(),
        });

        const probe = new InputLevelProbe();
        await probe.start();
        await settle();

        emitSession(4);
        expect(get(probe.rms)).toBeGreaterThan(0);
        expect(get(probe.gateOpen)).toBe(true);
        await probe.stop();
    });

    it("shows the unquantised amplitude from a stream it started", async () => {
        const probe = new InputLevelProbe();
        await probe.start();

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
            input_capture_active: () => {
                throw new Error("no audio manager");
            },
            stop_input_meter: () => null,
        });

        const probe = new InputLevelProbe();
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
            input_capture_active: () => pending,
            stop_input_meter: () => null,
        });

        const probe = new InputLevelProbe();
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
            input_capture_active: () => false,
            start_input_meter: () => {
                throw new Error("device is held by another application");
            },
        });

        const probe = new InputLevelProbe();
        await probe.start();

        expect(get(probe.available)).toBe(false);
    });
});

/**
 * The pane must not open a second `audio-levels` registration.
 *
 * It used to `listen` for itself, so a window with settings open held two registrations for one
 * event and tore one of them down on the way out. The dashboard's is a singleton opened once at
 * boot; the pane's is opened and dropped on every visit, which is why the pane's meter always
 * worked and the pill did not. Going through the shared feed leaves exactly one registration in
 * the window however many meters are reading it, and closing the pane drops a sink rather than
 * a listener.
 */
describe("the pane's share of the level feed", () => {
    beforeEach(() => {
        listeners.clear();
        LevelFeed.shared().forgetRegistrationForTest();
    });

    function levelRegistrations(): number {
        return [...listeners.keys()].filter((e) => e === "audio-levels").length;
    }

    it("adds no registration of its own for audio-levels", async () => {
        mockInvoke({
            input_capture_active: () => true,
            probe_audio_levels: () => answerProbe(),
            stop_input_meter: () => null,
        });

        const before = levelRegistrations();
        const probe = new InputLevelProbe();
        await probe.start();

        // One shared registration at most, whoever opened it.
        expect(levelRegistrations()).toBeLessThanOrEqual(Math.max(before, 1));

        await probe.stop();
    });

    it("still shows a live session's level through the shared feed", async () => {
        mockInvoke({
            input_capture_active: () => true,
            probe_audio_levels: () => answerProbe(),
            stop_input_meter: () => null,
        });

        const probe = new InputLevelProbe();
        await probe.start();
        await settle();

        emitSession(4);

        expect(get(probe.rms)).toBeGreaterThan(0);
        expect(get(probe.gateOpen)).toBe(true);
        await probe.stop();
    });

    // Closing the pane must not take the dashboard's levels with it.
    it("leaves the feed delivering after the pane closes", async () => {
        mockInvoke({
            input_capture_active: () => true,
            probe_audio_levels: () => answerProbe(),
            stop_input_meter: () => null,
        });

        const seen: unknown[] = [];
        const off = LevelFeed.shared().subscribe((s) => seen.push(s));
        await settle();

        const probe = new InputLevelProbe();
        await probe.start();
        await settle();
        await probe.stop();

        emitSession(5);
        expect(seen.length).toBeGreaterThan(0);
        off();
    });
});
