import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../tauri";

/**
 * The webview event bus, under the test's control.
 *
 * A press paints immediately and settles on the command's return value. The echo is what carries
 * changes this window did not initiate — a global hotkey, an in-game command — and asserting that
 * path means being able to deliver one, which means owning the listener registry.
 */
const listeners = new Map<string, (event: { payload: unknown }) => void>();

vi.mock("@tauri-apps/api/webviewWindow", () => ({
    getCurrentWebviewWindow: () => ({
        listen: async (event: string, run: (e: { payload: unknown }) => void) => {
            listeners.set(event, run);
            return () => listeners.delete(event);
        },
    }),
}));

function emit(event: string, payload: unknown): void {
    listeners.get(event)?.({ payload });
}

const { SelfController } = await import("../../../js/app/dashboard/SelfController");

function store(values: Record<string, unknown> = {}) {
    return {
        get: async (key: string) => values[key],
    } as never;
}

describe("SelfController", () => {
    beforeEach(() => {
        listeners.clear();
    });

    it("takes its first state from the backend rather than from defaults", async () => {
        mockInvoke({
            mute_status: (args: { device: string }) => args.device === "InputDevice",
            is_recording: () => false,
        });

        const self = new SelfController(store({ keybinds: { voiceMode: "openMic" } }));
        await self.start();

        // A mute survives a webview reload. A pill that opened unmuted would be lying, and
        // the press that "fixes" it would mute them.
        expect(self.state.snapshot.muted).toBe(true);
        expect(self.state.snapshot.deafened).toBe(false);
    });

    it("reads push-to-talk out of the saved keybinds", async () => {
        mockInvoke({ mute_status: () => false, is_recording: () => false });

        const self = new SelfController(store({ keybinds: { voiceMode: "pushToTalk" } }));
        await self.start();

        expect(self.state.snapshot.mode).toBe("ptt");
    });

    /**
     * Immediate, because waiting read as a broken control.
     *
     * The press used to move nothing until the backend echoed. That was right about authority and
     * wrong about feel: on a phone the IPC hop plus the audio thread's acknowledgement is long
     * enough that the user presses again, and the second press undoes the first.
     */
    it("moves on the press rather than making the user wait for the round trip", async () => {
        mockInvoke({
            mute_status: () => false,
            is_recording: () => false,
            set_mute: () => true,
        });

        const self = new SelfController(store());
        await self.start();
        self.pressMute();

        expect(invokeCalls().some((c) => c.cmd === "set_mute")).toBe(true);
        expect(self.state.snapshot.muted).toBe(true);
    });

    /**
     * The invariant the wait was protecting, kept.
     *
     * The backend is still the authority — it now corrects the button instead of being asked
     * first. Reconciling on the command's own return value rather than on the broadcast echo is
     * what makes that safe: the return is correlated with this press, so it cannot be applied out
     * of order with another one.
     */
    it("settles on what the backend actually reached, not on what was asked", async () => {
        mockInvoke({
            mute_status: () => false,
            is_recording: () => false,
            // Refuses the mute — a device that has gone away, say.
            set_mute: () => false,
        });

        const self = new SelfController(store());
        await self.start();
        self.pressMute();
        expect(self.state.snapshot.muted).toBe(true);

        await vi.waitFor(() => expect(self.state.snapshot.muted).toBe(false));
    });

    // The echo path stays for the changes this window did not initiate — a global hotkey, an
    // in-game command — where there is no return value to settle on.
    it("still follows an echo it did not ask for", async () => {
        mockInvoke({ mute_status: () => false, is_recording: () => false });

        const self = new SelfController(store());
        await self.start();

        emit("mute:input", true);
        expect(self.state.snapshot.muted).toBe(true);
    });

    // A command that never resolves leaves an optimistic paint standing with nothing to correct
    // it, so the failure path has to go back and ask.
    it("re-reads the backend when the command fails outright", async () => {
        mockInvoke({
            mute_status: () => false,
            is_recording: () => false,
            set_mute: () => {
                throw new Error("no such device");
            },
        });

        const self = new SelfController(store());
        await self.start();
        self.pressMute();
        expect(self.state.snapshot.muted).toBe(true);

        await vi.waitFor(() => expect(self.state.snapshot.muted).toBe(false));
    });

    it("clears both flags when the mic button is pressed while deafened", async () => {
        mockInvoke({
            mute_status: () => true,
            is_recording: () => false,
            set_deafened: () => false,
        });

        const self = new SelfController(store());
        await self.start();
        expect(self.state.snapshot.deafened).toBe(true);

        self.pressMute();

        // Not `set_mute`: pressing the mic while deafened means "put me back in the
        // conversation", and muting the input alone would leave them still hearing nobody
        // with no reason to suspect a second button.
        const sent = invokeCalls().filter((c) => c.cmd === "set_deafened");
        expect(sent).toHaveLength(1);
        expect(sent[0].args).toEqual({ deafened: false });
    });

    it("ignores the mic button in push-to-talk, where not holding already is mute", async () => {
        mockInvoke({ mute_status: () => false, is_recording: () => false });

        const self = new SelfController(store({ keybinds: { voiceMode: "pushToTalk" } }));
        await self.start();
        self.pressMute();

        expect(invokeCalls().some((c) => c.cmd === "set_mute")).toBe(false);
    });

    it("follows the global hotkey's hold, which is the whole point of a global hotkey", async () => {
        mockInvoke({ mute_status: () => false, is_recording: () => false });

        const self = new SelfController(store({ keybinds: { voiceMode: "pushToTalk" } }));
        await self.start();

        expect(self.state.snapshot.transmitting).toBe(false);
        emit("ptt:active", true);
        expect(self.state.snapshot.transmitting).toBe(true);
        emit("ptt:active", false);
        expect(self.state.snapshot.transmitting).toBe(false);
    });

    it("times a recording from when it was observed starting", async () => {
        mockInvoke({ mute_status: () => false, is_recording: () => false });

        const self = new SelfController(store());
        await self.start();

        const started = performance.now();
        emit("recording:started", null);

        // Zero rather than the milliseconds since this object was constructed, which is what
        // a stamp of 0 would have produced.
        expect(self.state.elapsed(started)).toBeLessThan(50);
    });
});
