/** What the watchdog needs from a sentinel: a write, and whether it was applied. */
export interface SchedulerProbe {
    pulse(): void;
    readonly settled: boolean;
}

/** Trigger tuning. Tests shorten these; the defaults are the shipped behavior. */
export interface ReactivityWatchdogOptions {
    heartbeatMs?: number;
    minGapMs?: number;
}

/**
 * Recovers Svelte's reactivity scheduler when it wedges.
 *
 * A long enough suspension — minutes in the background under a heavy game — can cost the
 * scheduler its pending drain callback. Its queue then reads as "drain already scheduled"
 * forever, and the current batch as "someone will flush me", so neither ever re-arms: the
 * UI takes every tap and paints none of them, while timers, canvases and the Rust side
 * carry on. The state is invisible from inside — no error, no rejected promise — and
 * permanent until something calls `flushSync`.
 *
 * Three triggers, because the wedge shows itself three ways:
 *
 * - `visibilitychange` to visible: the suspension that caused it just ended.
 * - `pointerdown`, throttled: a foreground wedge surfaces as a tap that does nothing, and
 *   that same tap is the recovery — its writes land in the backlog the flush applies, so
 *   the first "dead" tap works, one macrotask late.
 * - a heartbeat: a frozen meter with no finger near the screen has neither of the above.
 */
export class ReactivityWatchdog {
    /**
     * How long the probe gets to settle before the scheduler is declared wedged.
     *
     * A healthy flush is a microtask, and a macrotask boundary runs after every pending
     * microtask by definition — so zero waits exactly long enough, at any load.
     */
    static readonly SETTLE_DELAY_MS = 0;

    /**
     * The heartbeat period. Each beat costs one state write and one comparison — nothing
     * against the 100 ms level publisher — so this is set by how long a visibly frozen
     * meter is tolerable, not by what the check costs.
     */
    static readonly HEARTBEAT_MS = 5_000;

    /**
     * The floor between checks, whatever triggers them. A drag emits no pointerdown after
     * its first, so this only spaces out tap bursts, where one check answers for all.
     */
    static readonly MIN_GAP_MS = 1_000;

    #probe: SchedulerProbe;
    #flush: () => void;
    #report: (message: string) => void;
    #heartbeatMs: number;
    #minGapMs: number;
    #checking = false;
    #lastCheckAt = 0;
    #heartbeat: ReturnType<typeof setInterval> | null = null;
    #onVisibility: () => void;
    #onPointerDown: () => void;

    constructor(
        probe: SchedulerProbe,
        flush: () => void,
        report: (message: string) => void,
        options: ReactivityWatchdogOptions = {},
    ) {
        this.#probe = probe;
        this.#flush = flush;
        this.#report = report;
        this.#heartbeatMs = options.heartbeatMs ?? ReactivityWatchdog.HEARTBEAT_MS;
        this.#minGapMs = options.minGapMs ?? ReactivityWatchdog.MIN_GAP_MS;
        // A resume check ignores the gap: it is the highest-signal trigger, and the
        // suspension that just ended is exactly when a stale gap timestamp lies.
        this.#onVisibility = () => {
            if (document.visibilityState === "visible") void this.check();
        };
        this.#onPointerDown = () => {
            if (Date.now() - this.#lastCheckAt < this.#minGapMs) return;
            void this.check();
        };
    }

    start(): void {
        document.addEventListener("visibilitychange", this.#onVisibility);
        // Capture phase and passive: it must fire however wedged the tree above the tap
        // is, and it must never stall scrolling.
        document.addEventListener("pointerdown", this.#onPointerDown, {
            capture: true,
            passive: true,
        });
        this.#heartbeat = setInterval(this.#onPointerDown, this.#heartbeatMs);
    }

    cleanup(): void {
        document.removeEventListener("visibilitychange", this.#onVisibility);
        document.removeEventListener("pointerdown", this.#onPointerDown, { capture: true });
        if (this.#heartbeat !== null) {
            clearInterval(this.#heartbeat);
            this.#heartbeat = null;
        }
    }

    /** Probes the scheduler and heals it if wedged. True when a flush was needed. */
    async check(): Promise<boolean> {
        if (this.#checking) return false;
        this.#checking = true;
        this.#lastCheckAt = Date.now();
        try {
            this.#probe.pulse();
            await new Promise((r) => setTimeout(r, ReactivityWatchdog.SETTLE_DELAY_MS));
            if (this.#probe.settled) return false;

            this.#flush();
            this.#report(
                "Svelte scheduler was wedged; flushSync applied the backlog",
            );
            return true;
        } finally {
            this.#checking = false;
        }
    }
}
