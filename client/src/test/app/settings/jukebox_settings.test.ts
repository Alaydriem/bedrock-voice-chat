import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../tauri";

const saved: Record<string, unknown> = {};

/**
 * Captures the event handlers the manager registers, so a backend-side change can be replayed.
 *
 * The shared helper mocks `listen` with a no-op, which cannot show whether the manager reacts to
 * anything. Overriding it here is the only way to prove the reconciliation works.
 */
const handlers: Record<string, (event: { payload: unknown }) => void> = {};
let unlistenCalls = 0;

vi.mock("@tauri-apps/api/event", () => ({
    listen: async (name: string, handler: (event: { payload: unknown }) => void) => {
        handlers[name] = handler;
        return () => void unlistenCalls++;
    },
}));

vi.mock("@tauri-apps/plugin-store", () => ({
    Store: {
        load: async () => ({
            get: async (key: string) => saved[key],
            set: async (key: string, value: unknown) => void (saved[key] = value),
            save: async () => {},
        }),
    },
}));

const { AudioSettingsManager } = await import(
    "../../../js/app/managers/settings/AudioSettingsManager"
);

function read<T>(store: { subscribe: (run: (v: T) => void) => () => void }): T {
    let value!: T;
    store.subscribe((v) => (value = v))();
    return value;
}

function metadata(key: string) {
    return invokeCalls().find(
        (c) => c.cmd === "update_stream_metadata" && (c.args as { key: string }).key === key,
    )?.args as { key: string; value: string; device: string } | undefined;
}

beforeEach(() => {
    for (const key of Object.keys(saved)) delete saved[key];
    for (const key of Object.keys(handlers)) delete handlers[key];
    unlistenCalls = 0;
    mockInvoke({
        update_stream_metadata: () => null,
        set_jukebox_muted: () => true,
        set_jukebox_gain: () => 0.6,
    });
});

describe("jukebox volume", () => {
    it("starts untouched", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        expect(read(audio.jukeboxGain)).toBe(100);
        expect(read(audio.jukeboxMuted)).toBe(false);
    });

    it("asks the backend to set the level rather than writing it", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        await audio.handleJukeboxGainChange(60);

        const call = invokeCalls().find((c) => c.cmd === "set_jukebox_gain");
        expect(call?.args).toEqual({ gain: 0.6 });
        expect(read(audio.jukeboxGain)).toBe(60);
    });

    /**
     * The stream metadata and `store.json` are two of the backend's three copies of this level.
     * A write from here is how they drift out of step with the third, the mixing-path atomic.
     */
    it("does not write the stream metadata itself", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        await audio.handleJukeboxGainChange(60);

        expect(metadata("jukebox_gain")).toBeUndefined();
    });

    /**
     * A WebSocket controller and the in-game panel both change this without the pane being asked.
     * The event is the only thing that corrects the slider while it stays mounted.
     */
    it("follows a level change this window did not make", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        handlers["jukebox_gain_updated"]?.({ payload: 0.4 });

        expect(read(audio.jukeboxGain)).toBe(40);
    });

    it("restores a saved level", async () => {
        saved.jukebox_gain = 0.35;
        const audio = new AudioSettingsManager();

        await audio.initialize();

        expect(read(audio.jukeboxGain)).toBe(35);
    });

    /**
     * The pane does not await `initialize`, so a change made in the first moments must still
     * reach the backend rather than moving the control and doing nothing else.
     */
    it("applies a change made before initialize has finished", async () => {
        const audio = new AudioSettingsManager();

        await audio.handleJukeboxGainChange(20);

        const call = invokeCalls().find((c) => c.cmd === "set_jukebox_gain");
        expect(call?.args).toEqual({ gain: 0.2 });
    });
});

describe("jukebox mute", () => {
    /**
     * The backend owns the flag. Three copies of it have to move together — the mixing-path
     * atomic, the stream metadata a rebuild restores from, and `store.json` — and a WebSocket
     * controller and the in-game panel change it too. Writing any of them from here as well is
     * how they drift, so the pane asks and nothing more.
     */
    it("asks the backend to set the flag rather than writing it", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        await audio.handleJukeboxMutedChange(true);

        const call = invokeCalls().find((c) => c.cmd === "set_jukebox_muted");
        expect(call?.args).toEqual({ muted: true });
        expect(metadata("jukebox_muted")).toBeUndefined();
        expect(saved.jukebox_muted).toBeUndefined();
    });

    // The switch has to move under the finger rather than a round trip later. The event the
    // backend emits reconciles it either way.
    it("moves the switch without waiting for the backend", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        await audio.handleJukeboxMutedChange(true);

        expect(read(audio.jukeboxMuted)).toBe(true);
    });

    // The two are separate controls, so unmuting has to come back to the level that was set.
    it("leaves the level alone", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();
        await audio.handleJukeboxGainChange(40);

        await audio.handleJukeboxMutedChange(true);
        await audio.handleJukeboxMutedChange(false);

        expect(read(audio.jukeboxGain)).toBe(40);
        // The backend persists the level now, so the proof the mute path left it alone is that
        // muting never asked for a level at all — only the one deliberate change did.
        expect(invokeCalls().filter((c) => c.cmd === "set_jukebox_gain")).toHaveLength(1);
    });

    /**
     * A WebSocket controller and the in-game panel both change this without the pane being asked.
     * The event is the only thing that corrects the switch while it stays mounted, which is the
     * whole reason it exists.
     */
    it("follows a change this window did not make", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        handlers["jukebox_muted_updated"]?.({ payload: true });

        expect(read(audio.jukeboxMuted)).toBe(true);
    });

    // A listener outliving the manager keeps writing to a store nothing reads. Both of them —
    // the level and the mute flag each have one.
    it("stops listening once cleaned up", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        audio.cleanup();

        expect(unlistenCalls).toBe(2);
    });

    it("restores a saved mute", async () => {
        saved.jukebox_muted = true;
        const audio = new AudioSettingsManager();

        await audio.initialize();

        expect(read(audio.jukeboxMuted)).toBe(true);
    });
});
