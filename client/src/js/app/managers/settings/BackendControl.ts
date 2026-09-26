import { writable, type Readable, type Writable } from "svelte/store";
import { warn } from "@charlesportwoodii/tauri-plugin-curia";

/**
 * A setting whose value the backend owns, drawn by a control that moves before the backend answers.
 *
 * The control shows the requested value at once, so a switch or a slider moves under the finger.
 * A burst of requests inside `delayMs` sends only the last one, and the control then settles on
 * the value the backend reached, which may be clamped. An answer to an older request never
 * overwrites a newer one. When a request fails, nothing was applied, so the control returns to
 * what `recover` reads back from the backend's own copy.
 */
export class BackendControl<T> {
    private readonly valueStore: Writable<T>;
    public readonly value: Readable<T>;
    private readonly apply: (value: T) => Promise<T>;
    private readonly recover: () => Promise<T>;
    private readonly delayMs: number;
    private latest: T;
    private timer: ReturnType<typeof setTimeout> | null = null;
    private waiting: Array<() => void> = [];
    private sent = 0;

    constructor(
        initial: T,
        apply: (value: T) => Promise<T>,
        recover: () => Promise<T>,
        delayMs: number,
    ) {
        this.valueStore = writable(initial);
        this.value = { subscribe: this.valueStore.subscribe };
        this.apply = apply;
        this.recover = recover;
        this.delayMs = delayMs;
        this.latest = initial;
    }

    /** Draws a value the backend already holds, such as a saved one or a change from elsewhere. */
    set(value: T): void {
        this.valueStore.set(value);
    }

    /** Asks the backend for `next`; resolves once the control has settled or been superseded. */
    request(next: T): Promise<void> {
        this.valueStore.set(next);
        this.latest = next;
        if (this.timer !== null) clearTimeout(this.timer);

        return new Promise<void>((resolve) => {
            this.waiting.push(resolve);
            this.timer = setTimeout(() => void this.flush(), this.delayMs);
        });
    }

    private async flush(): Promise<void> {
        this.timer = null;
        const waiting = this.waiting;
        this.waiting = [];
        const request = ++this.sent;

        const settled = await this.settle(this.latest);
        // A newer request was sent or is queued; its answer is the one to draw.
        if (settled !== undefined && request === this.sent && this.timer === null) {
            this.valueStore.set(settled);
        }
        for (const resolve of waiting) resolve();
    }

    private async settle(next: T): Promise<T | undefined> {
        try {
            return await this.apply(next);
        } catch (e) {
            void warn(`A setting change was not applied: ${String(e)}`);
        }
        try {
            return await this.recover();
        } catch (e) {
            void warn(`The saved value of a setting could not be read back: ${String(e)}`);
            return undefined;
        }
    }
}
