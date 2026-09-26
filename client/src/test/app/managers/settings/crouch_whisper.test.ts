import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../../tauri";

let saved: Record<string, unknown> = {};

vi.mock("@tauri-apps/plugin-store", () => ({
    Store: {
        load: async () => ({
            get: async (key: string) => saved[key] ?? null,
            set: async (key: string, value: unknown) => {
                saved[key] = value;
            },
            save: async () => {},
        }),
    },
}));

const { AudioSettingsManager } = await import(
    "../../../../js/app/managers/settings/AudioSettingsManager"
);

beforeEach(() => {
    saved = {};
    mockInvoke({ set_crouch_whisper: (args: { enabled: boolean }) => args.enabled });
});

describe("crouch to whisper", () => {
    /**
     * Opt-in. Every install arrives with no key, and reading that as on would shrink the range
     * of every player who crouches, which is the behaviour this setting exists to make optional.
     */
    it("is off when the setting has never been touched", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        expect(get(audio.crouchWhisper)).toBe(false);
    });

    it("shows a choice that was already made", async () => {
        saved = { crouch_whisper_enabled: true };

        const audio = new AudioSettingsManager();
        await audio.initialize();

        expect(get(audio.crouchWhisper)).toBe(true);
    });

    /** The backend writes the store and tells the server; the pane only asks. */
    it("asks the backend rather than writing the store itself", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        await audio.handleCrouchWhisperChange(true);

        expect(invokeCalls()).toContainEqual({
            cmd: "set_crouch_whisper",
            args: { enabled: true },
        });
        expect(saved.crouch_whisper_enabled).toBeUndefined();
        expect(get(audio.crouchWhisper)).toBe(true);
    });

    /** Nothing reached the server, so the switch must not claim the choice was made. */
    it("returns to the saved choice when the backend refuses", async () => {
        saved = { crouch_whisper_enabled: false };
        mockInvoke({
            set_crouch_whisper: () => {
                throw new Error("app state is not ready");
            },
        });
        const audio = new AudioSettingsManager();
        await audio.initialize();

        await audio.handleCrouchWhisperChange(true);

        expect(get(audio.crouchWhisper)).toBe(false);
    });
});
