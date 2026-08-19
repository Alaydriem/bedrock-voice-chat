import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { info, warn } from '@tauri-apps/plugin-log';
import type { LevelSnapshot } from '../../bindings/LevelSnapshot';

export type LevelSink = (snapshot: LevelSnapshot) => void;

/**
 * The one subscription to `audio-levels`, fanned out in JavaScript — and verified, because a
 * resolved `listen()` is not proof of one.
 *
 * Registering a listener ends with an eval into the page that writes the listener's id where
 * the dispatcher looks it up. On Android that eval is fire-and-forget: it can be lost during a
 * busy page load, the promise still resolves, and the backend keeps dispatching to an id the
 * page silently skips — a phantom that reports itself attached and never receives anything.
 * Nothing inside the page can see the missing entry, which is why "one listener delivered and
 * another did not, on the same event, in the same window" was observable but not inspectable.
 *
 * So every registration is proven before it is trusted: the feed asks the backend for one
 * unconditional snapshot (`probe_audio_levels`) and waits for it to arrive through the listener
 * itself. A registration whose probe never lands is dropped and replaced, with backoff, because
 * re-registering is the only move that rolls the dice again. The last attempt is kept even
 * unverified — a false-negative probe on a starved main thread must not cost the one
 * registration that might still be working.
 *
 * Shared rather than injected, like the animation loop. The consumers have different lifetimes
 * — the roster outlives a reconnect, the controller is re-entered by one — and threading an
 * owner through all of them to guarantee a single listener would be more machinery than the
 * guarantee is worth.
 */
export class LevelFeed {
    static #shared: LevelFeed | null = null;

    /**
     * How long the probe's snapshot may take before the registration is judged a phantom.
     *
     * Generous relative to the round trip it measures, because the failure it must not
     * produce is the false negative: a busy Android main thread delays event delivery and
     * a too-eager verdict would tear down working registrations in exactly that moment.
     */
    static readonly PROBE_TIMEOUT_MS = 1000;

    /** Registration attempts before the feed stops re-rolling and keeps what it has. */
    static readonly MAX_ATTEMPTS = 5;

    /** Pause before replacing a failed registration, doubled every attempt. */
    static readonly RETRY_BASE_MS = 250;

    /**
     * How often a feed with an audience checks that it still holds a registration.
     *
     * The check is a null test on a field, so the period is set by how long a flat meter is
     * tolerable rather than by what it costs. Nothing crosses the bridge unless it finds the
     * registration gone.
     */
    static readonly WATCH_MS = 5_000;

