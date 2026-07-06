import { writable, type Readable, type Writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { Store } from "@tauri-apps/plugin-store";
import Analytics from "../../analytics";
import PlatformDetector from "../../utils/PlatformDetector";
import type { KeybindConfig } from "../../../bindings/KeybindConfig";
import type { VoiceMode } from "../../../bindings/VoiceMode";

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
    }

    async initialize(): Promise<void> {
        const store = await Store.load("store.json", {
            autoSave: false,
            defaults: {},
        });
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

    async handleVoiceModeChange(mode: VoiceMode): Promise<void> {
        this.voiceModeStore.set(mode);

        let store: Store | undefined;
        this.storeStore.update((current) => {
            store = current;
            return current;
        });
        if (!store) return;

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
        await invoke("start_keybind_listener", { config });
        Analytics.track("VoiceModeChanged", { mode });
    }
}
