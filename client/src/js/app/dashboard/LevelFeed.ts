import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { info, warn } from '@tauri-apps/plugin-log';
import type { LevelSnapshot } from '../../bindings/LevelSnapshot';

export type LevelSink = (snapshot: LevelSnapshot) => void;

/**
 * The one subscription to `audio-levels`, fanned out in JavaScript.
 *
 * There were three — the pill's meter, the roster's level sources, and the activity manager —
 * each calling `listen` for the same event and each getting its own registration. One of them
 * delivered and another did not, on the same event, in the same window, with the second
 * reporting itself as attached; a registered handler that is simply never invoked leaves
 * nothing to inspect from inside the page.
 *
 * So there is one registration and the fan-out happens here, where it can be reasoned about.
 * That is also what the merge was for: one event carrying everyone's levels wants one
 * subscriber, and three was a leftover from when there were two events and several readers.
 *
 * Shared rather than injected, like the animation loop. The consumers have different lifetimes
 * — the roster outlives a reconnect, the controller is re-entered by one — and threading an
 * owner through all of them to guarantee a single listener would be more machinery than the
 * guarantee is worth.
 */
export class LevelFeed {
    static #shared: LevelFeed | null = null;

    #sinks = new Set<LevelSink>();
    #unlisten: UnlistenFn | null = null;
    #starting: Promise<void> | null = null;
    #received = 0;

    static shared(): LevelFeed {
        LevelFeed.#shared ??= new LevelFeed();
        return LevelFeed.#shared;
    }

    /** Events delivered since the subscription opened. */
    get received(): number {
        return this.#received;
    }

    /** Whether the single underlying listener is registered. */
    get attached(): boolean {
        return this.#unlisten !== null;
    }

    /**
     * Take levels until the returned function is called.
     *
     * The registration is opened on the first subscriber and closed after the last one leaves,
     * so a screen that is gone stops costing a delivery — and a re-entered one does not open a
     * second registration on top of the first.
     */
    subscribe(sink: LevelSink): () => void {
        this.#sinks.add(sink);
        void this.#open();
        return () => {
            this.#sinks.delete(sink);
            if (this.#sinks.size === 0) this.#close();
        };
    }

    async #open(): Promise<void> {
        if (this.#unlisten) return;
        if (this.#starting) {
            await this.#starting;
            return;
        }

        this.#starting = (async () => {
            try {
                const off = await listen<LevelSnapshot>('audio-levels', (event) => {
                    this.#received += 1;
                    // Each sink is isolated: one throwing must not stop the others being fed,
                    // and it must not take the count with it either — a handler that failed and
                    // an event that never came are different faults with different fixes.
                    for (const sink of this.#sinks) {
                        try {
                            sink(event.payload);
                        } catch (e) {
                            void warn(`LevelFeed: a level sink failed: ${e}`);
                        }
                    }
                });
                // Raced against its own teardown: a subscriber that left while this was in
                // flight leaves nothing to feed, and the registration has to go with it rather
                // than linger unread.
                if (this.#sinks.size === 0) {
                    off();
                    return;
                }
                this.#unlisten = off;
                void info('LevelFeed: subscribed to audio-levels');
            } catch (e) {
                void warn(`LevelFeed: could not subscribe to audio-levels: ${e}`);
            } finally {
                this.#starting = null;
            }
        })();

        await this.#starting;
    }

    #close(): void {
        this.#unlisten?.();
        this.#unlisten = null;
        this.#received = 0;
    }
}
