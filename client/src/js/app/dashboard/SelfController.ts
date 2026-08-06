import { invoke } from '@tauri-apps/api/core';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { info, warn } from '@tauri-apps/plugin-log';
import type { Store } from '@tauri-apps/plugin-store';
import { writable, type Readable, type Writable } from 'svelte/store';
import { SelfState } from '$radial/core/controllers/SelfState';
import type { KeybindConfig } from '../../bindings/KeybindConfig';
import type { VoiceMode as ConfiguredVoiceMode } from '../../bindings/VoiceMode';
import type { VoiceRuntimeState } from '../../bindings/VoiceRuntimeState';
import { MicLevelSource, type MicActivity } from './MicLevelSource';

/**
 * Mute, deafen, record and push-to-talk, against the real backend.
 *
 * Every press is a command followed by a wait: the backend answers with `mute:input` or
 * `mute:output` carrying the state it actually reached, and only that moves `SelfState`.
 * Nothing here is optimistic, because these four are not this window's to own — a global
 * hotkey works while Minecraft has focus, an in-game command works from chat, and both
 * change the same flags underneath. A UI that assumed its own press had landed would drift
 * the first time either fired, and stay wrong.
 */
/** The backend's own answer plus proof of life from the capture stream. */
export interface VoiceDiagnostics {
    readonly backend: VoiceRuntimeState | null;
    readonly mic: MicActivity;
    readonly error?: string;
}

export class SelfController {
    readonly state = new SelfState();

    /** How often the diagnostics readout re-asks the backend what it believes. */
    private static readonly PROBE_MS = 1000;

    private readonly mic = new MicLevelSource();
    private readonly store: Store;
    private unlisteners: UnlistenFn[] = [];
    private readonly diagnosticsStore: Writable<VoiceDiagnostics | null> = writable(null);
    private probe: ReturnType<typeof setInterval> | null = null;

    constructor(store: Store) {
        this.store = store;
    }

    /** Your microphone, for the pill's meter. */
    get micSource() {
        return this.mic.source;
    }

    /** What the backend believes, for the status panel. */
    get diagnostics(): Readable<VoiceDiagnostics | null> {
        return { subscribe: this.diagnosticsStore.subscribe };
    }

    /**
     * Ask the backend what it believes, and adopt it.
     *
     * The voice mode and the mute flag are not this window's to own, and the event that was
     * supposed to carry a mode change did not arrive on Android — leaving the mic button
     * offering a toggle for a mode where holding is the only thing that transmits, and that
     * toggle then opening the microphone for real.
     *
     * So the mode is read rather than awaited. The events below still arrive first when they
     * arrive at all, which is what keeps a press feeling immediate; this is what makes the
     * button correct within a second whether or not they do.
     */
    /** Read the backend now rather than waiting for the next tick. */
    async refresh(): Promise<void> {
        await this.pollBackend();
    }

    private async pollBackend(): Promise<void> {
        try {
            const backend = await invoke<VoiceRuntimeState>('voice_runtime_state');
            this.diagnosticsStore.set({ backend, mic: this.mic.activity });
            this.state.sync({
                mode: backend.voiceMode === 'pushToTalk' ? 'ptt' : 'activated',
                muted: backend.inputMuted,
                deafened: backend.outputMuted,
            });
            // Reconciles an optimistic hold the backend refused, and a hold released by a
            // gesture whose pointerup never landed.
            this.state.hold(backend.pttActive);
        } catch (e) {
            this.diagnosticsStore.set({ backend: null, mic: this.mic.activity, error: String(e) });
        }
    }

    async start(): Promise<void> {
        await this.seed();
        await this.subscribe();
        await this.mic.start();
        await this.pollBackend();
        this.probe = setInterval(() => void this.pollBackend(), SelfController.PROBE_MS);
    }

    /**
     * First paint from the backend rather than from defaults.
     *
     * A mute survives a webview reload, so a pill that starts unmuted is lying until the
     * user presses something — and what they press will be the wrong thing.
     */
    private async seed(): Promise<void> {
        try {
            const [muted, deafened, recording] = await Promise.all([
                invoke<boolean>('mute_status', { device: 'InputDevice' }),
                invoke<boolean>('mute_status', { device: 'OutputDevice' }),
                invoke<boolean>('is_recording'),
            ]);
            const keybinds = await this.store.get<KeybindConfig>('keybinds');
            this.state.sync({
                muted,
                deafened,
                recording,
                mode: keybinds?.voiceMode === 'pushToTalk' ? 'ptt' : 'activated',
            });
        } catch (e) {
            warn(`SelfController: could not read self state, using defaults: ${e}`);
        }
    }

