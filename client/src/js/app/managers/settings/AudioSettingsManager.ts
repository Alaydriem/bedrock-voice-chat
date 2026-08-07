import { writable, type Readable, type Writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { Store } from "@tauri-apps/plugin-store";
import Analytics from "../../analytics";
import PlatformDetector from "../../utils/PlatformDetector";
import type { KeybindConfig } from "../../../bindings/KeybindConfig";
import type { VoiceMode } from "../../../bindings/VoiceMode";
import type { VoiceRuntimeState } from "../../../bindings/VoiceRuntimeState";
import { AppStore } from "../../services/AppStore";

export class AudioSettingsManager {
    private readonly platformDetector: PlatformDetector;

    private storeStore: Writable<Store | undefined>;
    public readonly store: Readable<Store | undefined>;
    private isReadyStore: Writable<boolean>;
    public readonly isReady: Readable<boolean>;
    private isMobileStore: Writable<boolean>;
    public readonly isMobile: Readable<boolean>;
    private voiceModeStore: Writable<VoiceMode>;
    public readonly voiceMode: Readable<VoiceMode>;
    private panningIntensityStore: Writable<number>;
    public readonly panningIntensity: Readable<number>;
    private voiceModeErrorStore: Writable<string>;
    /** Why the last mode change did not take, or empty. */
    public readonly voiceModeError: Readable<string>;

    constructor() {
        this.platformDetector = new PlatformDetector();

        this.storeStore = writable(undefined);
        this.store = { subscribe: this.storeStore.subscribe };
        this.isReadyStore = writable(false);
        this.isReady = { subscribe: this.isReadyStore.subscribe };
        this.isMobileStore = writable(false);
        this.isMobile = { subscribe: this.isMobileStore.subscribe };
        this.voiceModeStore = writable("openMic");
        this.voiceMode = { subscribe: this.voiceModeStore.subscribe };
        this.panningIntensityStore = writable(80);
        this.panningIntensity = { subscribe: this.panningIntensityStore.subscribe };
        this.voiceModeErrorStore = writable("");
        this.voiceModeError = { subscribe: this.voiceModeErrorStore.subscribe };
    }

    async initialize(): Promise<void> {
        const store = await AppStore.load();
        this.storeStore.set(store);

        this.isMobileStore.set(await this.platformDetector.checkMobile());

        const savedPanning = await store.get<number>("panning_intensity");
        if (savedPanning !== null && savedPanning !== undefined) {
            this.panningIntensityStore.set(Math.round(savedPanning * 100));
        }

        const saved = await store.get<KeybindConfig>("keybinds");
        if (saved?.voiceMode) {
            this.voiceModeStore.set(saved.voiceMode);
        }

        this.isReadyStore.set(true);
    }

    async handlePanningIntensityChange(value: number): Promise<void> {
        this.panningIntensityStore.set(value);

        let store: Store | undefined;
        this.storeStore.update((current) => {
            store = current;
            return current;
        });
        if (!store) return;

        const normalized = value / 100;
        await store.set("panning_intensity", normalized);
        await store.save();
        await invoke("update_stream_metadata", {
            key: "panning_intensity",
            value: normalized.toString(),
            device: "OutputDevice",
        });
    }

    /**
     * Change the voice mode, and show what the backend actually reached.
     *
     * The control used to move first and hope. When the command failed — or was dropped
     * because the store had not finished loading — the segmented control sat on a mode the
     * backend was not in, and the mic button, which reads the backend, kept behaving as the
     * old one. So nothing here moves until the backend answers.
     */
    async handleVoiceModeChange(mode: VoiceMode): Promise<void> {
        this.voiceModeErrorStore.set("");
        try {
            // Loaded on demand rather than skipped. `initialize` is not awaited by the pane,
            // so a change made early used to be discarded without a word.
            const store = await this.requireStore();
            const saved = await store.get<KeybindConfig>("keybinds");
            const config: KeybindConfig = {
                toggleMute: saved?.toggleMute ?? "ControlLeft+BracketLeft",
                toggleDeafen: saved?.toggleDeafen ?? "ControlLeft+BracketRight",
                toggleRecording: saved?.toggleRecording ?? "ControlLeft+Backslash",
                pushToTalk: saved?.pushToTalk ?? "Backquote",
                voiceMode: mode,
            };
            await store.set("keybinds", config);
            await store.save();

            const reached = await invoke<VoiceRuntimeState>("start_keybind_listener", { config });
            this.voiceModeStore.set(reached.voiceMode);
            if (reached.voiceMode !== mode) {
                this.voiceModeErrorStore.set(
                    `The app is still in ${reached.voiceMode === "pushToTalk" ? "push-to-talk" : "open mic"}.`,
                );
                return;
            }
            Analytics.track("VoiceModeChanged", { mode });
        } catch (e) {
            this.voiceModeErrorStore.set(String(e));
        }
    }

    /** The store, loading it if `initialize` has not finished. */
    private async requireStore(): Promise<Store> {
        let current: Store | undefined;
        this.storeStore.update((value) => {
            current = value;
            return value;
        });
        if (current) return current;

        const store = await AppStore.load();
        this.storeStore.set(store);
        return store;
    }
}
