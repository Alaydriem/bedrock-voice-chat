/** What a named meter has been handed, and what it has actually drawn. */
export interface MeterProbeSnapshot {
    /** Whether any binding has ever registered under this name. */
    readonly mounted: boolean;
    /**
     * How many bindings hold this name right now.
     *
     * More than one means the counts below are merged, so "it is painting" is a claim about
     * whichever canvas painted rather than about the one being looked at.
     */
    readonly bindings: number;
    /** Audible levels handed to the binding by its source. Silence is not counted. */
    readonly levels: number;
    readonly lastLevel: number;
    /** Milliseconds since the last audible level, or null if none ever arrived. */
    readonly levelAgeMs: number | null;
    /** Frames drawn above the resting floor. */
    readonly paints: number;
    readonly paintAgeMs: number | null;
}

interface MeterRecord {
    bindings: number;
    levels: number;
    lastLevel: number;
    levelAt: number | null;
    paints: number;
    paintAt: number | null;
}

/**
 * A ledger of what each named meter received against what it drew.
 *
 * The self pill has failed in two different layers on the same phone — a listener the page
 * silently skipped, and a renderer that drew nothing while levels demonstrably arrived — and
 * from the outside both are the same flat meter. A binding that carries a probe name records
 * both sides here, so a broken launch names its own layer: levels without paints is a renderer
 * fault, and no levels at all sends the investigation back to the feed.
 *
 * Counts survive re-registration because a remounted pill is the same meter: zeroing them on
 * mount would erase exactly the history a stall was measured by.
 */
export class MeterProbe {
    static #records = new Map<string, MeterRecord>();

    /** Announce a binding under this name, so "mounted, silent" reads apart from "absent". */
    static register(name: string): void {
        const record = MeterProbe.#records.get(name);
        if (record) {
            record.bindings += 1;
            return;
        }
        MeterProbe.#records.set(name, {
            bindings: 1,
            levels: 0,
            lastLevel: 0,
            levelAt: null,
            paints: 0,
            paintAt: null,
        });
    }

    /**
     * Give up a binding's claim on this name, from its `destroy`.
     *
     * The counts stay: they are the evidence a stall was measured by, and a meter that was
     * remounted is the same meter. Only the number of canvases behind them changes.
     */
    static release(name: string): void {
        const record = MeterProbe.#records.get(name);
        if (!record) return;
        record.bindings = Math.max(0, record.bindings - 1);
    }

    /** Record a level the binding was handed. Silence is ignored: the meter is fed a zero
     * between every burst, and a "last level" that is almost always 0.00 says nothing. */
    static level(name: string, value: number): void {
        const record = MeterProbe.#records.get(name);
        if (!record || value <= 0) return;
        record.lastLevel = value;
        record.levels += 1;
        record.levelAt = performance.now();
    }

    /** Record a frame drawn above the resting floor. */
    static painted(name: string): void {
        const record = MeterProbe.#records.get(name);
        if (!record) return;
        record.paints += 1;
        record.paintAt = performance.now();
    }

    static read(name: string): MeterProbeSnapshot {
        const record = MeterProbe.#records.get(name);
        if (!record) {
            return {
                mounted: false,
                bindings: 0,
                levels: 0,
                lastLevel: 0,
                levelAgeMs: null,
                paints: 0,
                paintAgeMs: null,
            };
        }
        const now = performance.now();
        return {
            mounted: true,
            bindings: record.bindings,
            levels: record.levels,
            lastLevel: record.lastLevel,
            levelAgeMs: record.levelAt === null ? null : now - record.levelAt,
            paints: record.paints,
            paintAgeMs: record.paintAt === null ? null : now - record.paintAt,
        };
    }

    /** Forget a name entirely. For tests. */
    static reset(name: string): void {
        MeterProbe.#records.delete(name);
    }
}