    #sinks = new Set<LevelSink>();
    #unlisten: UnlistenFn | null = null;
    #starting: Promise<void> | null = null;
    #watch: ReturnType<typeof setInterval> | null = null;
    #received = 0;
    // The verification in flight, told about the next arrival. One at a time by construction:
    // attempts are sequential inside a single #openVerified.
    #arrival: (() => void) | null = null;

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
        this.#watchRegistration();
        return () => {
            this.#sinks.delete(sink);
            if (this.#sinks.size === 0) {
                this.#close();
                this.#stopWatching();
            }
        };
    }

    /**
     * Put a registration back when the audience still has one and the feed does not.
     *
     * Every path that drops a registration is supposed to open another, and one of them does
     * not: a teardown that lands while `#openVerified` is between registering and verifying
     * clears `#unlisten` under it, and the attempt that then succeeds returns without noticing
     * it is no longer the held one. Nothing owns the result — the sinks are still subscribed,
     * the feed holds nothing, and only a new subscriber would re-open it. On a dashboard that
     * mounts its subscribers once, there is no new subscriber, so the meters stay flat for the
     * life of the page.
     *
     * Measured that way on a live client: four `audio-levels` registrations, every callback
     * deleted, the dashboard still subscribed.
     *
     * Verification cannot cover this — it runs when a registration is opened, and the fault is
     * that none is.
     */
    #watchRegistration(): void {
        if (this.#watch !== null || typeof setInterval === 'undefined') return;
        this.#watch = setInterval(() => {
            if (this.#sinks.size === 0 || this.#unlisten || this.#starting) return;
            void warn('LevelFeed: the audio-levels registration was lost; re-opening it');
            void this.#open();
        }, LevelFeed.WATCH_MS);
    }

    #stopWatching(): void {
        if (this.#watch === null) return;
        clearInterval(this.#watch);
        this.#watch = null;
    }

    /**
     * Throw away the current registration and open a fresh, verified one.
     *
     * For a moment the app already knows about — a screen that was covering the meters closing.
     * The settings pane's own meter registers on every mount and is therefore never the broken
     * one; this gives the dashboard's long-lived registration the same fresh start rather than
     * leaving it to a watchdog to notice, or to a reload.
     *
     * Waits out a registration that is already opening rather than racing it, and does nothing
     * for an empty audience: a registration nobody reads is one the page pays for every emit.
     */
    async resync(): Promise<void> {
        if (this.#sinks.size === 0) return;
        if (this.#starting) {
            await this.#starting;
            return;
        }
        this.#close();
        await this.#open();
    }

    /** Drop the registration while keeping the audience, which is the state this recovers from. */
    forgetRegistrationForTest(): void {
        this.#unlisten = null;
    }

    async #open(): Promise<void> {
        if (this.#unlisten) return;
        if (this.#starting) {
            await this.#starting;
            // Raced against a teardown: this subscriber may have arrived while an open whose
            // whole audience had left was still winding down. It must not be left waiting on
            // a registration that was dropped for the previous, empty audience.
            if (!this.#unlisten && !this.#starting && this.#sinks.size > 0) {
                await this.#open();
            }
            return;
        }

        this.#starting = this.#openVerified().finally(() => {
            this.#starting = null;
        });
        await this.#starting;
    }

    /** Register, prove the registration receives, and replace it until one does. */
    async #openVerified(): Promise<void> {
        for (let attempt = 1; attempt <= LevelFeed.MAX_ATTEMPTS; attempt += 1) {
            if (this.#sinks.size === 0) return;

            let off: UnlistenFn;
            try {
                off = await listen<LevelSnapshot>('audio-levels', (event) =>
                    this.#deliver(event.payload),
                );
            } catch (e) {
                void warn(`LevelFeed: could not subscribe to audio-levels: ${e}`);
                return;
            }

            // The audience left while the registration was in flight; it has nothing to feed
            // and must go with them rather than linger unread.
            if (this.#sinks.size === 0) {
                this.#drop(off);
                return;
            }

            this.#unlisten = off;

            if (await this.#verify()) {
                void info(
                    attempt === 1
                        ? 'LevelFeed: subscribed to audio-levels'
                        : `LevelFeed: audio-levels subscription verified on attempt ${attempt}`,
                );
                return;
            }

            if (attempt === LevelFeed.MAX_ATTEMPTS) {
                void warn(
                    'LevelFeed: audio-levels subscription never verified; keeping the last registration',
                );
                return;
            }

            void warn(
                `LevelFeed: probe went unanswered (attempt ${attempt}); replacing the registration`,
            );
            // #close may have dropped it already if the audience emptied mid-verification.
            if (this.#unlisten === off) {
                this.#unlisten = null;
                this.#drop(off);
            }
            await LevelFeed.#delay(LevelFeed.RETRY_BASE_MS * 2 ** (attempt - 1));
        }
    }

    /**
     * Whether this registration actually receives.
     *
     * The backend emits one snapshot outside its emit policy — which never re-sends silence,
     * so a quiet room offers a fresh listener nothing to wait for otherwise — and the answer
     * is its arrival through the listener itself. Any snapshot counts, probed or not.
     *
     * A probe that could not even be sent proves nothing about the listener and would fail
     * identically on every retry, so it verifies rather than churns.
     */
    async #verify(): Promise<boolean> {
        const arrival = new Promise<boolean>((resolve) => {
            this.#arrival = () => resolve(true);
        });

        try {
            await invoke('probe_audio_levels');
        } catch (e) {
            this.#arrival = null;
            void warn(`LevelFeed: could not request a level probe: ${e}`);
            return true;
        }

        // Timed from the backend accepting the probe, not from before it: the command waits on
        // the stream manager's lock, and a contended lock must not eat the window the snapshot
        // is supposed to arrive in.
        const timeout = new Promise<boolean>((resolve) => {
            setTimeout(() => resolve(false), LevelFeed.PROBE_TIMEOUT_MS);
        });

        const arrived = await Promise.race([arrival, timeout]);
        this.#arrival = null;
        return arrived;
    }

    #deliver(snapshot: LevelSnapshot): void {
        this.#received += 1;
        this.#arrival?.();
        // Each sink is isolated: one throwing must not stop the others being fed, and it must
        // not take the count with it either — a handler that failed and an event that never
        // came are different faults with different fixes.
        for (const sink of this.#sinks) {
            try {
                sink(snapshot);
            } catch (e) {
                void warn(`LevelFeed: a level sink failed: ${e}`);
            }
        }
    }

    /**
     * Let a registration go without letting its teardown throw.
     *
     * Tauri's injected unregister helper reads the page-side entry without a guard, so
     * dropping exactly the phantom this feed exists to replace throws before the backend is
     * told. Contained here so a failed cleanup cannot take the re-registration with it.
     */
    #drop(off: UnlistenFn): void {
        void (async () => off())().catch((e) =>
            warn(`LevelFeed: dropping a level listener failed: ${e}`),
        );
    }

    #close(): void {
        if (this.#unlisten) {
            this.#drop(this.#unlisten);
            this.#unlisten = null;
        }
        this.#received = 0;
    }

    static #delay(ms: number): Promise<void> {
        return new Promise((resolve) => setTimeout(resolve, ms));
    }
}
