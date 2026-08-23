import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke } from "../../../tauri";

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
    mockInvoke({});
});

describe("mute cues", () => {
    /**
     * Every install that predates the feature has no key. Reading an absent key as off would
     * ship a default-on feature switched off for the entire existing user base, and nothing
     * about that failure is visible — the app is simply quiet.
     */
    it("is on when the setting has never been touched", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        expect(get(audio.muteCues)).toBe(true);
    });

    it("stays off once it has been turned off", async () => {
        saved = { mute_cues_enabled: false };

        const audio = new AudioSettingsManager();
        await audio.initialize();

        expect(get(audio.muteCues)).toBe(false);
    });

    it("writes the choice so the backend reads the same answer", async () => {
        const audio = new AudioSettingsManager();
        await audio.initialize();

        await audio.handleMuteCuesChange(false);

        expect(saved.mute_cues_enabled).toBe(false);
        expect(get(audio.muteCues)).toBe(false);
    });
});
