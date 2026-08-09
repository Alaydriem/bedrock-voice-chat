import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { info, warn } from '@tauri-apps/plugin-log';
import type { Store } from '@tauri-apps/plugin-store';
import { writable, type Readable, type Writable } from 'svelte/store';
import { MeterProbe } from '$radial/core/canvas/MeterProbe';
import { SelfState } from '$radial/core/controllers/SelfState';
import type { KeybindConfig } from '../../bindings/KeybindConfig';
import type { VoiceMode as ConfiguredVoiceMode } from '../../bindings/VoiceMode';
import type { VoiceRuntimeState } from '../../bindings/VoiceRuntimeState';
import type { MicActivity, PlayerLevelSources } from './PlayerLevelSources';

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

    /**
     * When the one-shot meter verdict is logged, measured from the first `start`.
     *
     * Late enough that a session has connected and somebody has usually spoken; early enough
     * that ten launches in a row cost ten short waits. One line per launch is what makes
     * reliability measurable across builds: grep the device log for `meter self-check` and
     * count verdicts instead of watching a pill.
     */
    private static readonly SELF_CHECK_MS = 20_000;

    // Not its own. Your level is one entry in the same snapshot as everyone else's, and a
    // second mechanism for it is what left the pill's meter still while the roster's moved.
    private readonly levels: PlayerLevelSources;
    private readonly store: Store;
    private unlisteners: UnlistenFn[] = [];
    private readonly diagnosticsStore: Writable<VoiceDiagnostics | null> = writable(null);
    private probe: ReturnType<typeof setInterval> | null = null;
    private selfCheck: ReturnType<typeof setTimeout> | null = null;

    constructor(store: Store, levels: PlayerLevelSources) {
        this.store = store;
        this.levels = levels;
    }

    /** Your microphone, for the pill's meter. The roster's own source, not a parallel one. */
    get micSource() {
        return this.levels.own();
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
            this.diagnosticsStore.set({ backend, mic: this.levels.activity });
            this.state.sync(
                {
                    mode: backend.voiceMode === 'pushToTalk' ? 'ptt' : 'activated',
                    muted: backend.inputMuted,
                    deafened: backend.outputMuted,
                    // Reconciled here for the same reason as mute: the events that announce
                    // it are not the only way it changes, and one that does not arrive used
                    // to leave the button wrong for the rest of the session.
                    recording: backend.recording,
                    recordAllowed: backend.recordingAllowed,
                },
                performance.now(),
            );
            // Reconciles an optimistic hold the backend refused, and a hold released by a
            // gesture whose pointerup never landed.
            this.state.hold(backend.pttActive);
        } catch (e) {
            this.diagnosticsStore.set({ backend: null, mic: this.levels.activity, error: String(e) });
        }
    }

    /**
     * Re-entrant, because a reconnect calls it again on the same instance.
     *
     * The instance is reused rather than replaced — the pill holds `micSource` from mount and
     * the status panel subscribes to `diagnostics` once — so this has to be able to run twice
     * without stacking a second set of listeners and a second probe on top of the first.
     */
    async start(): Promise<void> {
        this.detach();
        await this.seed();
        await this.subscribe();
        await this.pollBackend();
        this.probe = setInterval(() => void this.pollBackend(), SelfController.PROBE_MS);
        if (this.selfCheck === null) {
            this.selfCheck = setTimeout(() => this.logSelfCheck(), SelfController.SELF_CHECK_MS);
        }
    }

    /**
     * One line per launch stating which meter layer, if any, is broken.
     *
     * The pill has failed in two different layers on the same phone — levels that never
     * arrived, and levels that arrived and were never drawn — and both look identical on
     * screen. This is the measurement: launch the app, grep the device log for
     * `meter self-check`, read the verdict. No panel, no debugger, no watching.
     */
    private logSelfCheck(): void {
        const mic = this.levels.activity;
        const meter = MeterProbe.read('self');
        const verdict = !mic.attached
            ? 'FEED-DETACHED'
            : mic.events === 0
              ? 'NO-EVENTS'
              : meter.mounted && meter.levels > 0 && meter.paints === 0
                ? 'NO-PAINTS'
                : 'OK';
        void info(
            `meter self-check: ${verdict} — feed attached=${mic.attached} events=${mic.events} ` +
                `rate=${mic.eventsPerSecond.toFixed(1)}/s ownLevel=${mic.lastRms.toFixed(2)} | ` +
                `pill mounted=${meter.mounted} levels=${meter.levels} paints=${meter.paints}`,
        );
    }

    /** Drop every listener and the probe, leaving the state and the level source in place. */
    private detach(): void {
        if (this.probe) {
            clearInterval(this.probe);
            this.probe = null;
        }
        for (const off of this.unlisteners) off();
        this.unlisteners = [];
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

    /**
     * Every listener registered against `Any` rather than this webview.
     *
     * These were webview-scoped, which meant each one first called
     * `getCurrentWebviewWindow()` — and that dereferences
     * `__TAURI_INTERNALS__.metadata.currentWebview` on the spot. Where that metadata is absent
     * the call throws before any listener is registered, every event below is lost for the
     * session, and the only thing keeping the buttons honest is the one-second poll. Nothing
     * here is ever emitted to a named window, and an `Any` listener matches a targeted emit
     * too, so the scoping only ever narrowed what could work.
     */
    private async subscribe(): Promise<void> {
        const subscribe = async <T>(event: string, run: (payload: T) => void) => {
            try {
                this.unlisteners.push(await listen<T>(event, (e) => run(e.payload)));
            } catch (e) {
                warn(`SelfController: could not listen for ${event}: ${e}`);
            }
        };

        // Logged at info because this is the half of the round trip a device log cannot
        // otherwise see: the command's own logging proves the press reached Rust, and this
        // proves the answer came back. A press that logs one and not the other localises the
        // fault immediately.
        await subscribe<boolean>('mute:input', (muted) => {
            info(`SelfController: mute:input echo -> ${muted}`);
            this.state.sync({ muted });
        });
        await subscribe<boolean>('mute:output', (deafened) => {
            info(`SelfController: mute:output echo -> ${deafened}`);
            this.state.sync({ deafened });
        });
        await subscribe<boolean>('ptt:active', (down) => this.state.hold(down));
        // Settings, a Stream Deck and a hotkey all write the same setting, and the mic
        // button is a hold in one mode and a toggle in the other. Read once at start-up it
        // goes stale the first time anything changes it.
        await subscribe<ConfiguredVoiceMode>('voice-mode:changed', (mode) => {
            info(`SelfController: voice mode -> ${mode}`);
            this.state.sync({ mode: mode === 'pushToTalk' ? 'ptt' : 'activated' });
        });
        await subscribe<unknown>('recording:started', () =>
            this.state.sync({ recording: true }, performance.now()),
        );
        await subscribe<unknown>('recording:stopped', () => this.state.sync({ recording: false }));
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

    /**
     * Arm or finish a recording.
     *
     * Settles on the command rather than on the event, which is what mute and deafen
     * already did. Firing and forgetting left the button waiting for `recording:started`,
     * and an event that did not arrive stranded it off over a backend that was recording:
     * the next press asked to start again, was refused with "Recording already in
     * progress", and nothing corrected either side.
     *
     * A refusal is the strongest evidence available that the button is the wrong one, so
     * it re-reads rather than keeping its own answer.
     */
    pressRecord(): void {
        const recording = this.state.snapshot.recording;
        // Stopping stays available on a server that turned recording off mid-session:
        // the refusal must never be the thing that strands an open recording.
        if (!recording && !this.state.snapshot.recordAllowed) {
            return;
        }
        this.state.sync({ recording: !recording }, performance.now());
        void this.confirm(recording ? 'stop_recording' : 'start_recording');
    }

    private async confirm(command: string): Promise<void> {
        try {
            info(`SelfController: invoking ${command}`);
            await invoke(command, {});
        } catch (e) {
            warn(`SelfController: ${command} failed: ${e}`);
            await this.seed();
        }
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
        this.detach();
        if (this.selfCheck !== null) {
            clearTimeout(this.selfCheck);
            this.selfCheck = null;
        }
    }
}
