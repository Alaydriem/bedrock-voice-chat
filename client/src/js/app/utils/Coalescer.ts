/**
 * One crossing of the process boundary per interval, however fast the input arrives.
 *
 * A slider drag fires an input event per pixel of travel. Doing the persist-and-push work on
 * each one meant four IPC round trips and a disk write per event, on a channel Android
 * serialises — so the queue grew faster than it drained and the audio lagged the finger by
 * however long it had got. Reactive state still updates on every event; only the trips out of
 * the webview are coalesced.
 *
 * Leading edge, so a single click is immediate and feels like a direct response. Trailing edge
 * guaranteed, so the last value a drag settled on is never the one that gets dropped — which is
 * the only value that actually matters.
 */
export class Coalescer {
    private readonly gapMs: number;
    private readonly run: () => Promise<void>;

    private timer: ReturnType<typeof setTimeout> | null = null;
    private lastStart = 0;
    private running = false;
    private pending = false;

    constructor(gapMs: number, run: () => Promise<void>) {
        this.gapMs = gapMs;
        this.run = run;
    }

    /**
     * Ask for a run.
     *
     * Runs now if nothing has run recently, otherwise marks the work outstanding and lets the
     * interval that is already waiting pick it up. Requests never queue: they collapse, because
     * the work reads current state rather than carrying a value of its own.
     */
    request(): void {
        if (this.running) {
            this.pending = true;
            return;
        }

        const since = Date.now() - this.lastStart;
        if (since >= this.gapMs) {
            void this.fire();
            return;
        }

        this.pending = true;
        if (!this.timer) {
            this.timer = setTimeout(() => {
                this.timer = null;
                if (this.pending) void this.fire();
            }, this.gapMs - since);
        }
    }

    private async fire(): Promise<void> {
        this.pending = false;
        this.running = true;
        this.lastStart = Date.now();
        try {
            await this.run();
        } finally {
            this.running = false;
            // Anything that arrived while this was in flight gets its own run, so a drag that
            // ends mid-write still ends up persisted.
            if (this.pending) this.request();
        }
    }

    cancel(): void {
        if (this.timer) {
            clearTimeout(this.timer);
            this.timer = null;
        }
        this.pending = false;
    }
}
