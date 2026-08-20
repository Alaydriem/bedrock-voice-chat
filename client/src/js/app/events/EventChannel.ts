import { invoke } from '@tauri-apps/api/core';
import { info, warn } from '@tauri-apps/plugin-log';
import type { InternalEndpoint } from '../../bindings/InternalEndpoint';

export type EventSink<T> = (data: T) => void;

interface Envelope {
    type?: string;
    data?: unknown;
}

/**
 * The one socket to this process's own push listener, fanned out by frame type.
 *
 * This replaces `listen()` for everything the dashboard renders continuously. Registering a
 * Tauri listener ends with an `eval` into the page that writes the listener id where the
 * dispatcher looks it up, and on Android that `eval` can be lost: the promise still resolves,
 * the backend keeps dispatching to an id the page silently skips, and nothing inside the page
 * can see the missing entry. A socket has no such registry — it is either open or it is not,
 * and `readyState` says which.
 *
 * The endpoint comes from `invoke`, deliberately. `invoke` is the mechanism that keeps working
 * when the listener registry is lost, so discovering this channel through an event would
 * reintroduce the fault it exists to escape.
 *
 * Every payload here is absolute state — current levels, current diagnostics, current health,
 * never a delta. That is what makes a reconnect a full correction rather than a resync, and it
 * is a constraint on what may ever be added: a delta would be silently lost across a gap.
 */
export class EventChannel {
    static #shared: EventChannel | null = null;

    /** First reconnect delay, doubled per attempt. */
    static readonly RETRY_BASE_MS = 250;

    /** Ceiling on the reconnect delay, so a long outage still recovers promptly. */
    static readonly RETRY_MAX_MS = 10_000;

    /**
     * How long the channel may carry nothing at all before it is presumed dead.
     *
     * The listener sends a keepalive every five seconds whether or not anything happened, so
     * silence for three of them is not a quiet room — it is a socket that stopped delivering
     * without saying so. Android does exactly that to a suspended socket: `readyState` stays 1,
     * no close event ever arrives, and nothing else in the page can tell.
     */
    static readonly LIVENESS_MS = 15_000;

    /** How often that deadline is checked. */
    static readonly WATCHDOG_MS = 5_000;

    /** How often an unclaimed frame kind is worth a log line. */
    static readonly UNCLAIMED_LOG_MS = 10_000;

    /** The heartbeat the listener sends whether or not anything happened. */
    static readonly KEEPALIVE_KIND = 'keepalive';

    #sinks = new Map<string, Set<EventSink<never>>>();
    #socket: WebSocket | null = null;
    #timer: ReturnType<typeof setTimeout> | null = null;
    #attempts = 0;
    #opening = false;
    #lastFrameAt: number | null = null;
    // Distinct from #lastFrameAt, which is a readout. This is the deadline's baseline, and an
    // open socket that has not delivered yet has to count from the open rather than from never.
    #aliveAt: number | null = null;
    #watchdog: ReturnType<typeof setInterval> | null = null;
    #unclaimedAt = new Map<string, number>();
    #visibility: (() => void) | null = null;

    static shared(): EventChannel {
        EventChannel.#shared ??= new EventChannel();
        return EventChannel.#shared;
    }

    get connected(): boolean {
        return this.#socket !== null && this.#socket.readyState === 1;
    }

    /** How long since any frame arrived, for the panel row that reports this channel. */
    get lastFrameAgoMs(): number | null {
        return this.#lastFrameAt === null ? null : Date.now() - this.#lastFrameAt;
    }

    /** Reconnect attempts since the last successful open. */
    get attempts(): number {
        return this.#attempts;
    }

