import { invoke } from '@tauri-apps/api/core';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { info, warn } from '@tauri-apps/plugin-log';
import type { Store } from '@tauri-apps/plugin-store';
import { SelfState } from '$radial/core/controllers/SelfState';
import type { KeybindConfig } from '../../bindings/KeybindConfig';
import { MicLevelSource } from './MicLevelSource';

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
export class SelfController {
    readonly state = new SelfState();

    private readonly mic = new MicLevelSource();
    private readonly store: Store;
    private unlisteners: UnlistenFn[] = [];

    constructor(store: Store) {
        this.store = store;
    }

    /** Your microphone, for the pill's meter. */
    get micSource() {
        return this.mic.source;
    }

    async start(): Promise<void> {
        await this.seed();
        await this.subscribe();
        await this.mic.start();
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

    /** Push-to-talk from the button. The global hotkey arrives as `ptt:active` instead. */
    hold(down: boolean): void {
        this.state.hold(down);
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
        for (const off of this.unlisteners) off();
        this.unlisteners = [];
        this.mic.stop();
    }
}
