import type { SchedulerProbe } from "./ReactivityWatchdog";

/**
 * A sentinel wired through Svelte's own scheduler.
 *
 * Svelte 5 funnels every state-to-DOM update through one module-level task queue, and that
 * queue schedules its drain only when it believes none is pending. A drain callback lost
 * while the webview is suspended leaves the belief standing with nothing behind it: from
 * then on every handler runs, every write is accepted, and no update ever reaches the DOM.
 * Nothing inside the app throws, so the only way to see the state is to behave like the
 * app and watch whether the scheduler answers.
 *
 * `pulse` writes a `$state`; the effect below copies it out again. The copy happens only
 * when the scheduler runs effects, so `settled` is a direct reading of the machinery that
 * the wedge disables — not of the DOM, not of timers, both of which stay healthy.
 */
export class ReactivityProbe implements SchedulerProbe {
    #counter = $state(0);
    #observed = 0;
    #stop: () => void;

    constructor() {
        this.#stop = $effect.root(() => {
            $effect(() => {
                this.#observed = this.#counter;
            });
        });
    }

    pulse(): void {
        this.#counter++;
    }

    get settled(): boolean {
        return this.#observed === this.#counter;
    }

    cleanup(): void {
        this.#stop();
    }
}
