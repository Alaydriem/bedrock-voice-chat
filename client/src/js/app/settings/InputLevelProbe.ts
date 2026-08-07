import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { warn } from "@tauri-apps/plugin-log";
import { type Readable, type Writable, writable } from "svelte/store";
import type { InputLevel } from "../../bindings/InputLevel";
import type { LevelSnapshot } from "../../bindings/LevelSnapshot";
import { LevelSteps } from "../dashboard/LevelSteps";

/**
 * The microphone level for a screen that may or may not have a session behind it.
 *
 * `start_input_meter` is not free: it runs `AudioStreamManager::init`, which stops and replaces
 * the input stream — and the replacement meters without transmitting. Starting one over a live
 * session takes the microphone off the air, so whether a session is already capturing has to be
 * answered correctly every time.
 *
 * It is now *asked*, not inferred. This waited a second and a half to see whether level events
 * arrived and claimed a stream when none did, which was sound only while levels were published
 * on a fixed clock. They are now published on change, so a quiet room produces no events at all
 * and looked exactly like a dead capture — opening the audio pane in silence tore down a
 * working session every time.
 *
 * A live session's level comes from `audio-levels`, the same event the dashboard meter uses.
 * `audio-input-level` is emitted only by a stream this probe started, and carries the
 * unquantised amplitude that calibration wants.
 */
export class InputLevelProbe {
    private readonly rmsStore: Writable<number>;
    private readonly gateOpenStore: Writable<boolean>;
    private readonly availableStore: Writable<boolean>;

    /** Post-gate RMS, 0 to 1. */
    public readonly rms: Readable<number>;
    public readonly gateOpen: Readable<boolean>;
    /** False only when this probe started a stream and the backend refused. */
    public readonly available: Readable<boolean>;

    private unlisteners: UnlistenFn[] = [];
    private running = false;
    /** Whether the stream being metered is ours to stop. */
    private owned = false;

    constructor() {
        this.rmsStore = writable(0);
        this.gateOpenStore = writable(false);
        this.availableStore = writable(true);
        this.rms = { subscribe: this.rmsStore.subscribe };
        this.gateOpen = { subscribe: this.gateOpenStore.subscribe };
        this.available = { subscribe: this.availableStore.subscribe };
    }

    async start(): Promise<void> {
        if (this.running) return;
        this.running = true;

        try {
            this.unlisteners.push(
                await listen<LevelSnapshot>("audio-levels", (event) => {
                    this.rmsStore.set(LevelSteps.toLevel(event.payload.own));
                    this.gateOpenStore.set(event.payload.own.speaking);
                }),
            );
            this.unlisteners.push(
                await listen<InputLevel>("audio-input-level", (event) => {
                    this.rmsStore.set(event.payload.rms);
                    this.gateOpenStore.set(event.payload.gate_open);
                }),
            );
        } catch (e) {
            // Nothing can arrive without the subscription, so claiming a stream would only
            // start a capture nobody reads. Reported as unreadable, which is what it is —
            // and not raised, because the rest of the pane still works.
            this.availableStore.set(false);
            await warn(`Could not subscribe to the input level: ${e}`);
            return;
        }

        if (await this.sessionIsCapturing()) return;
        await this.claim();
    }

    /**
     * Whether something is already capturing, asked of the backend.
     *
     * A failure answers "yes" rather than "no". Being wrong in that direction shows an empty
     * meter on a screen that has other things on it; being wrong the other way takes a working
     * microphone off the air.
     */
    private async sessionIsCapturing(): Promise<boolean> {
        try {
            return await invoke<boolean>("input_capture_active");
        } catch (e) {
            await warn(`Could not ask whether a capture is running: ${e}`);
            return true;
        }
    }

    /**
     * Start a capture-only stream, because nothing else is capturing.
     *
     * Reached before a session exists — the settings screen opened from the server list, or a
     * pane visited before connecting. The failure is reported rather than raised: a meter that
     * could not start looks exactly like a microphone picking up nothing, and telling those
     * two apart is what the meter is for.
     */
    private async claim(): Promise<void> {
        if (!this.running) return;
        try {
            await invoke("start_input_meter");
            this.owned = true;
        } catch (e) {
            this.availableStore.set(false);
            await warn(`Could not start the input meter: ${e}`);
        }
    }

    async stop(): Promise<void> {
        if (!this.running) return;
        this.running = false;

        if (this.owned) {
            this.owned = false;
            try {
                await invoke("stop_input_meter");
            } catch (e) {
                await warn(`Could not stop the input meter: ${e}`);
            }
        }

        for (const off of this.unlisteners) off();
        this.unlisteners = [];
        this.rmsStore.set(0);
        this.gateOpenStore.set(false);
        this.availableStore.set(true);
    }
}
