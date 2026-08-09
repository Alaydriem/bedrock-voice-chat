import { debug, info } from '@tauri-apps/plugin-log';

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
    /** Tasks that occupied the JS thread for over 50 ms, as the platform reports them. */
    private readonly longTasks: { name: string; start: number; duration: number }[] = [];
    private longTaskObserver: PerformanceObserver | null = null;

    static shared(): BootTimeline {
        if (!BootTimeline.instance) {
            BootTimeline.instance = new BootTimeline();
            BootTimeline.instance.watchLongTasks();
        }
        return BootTimeline.instance;
    }

    /**
     * Record what monopolises the JS thread.
     *
     * A scheduler trace can show that thread pinned at 95% for the whole launch but not what is
     * running on it, because WebView emits no Chromium track events. `longtask` is the platform's
     * own answer to the same question, and it costs nothing when nothing is slow.
     */
    private watchLongTasks(): void {
        if (typeof PerformanceObserver === 'undefined') return;
        try {
            this.longTaskObserver = new PerformanceObserver((list) => {
                for (const entry of list.getEntries()) {
                    this.longTasks.push({
                        name: entry.name,
                        start: entry.startTime,
                        duration: entry.duration,
                    });
                }
            });
            this.longTaskObserver.observe({ entryTypes: ['longtask'] });
        } catch {
            // Not every engine implements the entry type; its absence is not worth a log line
            // on a path that exists to measure other things.
        }
    }

    mark(name: string): void {
        this.markAt(name, performance.now());
    }

    /**
     * Record a phase that finished before the bundle could measure it.
     *
     * The document is parsing and fetching for some time before any of this code exists, so
     * that stretch can only be timed by something stashing a `performance.now()` on `window`
     * and handing it over once there is a timeline to hand it to.
     */
    markAt(name: string, at: number): void {
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
        debug(
            [
                '',
                '=== BOOT TIMELINE ===',
                `  ${'phase'.padEnd(width)}  ${'delta'.padStart(6)}      since launch`,
                ...lines,
                `  ${'TOTAL'.padEnd(width)}  ${total.toFixed(0).padStart(6)} ms`,
                ...BootTimeline.markResizeLines(),
                ...this.longTaskLines(),
                ...BootTimeline.deliveryLines(),
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

    /**
     * The tasks that held the JS thread, worst first.
     *
     * The total against the launch total is the number that matters: a launch whose thread is
     * busy for most of its duration is not waiting on anything, and every phase it reports is
     * really queueing behind these.
     */
    private longTaskLines(): string[] {
        if (!this.longTasks.length) return [];

        const total = this.longTasks.reduce((sum, t) => sum + t.duration, 0);
        const worst = [...this.longTasks].sort((a, b) => b.duration - a.duration).slice(0, 8);

        return [
            '',
            `  long tasks — ${this.longTasks.length} over 50ms, ${total.toFixed(0)} ms of JS thread`,
            ...worst.map(
                (t) => `    t+${t.start.toFixed(0).padStart(5)} ms   ${t.duration.toFixed(0).padStart(4)} ms   ${t.name}`,
            ),
        ];
    }

    /**
     * How the frontend arrived, and how long the document itself took.
     *
     * Printed on every timeline because a dev-server launch serves unbundled ES modules — one
     * request per module — and an installed build serves a handful of pre-built chunks. The two
     * are not comparable, and a timeline that does not say which one it is cannot be trusted
     * against any other timeline. The request count is the tell: single digits is a build,
     * hundreds is a dev server.
     */
    private static deliveryLines(): string[] {
        const lines: string[] = [];

        const requests = performance.getEntriesByType('resource').length;
        const bundled = !import.meta.env.DEV;
        lines.push(
            '',
            `  delivery — ${bundled ? 'bundled build' : 'DEV SERVER (not comparable to a build)'}`,
            `    document requests      ${requests}`,
        );

        const nav = performance.getEntriesByType('navigation')[0] as
            | PerformanceNavigationTiming
            | undefined;
        if (nav) {
            lines.push(
                `    responseEnd            ${nav.responseEnd.toFixed(0)} ms`,
                `    domContentLoaded       ${nav.domContentLoadedEventEnd.toFixed(0)} ms`,
                `    domComplete            ${nav.domComplete.toFixed(0)} ms`,
            );
        }

        return lines;
    }
}
