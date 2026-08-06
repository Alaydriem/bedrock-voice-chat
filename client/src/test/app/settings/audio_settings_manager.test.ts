import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../tauri";

const saved: Record<string, unknown> = {};
let loaded = 0;

vi.mock("@tauri-apps/plugin-store", () => ({
    Store: {
        load: async () => {
            loaded += 1;
            return {
                get: async (key: string) => saved[key],
                set: async (key: string, value: unknown) => void (saved[key] = value),
                save: async () => {},
            };
        },
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

function backend(voiceMode: "openMic" | "pushToTalk") {
    return {
        voiceMode,
        pttActive: false,
        inputMuted: voiceMode === "pushToTalk",
        outputMuted: false,
    };
}

beforeEach(() => {
    for (const key of Object.keys(saved)) delete saved[key];
    loaded = 0;
});

describe("changing the voice mode", () => {
    it("persists the mode and applies it to the backend", async () => {
        mockInvoke({ start_keybind_listener: () => backend("pushToTalk") });
        const audio = new AudioSettingsManager();
        await audio.initialize();

        await audio.handleVoiceModeChange("pushToTalk");

        const call = invokeCalls().find((c) => c.cmd === "start_keybind_listener");
        expect((call?.args as { config: { voiceMode: string } }).config.voiceMode).toBe(
            "pushToTalk",
        );
        expect((saved.keybinds as { voiceMode: string }).voiceMode).toBe("pushToTalk");
        expect(read(audio.voiceMode)).toBe("pushToTalk");
    });

    /**
     * The pane does not await `initialize`, so a change made in the first moments used to
     * hit an unloaded store and return — moving the control and doing nothing else.
     */
    it("applies a change made before initialize has finished", async () => {
        mockInvoke({ start_keybind_listener: () => backend("pushToTalk") });
        const audio = new AudioSettingsManager();

        await audio.handleVoiceModeChange("pushToTalk");

        expect(invokeCalls().some((c) => c.cmd === "start_keybind_listener")).toBe(true);
        expect(read(audio.voiceMode)).toBe("pushToTalk");
    });

    it("switches back to open mic", async () => {
        mockInvoke({ start_keybind_listener: () => backend("openMic") });
        const audio = new AudioSettingsManager();
        await audio.initialize();

        await audio.handleVoiceModeChange("openMic");

        expect(read(audio.voiceMode)).toBe("openMic");
        expect(read(audio.voiceModeError)).toBe("");
    });

    /**
     * The control shows the backend, never the intent.
     *
     * Moving first and hoping is what left the settings screen reading "voice activated"
     * over a backend still in push-to-talk — with the mic button, which reads the backend,
     * behaving like the mode the screen said was off.
     */
    it("does not show a mode the backend did not reach", async () => {
        mockInvoke({ start_keybind_listener: () => backend("pushToTalk") });
        const audio = new AudioSettingsManager();
        await audio.initialize();

        await audio.handleVoiceModeChange("openMic");

        expect(read(audio.voiceMode)).toBe("pushToTalk");
        expect(read(audio.voiceModeError)).toContain("push-to-talk");
    });

    it("reports a command that failed rather than swallowing it", async () => {
        mockInvoke({
            start_keybind_listener: () => {
                throw new Error("no listener");
            },
        });
        const audio = new AudioSettingsManager();
        await audio.initialize();

        await audio.handleVoiceModeChange("pushToTalk");

        expect(read(audio.voiceModeError)).toContain("no listener");
    });
});
