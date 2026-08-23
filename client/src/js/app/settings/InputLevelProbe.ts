import type { LevelSource } from "$radial/core/sources/LevelSource";
import { PushLevelSource } from "$radial/core/sources/LevelSource";
import type { LevelSnapshot } from "../../bindings/LevelSnapshot";
import { LevelFeed } from "../dashboard/LevelFeed";
import { LevelSteps } from "../dashboard/LevelSteps";

/**
 * The microphone level for the audio pane, read from the capture the pill already draws.
 *
 * This used to start a capture of its own. `start_input_meter` runs `AudioStreamManager::init`,
 * which *replaces* the input stream with one that meters without transmitting — so a pane that
 * claimed one took the microphone off the air, and `stop_input_meter` on the way out left
 * nothing capturing at all. The pill went flat for the rest of the session while the pane's own
 * meter kept working, because every visit built a fresh capture for it. That asymmetry read as a
 * rendering fault for a long time; it was two different microphones.
 *
 * Nothing is claimed now. The levels arriving on the push channel are absolute state about
 * whoever is capturing, so a screen that wants to show them subscribes and reads — the same
 * frames, the same `LevelFeed`, the same source type the pill's meter consumes. A flat mark here
 * is therefore the truth rather than an artefact: nothing is capturing, which is what the pane
 * should be saying.
 */
export class InputLevelProbe {
    /**
     * How long without a level before the meter is returned to rest.
     *
     * A capture that dies mid-word leaves the last amplitude standing, and a mark held at
     * half-height reads as somebody still talking — the one thing a meter must never say.
     * Longer than the backend's keepalive-while-speaking, so a quiet room is not mistaken for a
     * dead capture: levels are published on change and silence is never re-sent.
     */
    static readonly SILENCE_MS = 4_000;

    private readonly levelSource = new PushLevelSource();

    /** What the meter consumes. The same contract the pill's meter is bound to. */
    public readonly source: LevelSource = this.levelSource;

    private unlisteners: Array<() => void> = [];
    private running = false;
    private watchdog: ReturnType<typeof setInterval> | null = null;
    private lastLevelAt = 0;

    async start(): Promise<void> {
        if (this.running) return;
        this.running = true;

        this.lastLevelAt = performance.now();
        this.unlisteners.push(
            LevelFeed.shared().subscribe((snapshot) => this.receive(snapshot), 'InputLevelProbe'),
        );

        this.watchdog = setInterval(() => this.judge(), InputLevelProbe.SILENCE_MS);
    }

    private receive(snapshot: LevelSnapshot): void {
        this.lastLevelAt = performance.now();
        this.levelSource.push(LevelSteps.toLevel(snapshot.own));
    }

    private judge(): void {
        if (performance.now() - this.lastLevelAt <= InputLevelProbe.SILENCE_MS) return;
        this.levelSource.push(0);
    }

    async stop(): Promise<void> {
        if (!this.running) return;
        this.running = false;

        if (this.watchdog !== null) {
            clearInterval(this.watchdog);
            this.watchdog = null;
        }

        for (const off of this.unlisteners) off();
        this.unlisteners = [];
        this.levelSource.push(0);
    }
}
