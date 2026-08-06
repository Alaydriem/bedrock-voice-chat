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

    /**
     * A phone has no global hotkey, so this button is the only way into push-to-talk. Moving
     * `SelfState` here and stopping — which is what it did — lights the meter and the frame
     * stripe over a microphone the backend still has muted.
     */
    it("asks the backend to open the mic when the button is held", async () => {
        mockInvoke({ mute_status: () => false, is_recording: () => false, set_ptt: () => null });

        const self = new SelfController(store({ keybinds: { voiceMode: "pushToTalk" } }));
        await self.start();

        self.hold(true);
        self.hold(false);

        expect(invokeCalls().filter((c) => c.cmd === "set_ptt").map((c) => c.args)).toEqual([
            { down: true },
            { down: false },
        ]);
    });

    /**
     * Painted on the press, not on the answer.
     *
     * Waiting for the round trip read as a button that had not taken, and on a phone the
     * event that carries the answer does not reliably arrive at all. The backend is still
     * the authority — the poll below corrects a hold it refused.
     */
    it("paints the hold immediately", async () => {
        mockInvoke({ mute_status: () => false, is_recording: () => false, set_ptt: () => null });

        const self = new SelfController(store({ keybinds: { voiceMode: "pushToTalk" } }));
        await self.start();

        self.hold(true);
        expect(self.state.snapshot.transmitting).toBe(true);
    });

    /**
     * The mode used to be read once, in `seed`. Changing it in settings left the dashboard
     * offering a toggle for a mode where holding is the only thing that transmits — and on
     * desktop the backend had already muted the input, so that toggle unmuted for real and
     * quietly turned push-to-talk into an open mic.
     */
    it("follows a voice mode changed somewhere else", async () => {
        mockInvoke({ mute_status: () => false, is_recording: () => false });

        const self = new SelfController(store({ keybinds: { voiceMode: "openMic" } }));
        await self.start();
        expect(self.state.snapshot.mode).toBe("activated");

        emit("voice-mode:changed", "pushToTalk");
        expect(self.state.snapshot.mode).toBe("ptt");

        emit("voice-mode:changed", "openMic");
        expect(self.state.snapshot.mode).toBe("activated");
    });

    it("stops treating the mic button as a toggle the moment the mode changes", async () => {
        mockInvoke({ mute_status: () => false, is_recording: () => false });

        const self = new SelfController(store({ keybinds: { voiceMode: "openMic" } }));
        await self.start();

        emit("voice-mode:changed", "pushToTalk");
        self.pressMute();

        expect(invokeCalls().some((c) => c.cmd === "set_mute")).toBe(false);
    });

    /**
     * The fix for a mode change that never arrived.
     *
     * On Android the `voice-mode:changed` event did not reach this window, so the mic button
     * kept offering a toggle for push-to-talk — and that toggle opened the microphone for
     * real, defeating the mode. Reading the backend rather than waiting to be told makes the
     * button correct whether or not the event lands.
     */
    it("adopts the mode the backend reports, without any event", async () => {
        mockInvoke({
            mute_status: () => false,
            is_recording: () => false,
            voice_runtime_state: () => ({
                voiceMode: "pushToTalk",
                pttActive: false,
                inputMuted: true,
                outputMuted: false,
            }),
        });

        // Saved as open mic, so nothing but the backend can tell it otherwise.
        const self = new SelfController(store({ keybinds: { voiceMode: "openMic" } }));
        await self.start();

        expect(self.state.snapshot.mode).toBe("ptt");
        expect(self.state.snapshot.muted).toBe(true);
    });

    // With the mode right, the tap that used to defeat push-to-talk is refused.
    it("stops the mic button opening the mic once the backend is read", async () => {
        mockInvoke({
            mute_status: () => false,
            is_recording: () => false,
            voice_runtime_state: () => ({
                voiceMode: "pushToTalk",
                pttActive: false,
                inputMuted: true,
                outputMuted: false,
            }),
        });

        const self = new SelfController(store({ keybinds: { voiceMode: "openMic" } }));
        await self.start();
        self.pressMute();

        expect(invokeCalls().some((c) => c.cmd === "set_mute")).toBe(false);
    });

    // A hold the backend refused would otherwise leave the meter lit over a muted mic.
    it("clears a hold the backend never registered", async () => {
        mockInvoke({
            mute_status: () => false,
            is_recording: () => false,
            set_ptt: () => null,
            voice_runtime_state: () => ({
                voiceMode: "pushToTalk",
                pttActive: false,
                inputMuted: true,
                outputMuted: false,
            }),
        });

        const self = new SelfController(store({ keybinds: { voiceMode: "pushToTalk" } }));
        await self.start();
        self.hold(true);
        expect(self.state.snapshot.holding).toBe(true);

        await self.refresh();
        expect(self.state.snapshot.holding).toBe(false);
    });

    /**
     * Deafen and the mic button read the same flag, and the poll adopts it.
     *
     * The backend mutes the input on deafen and restores the voice mode's resting state on
     * undeafen, so in push-to-talk the microphone goes back to muted rather than open. The
     * button follows, instead of showing an open mic over a shut one.
     */
    it("keeps the mic button on the backend's answer through deafen", async () => {
        let inputMuted = true;
        mockInvoke({
            mute_status: () => false,
            is_recording: () => false,
            set_deafened: () => true,
            voice_runtime_state: () => ({
                voiceMode: "pushToTalk",
                pttActive: false,
                inputMuted,
                outputMuted: false,
            }),
        });

        const self = new SelfController(store({ keybinds: { voiceMode: "pushToTalk" } }));
        await self.start();
        expect(self.state.snapshot.muted).toBe(true);

        // Undeafened, and the backend puts push-to-talk back to its resting state.
        inputMuted = true;
        await self.refresh();
        expect(self.state.snapshot.muted).toBe(true);

        // A backend that reported the mic open would move the button, not be ignored.
        inputMuted = false;
        await self.refresh();
        expect(self.state.snapshot.muted).toBe(false);
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

    /**
     * Recording had neither of the two corrections the other controls have: the press did
     * not settle on what the command returned, and the reconcile poll did not carry it. So
     * a single dropped `recording:started` left the button off over a backend that was
     * recording, and the next press answered "Recording already in progress" — the toggle
     * and the truth permanently inverted, with nothing able to put them back.
     */
    it("adopts a recording the backend reports on the next poll", async () => {
        let recording = false;
        mockInvoke({
            mute_status: () => false,
            is_recording: () => recording,
            voice_runtime_state: () => ({
                voiceMode: "openMic",
                pttActive: false,
                inputMuted: false,
                outputMuted: false,
                recording,
            }),
        });

        const self = new SelfController(store());
        await self.start();
        expect(self.state.snapshot.recording).toBe(false);

        // Armed by something this window never saw: a hotkey, a Stream Deck, `/bvc`, or an
        // event that simply did not arrive.
        recording = true;
        await self.refresh();

        expect(self.state.snapshot.recording).toBe(true);
    });

    it("times a recording it learned about from the poll", async () => {
        mockInvoke({
            mute_status: () => false,
            is_recording: () => false,
            voice_runtime_state: () => ({
                voiceMode: "openMic",
                pttActive: false,
                inputMuted: false,
                outputMuted: false,
                recording: true,
            }),
        });

        const self = new SelfController(store());
        await self.start();

        const observed = performance.now();
        await self.refresh();

        expect(self.state.snapshot.recording).toBe(true);
        // Stamped when the poll observed it, not left at 0 — which would read as a
        // recording that started when this object was constructed.
        expect(self.state.elapsed(observed)).toBeLessThan(50);
    });

    it("moves the button on the press rather than waiting for the event", async () => {
        mockInvoke({
            mute_status: () => false,
            is_recording: () => false,
            start_recording: () => "session-1",
        });

        const self = new SelfController(store());
        await self.start();
        self.pressRecord();

        await vi.waitFor(() => expect(self.state.snapshot.recording).toBe(true));
    });

    /**
     * The reported symptom. The backend refuses because it is already recording, which is
     * the one case where the UI is provably the wrong one — so it re-reads rather than
     * keeping its own answer.
     */
    /**
     * The optimistic paint is a guess, and a refused command proves it wrong. Without the
     * re-read the button keeps the guess — which is how the two ended up inverted with
     * nothing able to put them back.
     */
    it("takes back the optimistic paint when the backend refuses the press", async () => {
        mockInvoke({
            mute_status: () => false,
            // Recording throughout: the stop below does not take.
            is_recording: () => true,
            stop_recording: () => {
                throw new Error("Failed to stop recording: No recording in progress");
            },
        });

        const self = new SelfController(store());
        await self.start();
        expect(self.state.snapshot.recording).toBe(true);

        self.pressRecord();

        // Painted false on the press, then corrected: the backend is still recording.
        await vi.waitFor(() => expect(self.state.snapshot.recording).toBe(true));
    });

    it("stops recording when the command succeeds", async () => {
        let recording = true;
        mockInvoke({
            mute_status: () => false,
            is_recording: () => recording,
            stop_recording: () => {
                recording = false;
                return null;
            },
        });

        const self = new SelfController(store());
        await self.start();
        expect(self.state.snapshot.recording).toBe(true);

        self.pressRecord();
        await vi.waitFor(() => expect(self.state.snapshot.recording).toBe(false));
        expect(invokeCalls().some((c) => c.cmd === "stop_recording")).toBe(true);
    });
});
