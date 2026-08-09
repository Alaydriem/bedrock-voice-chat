import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../tauri";

const saved: Record<string, unknown> = {};

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
    mockInvoke({ update_stream_metadata: () => null });
});

describe("jukebox volume", () => {
    it("starts untouched", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        expect(read(audio.jukeboxGain)).toBe(100);
        expect(read(audio.jukeboxMuted)).toBe(false);
    });

    it("persists a fraction and sends it to the output stream", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        await audio.handleJukeboxGainChange(60);

        expect(saved.jukebox_gain).toBe(0.6);
        expect(read(audio.jukeboxGain)).toBe(60);
        expect(metadata("jukebox_gain")?.value).toBe("0.6");
        expect(metadata("jukebox_gain")?.device).toBe("OutputDevice");
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

        expect(metadata("jukebox_gain")?.value).toBe("0.2");
        expect(saved.jukebox_gain).toBe(0.2);
    });
});

describe("jukebox mute", () => {
    it("persists the flag and sends it to the output stream", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        await audio.handleJukeboxMutedChange(true);

        expect(saved.jukebox_muted).toBe(true);
        expect(read(audio.jukeboxMuted)).toBe(true);
        expect(metadata("jukebox_muted")?.value).toBe("true");
    });

    // The two are separate controls, so unmuting has to come back to the level that was set.
    it("leaves the level alone", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();
        await audio.handleJukeboxGainChange(40);

        await audio.handleJukeboxMutedChange(true);
        await audio.handleJukeboxMutedChange(false);

        expect(read(audio.jukeboxGain)).toBe(40);
        expect(saved.jukebox_gain).toBe(0.4);
    });

    it("restores a saved mute", async () => {
        saved.jukebox_muted = true;
        const audio = new AudioSettingsManager();

        await audio.initialize();

        expect(read(audio.jukeboxMuted)).toBe(true);
    });
});
