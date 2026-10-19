import { warn } from "@charlesportwoodii/tauri-plugin-curia";
import type { UpdateStatus } from "../settings/UpdateStatus";

interface UpdatePollerOptions {
    readonly firstDelayMs?: number;
    readonly intervalMs?: number;
}

/**
 * Keeps the About pane's update row current through a long dashboard session.
 *
 * The launch offer is `UpdateOffer`'s. This only refreshes the row and its badge, so it
 * starts late and repeats slowly.
 */
export class UpdatePoller {
    /** Clear of first paint and of the connect that follows it. */
    private static readonly DEFAULT_FIRST_DELAY_MS = 30_000;
    private static readonly DEFAULT_INTERVAL_MS = 6 * 60 * 60 * 1000;

    private readonly updates: UpdateStatus;
    private readonly firstDelayMs: number;
    private readonly intervalMs: number;

    private first: ReturnType<typeof setTimeout> | null = null;
    private repeat: ReturnType<typeof setInterval> | null = null;

    constructor(updates: UpdateStatus, options: UpdatePollerOptions = {}) {
        this.updates = updates;
        this.firstDelayMs = options.firstDelayMs ?? UpdatePoller.DEFAULT_FIRST_DELAY_MS;
        this.intervalMs = options.intervalMs ?? UpdatePoller.DEFAULT_INTERVAL_MS;
    }

    start(): void {
        if (this.first !== null || this.repeat !== null) return;

        this.first = setTimeout(() => {
            this.first = null;
            this.run();
            this.repeat = setInterval(() => this.run(), this.intervalMs);
        }, this.firstDelayMs);
    }

    stop(): void {
        if (this.first !== null) {
            clearTimeout(this.first);
            this.first = null;
        }
        if (this.repeat !== null) {
            clearInterval(this.repeat);
            this.repeat = null;
        }
    }

    /**
     * A failed check is already a state `UpdateStatus` records and `AboutPane` has a sentence
     * for, so nothing here re-reports it. The catch exists because this runs detached from any
     * awaiting caller.
     */
    private run(): void {
        this.updates.check().catch((e) => {
            warn(`Background update check failed: ${e}`);
        });
    }
}
