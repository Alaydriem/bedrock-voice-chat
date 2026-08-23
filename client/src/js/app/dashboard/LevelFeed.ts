import { debug, info, warn } from '@charlesportwoodii/tauri-plugin-curia';
import type { LevelSnapshot } from '../../bindings/LevelSnapshot';
import { EventChannel } from '../events/EventChannel';

export type LevelSink = (snapshot: LevelSnapshot) => void;

/**
 * Meter levels, fanned out to every consumer from one channel subscription.
 *
 * This used to hold a Tauri event listener and a great deal of machinery to prove that listener
 * worked: a probed round trip, five retries with backoff, and a watchdog that re-opened a
 * registration lost some other way. None of it is here because none of it is needed — a socket
 * either carries frames or it does not, and `EventChannel` owns reconnecting.
 *
 * Kept as a class rather than folded into its callers because the shared lifetime is still
 * worth something: the roster outlives a reconnect and the controller is re-entered by one, so
 * threading an owner through every consumer would be more machinery than the single
 * subscription is worth.
 */
export class LevelFeed {
    static #shared: LevelFeed | null = null;

    #sinks = new Set<LevelSink>();
    /**
     * Who each sink belongs to, for the log.
     *
     * A shared fan-out whose last consumer leaves stops delivering to everybody, and until this
     * existed there was no way to see which consumer did it: the feed reported itself attached,
     * the socket stayed open, and one screen's meters simply never moved again.
     */
    #owners = new Map<LevelSink, string>();
    #off: (() => void) | null = null;
    #received = 0;

    static shared(): LevelFeed {
        LevelFeed.#shared ??= new LevelFeed();
        return LevelFeed.#shared;
    }

    /** Snapshots delivered since the subscription opened. */
    get received(): number {
        return this.#received;
    }

    /** Whether the feed is holding a channel subscription. */
    get attached(): boolean {
        return this.#off !== null;
    }

    /**
     * Take levels until the returned function is called.
     *
     * The channel subscription opens on the first sink and closes after the last one leaves, so
     * a screen that is gone stops costing a delivery.
     */
    subscribe(sink: LevelSink, owner = 'unknown'): () => void {
        this.#sinks.add(sink);
        this.#owners.set(sink, owner);
        this.#off ??= EventChannel.shared().subscribe<LevelSnapshot>('levels', (snapshot) =>
            this.#deliver(snapshot),
        );
        void debug(`LevelFeed: ${owner} subscribed (${this.#sinks.size} holding)`);

        let released = false;
        return () => {
            if (released) return;
            released = true;
            this.#sinks.delete(sink);
            this.#owners.delete(sink);
            if (this.#sinks.size > 0) {
                void debug(
                    `LevelFeed: ${owner} released; still held by ${[...this.#owners.values()].join(', ')}`,
                );
                return;
            }
            // The moment every meter in the window goes still. Logged as a fault because the
            // feed cannot tell a screen that meant to leave from one that lost its subscription
            // and is still on screen expecting levels.
            void debug(`LevelFeed: ${owner} released the last sink; the channel subscription is gone`);
            this.#off?.();
            this.#off = null;
            this.#received = 0;
        };
    }

    #deliver(snapshot: LevelSnapshot): void {
        this.#received += 1;
        // Each sink is isolated: one throwing must not stop the others being fed, and it must
        // not take the count with it either — a handler that failed and a frame that never came
        // are different faults with different fixes.
        for (const sink of this.#sinks) {
            try {
                sink(snapshot);
            } catch {
                // EventChannel logs the failure; swallowing here keeps the remaining sinks fed.
            }
        }
    }
}