    private async subscribe(): Promise<void> {
        const webview = getCurrentWebviewWindow();
        const listen = async <T>(event: string, run: (payload: T) => void) => {
            try {
                this.unlisteners.push(await webview.listen<T>(event, (e) => run(e.payload)));
            } catch (e) {
                warn(`SelfController: could not listen for ${event}: ${e}`);
            }
        };

        // Logged at info because this is the half of the round trip a device log cannot
        // otherwise see: the command's own logging proves the press reached Rust, and this
        // proves the answer came back. A press that logs one and not the other localises the
        // fault immediately.
        await listen<boolean>('mute:input', (muted) => {
            info(`SelfController: mute:input echo -> ${muted}`);
            this.state.sync({ muted });
        });
        await listen<boolean>('mute:output', (deafened) => {
            info(`SelfController: mute:output echo -> ${deafened}`);
            this.state.sync({ deafened });
        });
        await listen<boolean>('ptt:active', (down) => this.state.hold(down));
        // Settings, a Stream Deck and a hotkey all write the same setting, and the mic
        // button is a hold in one mode and a toggle in the other. Read once at start-up it
        // goes stale the first time anything changes it.
        await listen<ConfiguredVoiceMode>('voice-mode:changed', (mode) => {
            info(`SelfController: voice mode -> ${mode}`);
            this.state.sync({ mode: mode === 'pushToTalk' ? 'ptt' : 'activated' });
        });
        await listen<unknown>('recording:started', () =>
            this.state.sync({ recording: true }, performance.now()),
        );
        await listen<unknown>('recording:stopped', () => this.state.sync({ recording: false }));
    }

    /**
     * The mic button.
     *
     * In push-to-talk it is a hold control and a press means nothing. Otherwise it aims at
     * the state the kit's invariants describe: someone deafened who presses the mic wants
     * back into the conversation, so that press clears both flags rather than leaving them
     * deafened and wondering which other button to find.
     */
    pressMute(): void {
        const self = this.state.snapshot;
        if (self.mode === 'ptt') return;
        if (self.deafened) {
            this.state.sync({ muted: false, deafened: false });
            void this.settle('set_deafened', { deafened: false }, (reached) => ({
                deafened: reached,
                muted: reached,
            }));
        } else {
            const muted = !self.muted;
            this.state.sync({ muted });
            void this.settle('set_mute', { device: 'InputDevice', muted }, (reached) => ({
                muted: reached,
            }));
        }
    }

    pressDeafen(): void {
        const deafened = !this.state.snapshot.deafened;
        this.state.sync({ deafened, muted: deafened });
        void this.settle('set_deafened', { deafened }, (reached) => ({
            deafened: reached,
            muted: reached,
        }));
    }

    pressRecord(): void {
        void this.send(this.state.snapshot.recording ? 'stop_recording' : 'start_recording', {});
    }

    /**
     * Paint now, settle on what the backend reached.
     *
     * Waiting for the round trip before moving the button was correct about authority and wrong
     * about feel: on a phone the IPC hop plus the audio thread's acknowledgement is long enough
     * to read as a broken control, and the user presses again.
     *
     * The backend is still the authority — it just gets to correct the button instead of being
     * asked first. Reconciling on the command's own return value rather than on the broadcast
     * echo is what makes that safe: the return is correlated with this press, so two rapid
     * presses settle in the order they were sent. The echo listeners stay for the changes this
     * window did not initiate — a global hotkey, an in-game command — where there is no return
     * value to wait for.
     */
    private async settle(
        command: string,
        args: Record<string, unknown>,
        apply: (reached: boolean) => Parameters<SelfState['sync']>[0],
    ): Promise<void> {
        try {
            info(`SelfController: invoking ${command} ${JSON.stringify(args)}`);
            const reached = await invoke<boolean>(command, args);
            info(`SelfController: ${command} reached ${reached}`);
            this.state.sync(apply(reached));
        } catch (e) {
            warn(`SelfController: ${command} failed: ${e}`);
            // The optimistic paint is now a lie, and nothing else will correct it.
            await this.seed();
        }
    }

    /**
     * Push-to-talk from the on-screen button.
     *
     * The backend owns the microphone, so this asks rather than paints: `set_ptt` opens the
     * input and echoes `ptt:active`, which is the same path the global hotkey takes. Moving
     * `SelfState` here instead would light the meter over a mic that is still muted — which
     * is what a phone had, since a phone has no hotkey and this button is the only way in.
     */
    hold(down: boolean): void {
        // Painted immediately: a hold that waits for a round trip before the meter moves
        // reads as a button that did not take. The poll corrects it if the backend refused.
        this.state.hold(down);
        void this.send('set_ptt', { down });
    }

    /** mm:ss since recording was armed. */
    elapsed(now: number): string {
        const seconds = Math.floor(this.state.elapsed(now) / 1000);
        const mm = String(Math.floor(seconds / 60)).padStart(2, '0');
        const ss = String(seconds % 60).padStart(2, '0');
        return `${mm}:${ss}`;
    }

    private async send(command: string, args: Record<string, unknown>): Promise<void> {
        try {
            info(`SelfController: invoking ${command} ${JSON.stringify(args)}`);
            await invoke(command, args);
        } catch (e) {
            warn(`SelfController: ${command} failed: ${e}`);
        }
    }

    cleanup(): void {
        if (this.probe) {
            clearInterval(this.probe);
            this.probe = null;
        }
        for (const off of this.unlisteners) off();
        this.unlisteners = [];
        this.mic.stop();
    }
}