    /**
     * Take frames of one type until the returned function is called.
     *
     * The socket opens on the first subscriber and closes after the last one leaves, so a screen
     * that is gone stops costing a delivery.
     */
    subscribe<T>(kind: string, sink: EventSink<T>): () => void {
        let sinks = this.#sinks.get(kind);
        if (!sinks) {
            sinks = new Set();
            this.#sinks.set(kind, sinks);
        }
        sinks.add(sink as EventSink<never>);
        this.#watchVisibility();
        this.#watchLiveness();
        void this.#open();

        let released = false;
        return () => {
            if (released) return;
            released = true;
            // The registered Set is looked up now rather than trusted from the closure. A
            // released handle calling again would otherwise consult its own emptied Set, find
            // it at zero, and delete whichever registration has since replaced it — every later
            // frame of that kind dropped on the floor while its subscriber still holds a handle
            // and still reports itself attached. One kind dies permanently, the rest carry on,
            // and nothing anywhere says so.
            if (this.#sinks.get(kind) !== sinks) return;
            sinks.delete(sink as EventSink<never>);
            if (sinks.size === 0) this.#sinks.delete(kind);
            if (this.#audience === 0) this.#close();
        };
    }

    /** Drop everything, for a test that needs a fresh instance of a shared object. */
    resetForTest(): void {
        this.#close();
        this.#sinks.clear();
        this.#attempts = 0;
        this.#lastFrameAt = null;
        this.#aliveAt = null;
    }

    get #audience(): number {
        let total = 0;
        for (const sinks of this.#sinks.values()) total += sinks.size;
        return total;
    }

    /**
     * Re-open when the window comes back.
     *
     * Android suspends sockets in the background, so returning to the app is the known moment
     * this channel is found dead. It is the failure mode that replaces the one being removed,
     * and the only one that recovers on a user action rather than a timer.
     */
    #watchVisibility(): void {
        if (this.#visibility !== null || typeof document === 'undefined') return;
        this.#visibility = () => {
            if (document.visibilityState !== 'visible') return;
            if (this.#audience === 0 || this.connected || this.#opening) return;
            void this.#open();
        };
        document.addEventListener('visibilitychange', this.#visibility);
    }

    /**
     * Replace a socket that is open and carrying nothing.
     *
     * The reconnect on `onclose` covers a socket that admits it died. This covers the one that
     * does not — and that is the case the meters are now built on, so it cannot be left to a
     * user returning to the app to notice.
     */
    #watchLiveness(): void {
        if (this.#watchdog !== null || typeof setInterval === 'undefined') return;
        this.#watchdog = setInterval(() => {
            if (this.#audience === 0 || !this.connected) return;
            const since = this.#aliveAt;
            if (since === null || Date.now() - since <= EventChannel.LIVENESS_MS) return;
            this.#forceReopen('the push channel went silent');
        }, EventChannel.WATCHDOG_MS);
    }

    async #open(): Promise<void> {
        if (this.#opening || this.connected || this.#audience === 0) return;
        this.#opening = true;

        let endpoint: InternalEndpoint;
        try {
            endpoint = await invoke<InternalEndpoint>('websocket_internal_endpoint');
        } catch (e) {
            this.#opening = false;
            // Expected while the listener is still binding at startup, so this retries rather
            // than giving up.
            this.#scheduleRetry(`the push endpoint is not ready: ${e}`);
            return;
        }

        // The literal loopback address. `[::1]` is refused on both Android and iPadOS, and
        // `localhost` only adds a name lookup.
        const url = `ws://127.0.0.1:${endpoint.port}/events?key=${encodeURIComponent(endpoint.token)}`;

        let socket: WebSocket;
        try {
            socket = new WebSocket(url);
        } catch (e) {
            this.#opening = false;
            this.#scheduleRetry(`could not construct the push socket: ${e}`);
            return;
        }

        this.#socket = socket;
        this.#opening = false;

        socket.onopen = () => {
            this.#attempts = 0;
            this.#aliveAt = Date.now();
            void info('EventChannel: push channel open');
        };
        socket.onmessage = (event) => this.#deliver(event.data);
        // A webview exposes no reason for either, so both are treated as a dropped socket and
        // answered the same way.
        socket.onerror = () => this.#lost(socket);
        socket.onclose = () => this.#lost(socket);
    }

    #deliver(raw: unknown): void {
        this.#lastFrameAt = Date.now();
        this.#aliveAt = this.#lastFrameAt;

        let envelope: Envelope;
        try {
            envelope = JSON.parse(String(raw)) as Envelope;
        } catch (e) {
            void warn(`EventChannel: could not parse a frame: ${e}`);
            return;
        }
        if (typeof envelope.type !== 'string') return;

        const sinks = this.#sinks.get(envelope.type);
        if (!sinks || sinks.size === 0) {
            this.#unclaimed(envelope.type);
            return;
        }

        for (const sink of sinks) {
            try {
                (sink as EventSink<unknown>)(envelope.data);
            } catch (e) {
                void warn(`EventChannel: a sink for ${envelope.type} failed: ${e}`);
            }
        }
    }

    /**
     * Frames of a kind nobody is taking.
     *
     * Otherwise invisible, and the most misleading state this channel has: the socket is open,
     * frames are arriving on time, every count on the diagnostics panel looks healthy, and the
     * meter that kind feeds sits at zero — which is exactly what a quiet room looks like. Logged
     * once per kind per window so a 5/s stream does not bury the rest of the log.
     */
    #unclaimed(kind: string): void {
        // The keepalive is consumed here rather than by a sink: its arrival is what refreshes
        // the liveness deadline, so nothing subscribes to it and its absence from the map is
        // the design rather than a fault.
        if (kind === EventChannel.KEEPALIVE_KIND) return;
        const last = this.#unclaimedAt.get(kind) ?? 0;
        const now = Date.now();
        if (now - last < EventChannel.UNCLAIMED_LOG_MS) return;
        this.#unclaimedAt.set(kind, now);
        void warn(`EventChannel: ${kind} frames are arriving with nothing subscribed to them`);
    }

    /** A socket that is no longer ours must not be able to schedule a retry over a live one. */
    #lost(socket: WebSocket): void {
        if (this.#socket !== socket) return;
        this.#socket = null;
        if (this.#audience === 0) return;
        this.#scheduleRetry('the push channel closed');
    }

    #scheduleRetry(reason: string): void {
        if (this.#timer !== null) return;
        const delay = Math.min(
            EventChannel.RETRY_BASE_MS * 2 ** this.#attempts,
            EventChannel.RETRY_MAX_MS,
        );
        this.#attempts += 1;
        void warn(`EventChannel: ${reason}; retrying in ${delay}ms`);
        this.#timer = setTimeout(() => {
            this.#timer = null;
            void this.#open();
        }, delay);
    }

    /** Throw this socket away and open another, with the same backoff a close would get. */
    #forceReopen(reason: string): void {
        const socket = this.#socket;
        this.#socket = null;
        this.#discard(socket);
        this.#scheduleRetry(reason);
    }

    #close(): void {
        if (this.#timer !== null) {
            clearTimeout(this.#timer);
            this.#timer = null;
        }
        if (this.#watchdog !== null) {
            clearInterval(this.#watchdog);
            this.#watchdog = null;
        }
        const socket = this.#socket;
        this.#socket = null;
        this.#discard(socket);
    }

    /**
     * Detach before closing.
     *
     * A socket still holding this channel's handlers can deliver after it has been replaced, and
     * a frame from a socket nobody owns would refresh the liveness deadline the replacement is
     * being judged by.
     */
    #discard(socket: WebSocket | null): void {
        if (!socket) return;
        socket.onopen = null;
        socket.onmessage = null;
        socket.onerror = null;
        socket.onclose = null;
        try {
            socket.close();
        } catch {
            // A socket that never opened throws on some engines; it is going away regardless.
        }
    }
}
