import { info } from '@tauri-apps/plugin-log';

interface BootMark {
    readonly name: string;
    /** Milliseconds since the document started, so marks compare across routes. */
    readonly at: number;
    /** Milliseconds since the previous mark, which is the cost of that phase. */
    readonly delta: number;
}

/**
 * Where launch time actually goes.
 *
 * Diagnostic instrumentation, not part of the boot contract: it answers one question — whether
 * the wait is spent in the count of IPC round trips or in the QUIC handshake — because those
 * two answers point at completely different work.
 *
 * `performance.now()` is relative to the document's time origin, and launch is now a single
 * document load, so every mark is directly comparable to every other one no matter which route
 * recorded it.
 */
export class BootTimeline {
    private static instance: BootTimeline | null = null;

    private readonly marks: BootMark[] = [];
    private last = 0;
    private reportedThrough = 0;

    /**
     * The one timeline for this launch.
     *
     * Shared the way `AnimationLoop` is, because the phases being measured are spread across
     * the launch route and the dashboard and the total is the only interesting number.
     */
    static shared(): BootTimeline {
        if (!BootTimeline.instance) {
            BootTimeline.instance = new BootTimeline();
        }
        return BootTimeline.instance;
    }

    mark(name: string): void {
        const at = performance.now();
        this.marks.push({ name, at, delta: at - this.last });
        this.last = at;
    }

    /**
     * Emit the whole timeline as one log entry.
     *
     * One entry rather than a line per mark: the phases are only meaningful against each other,
     * and interleaved single lines are unreadable next to the rest of the boot log.
     *
     * Callable more than once. The launch route reports when it is the destination, and the
     * dashboard reports when it lands — on a saved-server list those are two separate launches
     * of the same timeline, and suppressing the second would lose the half that matters.
     * Repeated calls with nothing new to say stay quiet.
     */
    report(): void {
        if (this.marks.length === this.reportedThrough) return;
        this.reportedThrough = this.marks.length;

        const width = this.marks.reduce((w, m) => Math.max(w, m.name.length), 0);
        const slowest = this.marks.reduce(
            (worst, m) => (m.delta > worst.delta ? m : worst),
            { name: '', at: 0, delta: -1 } as BootMark,
        );

        const lines = this.marks.map((m) => {
            const flag = m === slowest ? '  <-- slowest' : '';
            return `  ${m.name.padEnd(width)}  ${m.delta.toFixed(0).padStart(6)} ms   (t+${m.at.toFixed(0)})${flag}`;
        });

        const total = this.marks.length ? this.marks[this.marks.length - 1].at : 0;
        info(
            [
                '',
                '=== BOOT TIMELINE ===',
                `  ${'phase'.padEnd(width)}  ${'delta'.padStart(6)}      since launch`,
                ...lines,
                `  ${'TOTAL'.padEnd(width)}  ${total.toFixed(0).padStart(6)} ms`,
                ...BootTimeline.markResizeLines(),
                '=====================',
            ].join('\n'),
        );
    }

    /**
     * The preloader's record of every change to its mark's box.
     *
     * Recorded there and read here because the preloader runs before the bundle and has no
     * logger. A single entry means the mark was sized once and never moved; more than one
     * means it was laid out, painted, and then resized — which is visible as the loader
     * changing size on screen after it has already appeared.
     */
    private static markResizeLines(): string[] {
        const samples = (window as unknown as { __bvcMarkResizes?: { t: number; w: number; h: number }[] })
            .__bvcMarkResizes;
        if (!samples?.length) return [];

        const lines = samples.map((s) => `    t+${String(s.t).padStart(5)} ms   ${s.w} x ${s.h}`);
        const verdict =
            samples.length === 1
                ? 'sized once, never resized'
                : `RESIZED ${samples.length - 1} time(s) after first layout`;
        return ['', `  mark box — ${verdict}`, ...lines];
    }
}
