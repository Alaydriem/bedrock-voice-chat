import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { warn } from '@tauri-apps/plugin-log';
import type { LevelSource } from '$radial/core/sources/LevelSource';
import { PushLevelSource } from '$radial/core/sources/LevelSource';
import { LevelScale } from '$radial/core/sources/LevelScale';

/** Payload of `audio-input-level`, emitted from the capture stream at about 10 Hz. */
interface InputLevel {
    rms: number;
    gate_open: boolean;
}

/**
 * Your own microphone, as a `LevelSource`.
 *
 * The emitter only ticks when a level arrived in its interval, so a stream that stops
 * pushing — a closed gate, a device that went away — leaves the last value standing. A
 * meter frozen at half height reads as a live microphone, which is the one thing it must
 * never do, so silence decays to nothing here rather than being assumed away.
 */
export class MicLevelSource {
    /** No push for this long means nothing is arriving, not that the level held. */
    private static readonly SILENCE_MS = 250;

    private static readonly SWEEP_MS = 100;

    private readonly level = new PushLevelSource();
    private unlisten: UnlistenFn | null = null;
    private sweep: ReturnType<typeof setInterval> | null = null;
    private lastPush = 0;
    private gateOpenAt = false;

    /** Hand this to a meter. */
    get source(): LevelSource {
        return this.level;
    }

    /** Whether the noise gate was open on the most recent frame that carried signal. */
    get gateOpen(): boolean {
        return this.gateOpenAt;
    }

    async start(): Promise<void> {
        this.stop();

        // Webview-scoped rather than the global listener: on Android a backend emit reaches
        // this reliably where a global-target listener can miss it, and it is the pattern the
        // working mute buttons already use.
        try {
            this.unlisten = await getCurrentWebviewWindow().listen<InputLevel>(
                'audio-input-level',
                (event) => {
                    this.gateOpenAt = event.payload.gate_open;
                    this.lastPush = performance.now();
                    // Scaled, not raw. Speech sits around 0.02-0.08 RMS and the meter's silence
                    // threshold is 0.08, so pushing the measurement straight through draws
                    // ordinary talking as silence.
                    this.level.push(LevelScale.fromRms(event.payload.rms));
                },
            );
        } catch (e) {
            warn(`MicLevelSource: could not listen for input level: ${e}`);
        }

        this.sweep = setInterval(() => {
            if (this.level.level === 0) return;
            if (performance.now() - this.lastPush < MicLevelSource.SILENCE_MS) return;
            this.gateOpenAt = false;
            this.level.push(0);
        }, MicLevelSource.SWEEP_MS);
    }

    stop(): void {
        if (this.unlisten) {
            this.unlisten();
            this.unlisten = null;
        }
        if (this.sweep) {
            clearInterval(this.sweep);
            this.sweep = null;
        }
        this.level.push(0);
    }
}
