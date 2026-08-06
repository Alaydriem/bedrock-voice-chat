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

/** Whether the capture stream is emitting at all, and what it last measured. */
export interface MicActivity {
    readonly events: number;
    readonly eventsPerSecond: number;
    readonly lastRms: number;
    /** Milliseconds since the last event, or null if none has ever arrived. */
    readonly silentForMs: number | null;
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
    private received = 0;
    private lastRms = 0;
    private startedAt = 0;

    /** Hand this to a meter. */
    get source(): LevelSource {
        return this.level;
    }

    /** Whether the noise gate was open on the most recent frame that carried signal. */
    get gateOpen(): boolean {
        return this.gateOpenAt;
    }

    /**
     * Proof of life for the capture stream, for the diagnostics readout.
     *
     * A muted input still emits, at `rms: 0`, and a stream that has stopped emitting decays
     * to zero here — so the meter draws both as the same flat line. The event count is what
     * tells them apart: zero events means nothing is capturing, events at rms 0 means a live
     * stream with a muted microphone.
     */
    get activity(): MicActivity {
        const elapsed = this.startedAt ? (performance.now() - this.startedAt) / 1000 : 0;
        return {
            events: this.received,
            eventsPerSecond: elapsed > 0 ? this.received / elapsed : 0,
            lastRms: this.lastRms,
            silentForMs: this.lastPush ? performance.now() - this.lastPush : null,
        };
    }

    async start(): Promise<void> {
        this.stop();
        this.received = 0;
        this.lastRms = 0;
        this.lastPush = 0;
        this.startedAt = performance.now();

        // Webview-scoped rather than the global listener: on Android a backend emit reaches
        // this reliably where a global-target listener can miss it, and it is the pattern the
        // working mute buttons already use.
        try {
            this.unlisten = await getCurrentWebviewWindow().listen<InputLevel>(
                'audio-input-level',
                (event) => {
                    this.gateOpenAt = event.payload.gate_open;
                    this.lastPush = performance.now();
                    this.received += 1;
                    this.lastRms = event.payload.rms;
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
