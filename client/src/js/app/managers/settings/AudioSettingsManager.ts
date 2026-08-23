import { writable, type Readable, type Writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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
    private jukeboxGainStore: Writable<number>;
    /** Percent, 0–150. The store holds the fraction; this is what a slider shows. */
    public readonly jukeboxGain: Readable<number>;
    private jukeboxMutedStore: Writable<boolean>;
    public readonly jukeboxMuted: Readable<boolean>;
    private muteCuesStore: Writable<boolean>;
    /** Whether mute and deafen announce themselves with a tone. */
    public readonly muteCues: Readable<boolean>;
    private voiceModeErrorStore: Writable<string>;
    /** Why the last mode change did not take, or empty. */
    public readonly voiceModeError: Readable<string>;
    private unlistenJukeboxMuted: UnlistenFn | null = null;
    private unlistenJukeboxGain: UnlistenFn | null = null;

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
        this.jukeboxGainStore = writable(100);
        this.jukeboxGain = { subscribe: this.jukeboxGainStore.subscribe };
        this.jukeboxMutedStore = writable(false);
        this.jukeboxMuted = { subscribe: this.jukeboxMutedStore.subscribe };
        this.muteCuesStore = writable(true);
        this.muteCues = { subscribe: this.muteCuesStore.subscribe };
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

        const savedJukeboxGain = await store.get<number>("jukebox_gain");
        if (savedJukeboxGain !== null && savedJukeboxGain !== undefined) {
            this.jukeboxGainStore.set(Math.round(savedJukeboxGain * 100));
        }

        const savedJukeboxMuted = await store.get<boolean>("jukebox_muted");
        if (savedJukeboxMuted !== null && savedJukeboxMuted !== undefined) {
            this.jukeboxMutedStore.set(savedJukeboxMuted);
        }

        // An absent key is on, not off. Nothing writes this key until the user touches the
        // switch, so every install that predates the feature arrives here with null.
        const savedMuteCues = await store.get<boolean>("mute_cues_enabled");
        this.muteCuesStore.set(savedMuteCues ?? true);

        // A WebSocket controller or the in-game panel changes this without the pane being asked,
        // and the switch reads this store. Without the listener it keeps drawing the pre-change
        // state for as long as it stays mounted.
        this.unlistenJukeboxMuted = await listen<boolean>("jukebox_muted_updated", (event) =>
            this.jukeboxMutedStore.set(event.payload),
        );

        // The same for the level, which the slider reads. The payload is the fraction the backend
        // applied, which is the requested value clamped, so this also corrects an out-of-range ask.
        this.unlistenJukeboxGain = await listen<number>("jukebox_gain_updated", (event) =>
            this.jukeboxGainStore.set(Math.round(event.payload * 100)),
        );

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
     * Set how loud jukebox music plays.
     *
     * The backend owns all three copies of this level — the mixing-path atomic, the stream
     * metadata a rebuild restores from, and `store.json` — and emits the event that moves this
     * store. Writing any of them from here as well is how they drift, so this only asks.
     *
     * The store is still set optimistically, like the mute switch: the slider has to move under
     * the finger rather than a round trip later, and the event reconciles it either way.
     *
     * The mute flag is deliberately untouched. They are separate controls on every surface, so a
     * level set while muted is the level that comes back on unmute.
     */
    async handleJukeboxGainChange(percent: number): Promise<void> {
        this.jukeboxGainStore.set(percent);
        await invoke("set_jukebox_gain", { gain: percent / 100 });
    }

    /**
     * Set whether jukebox music plays.
     *
     * The backend owns all three copies of this flag — the mixing-path atomic, the stream metadata
     * a rebuild restores from, and `store.json` — and emits the event that moves this store.
     * Writing any of them from here as well is how they drift, so this only asks.
     *
     * The store is still set optimistically: the switch has to move under the finger rather than
     * a round trip later, and the event reconciles it either way.
     */
    async handleJukeboxMutedChange(muted: boolean): Promise<void> {
        this.jukeboxMutedStore.set(muted);
        await invoke("set_jukebox_muted", { muted });
    }

    /**
     * Set whether mute and deafen announce themselves.
     *
     * Written here rather than asked of the backend, unlike the jukebox controls. The backend
     * reads this key from the store at the moment it plays, so the store is the only copy and
     * there is nothing for a round trip to reconcile.
     */
    async handleMuteCuesChange(next: boolean): Promise<void> {
        this.muteCuesStore.set(next);

        let store: Store | undefined;
        this.storeStore.update((current) => {
            store = current;
            return current;
        });
        if (!store) return;

        await store.set("mute_cues_enabled", next);
        await store.save();
    }

    /** Releases the event listeners. A listener outliving the manager writes to a dead store. */
    cleanup(): void {
        this.unlistenJukeboxMuted?.();
        this.unlistenJukeboxMuted = null;
        this.unlistenJukeboxGain?.();
        this.unlistenJukeboxGain = null;
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
