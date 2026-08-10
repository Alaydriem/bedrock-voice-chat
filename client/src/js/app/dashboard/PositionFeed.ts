import { invoke } from '@tauri-apps/api/core';
import { debug, info, warn } from '@tauri-apps/plugin-log';
import type { PositionSnapshot } from '../../bindings/PositionSnapshot';
import type { WebsocketTicketResponse } from '../../bindings/WebsocketTicketResponse';

export type SnapshotListener = (snapshot: PositionSnapshot) => void;

/**
 * The server's view of who is around you, as a stream of snapshots.
 *
 * Opened from the webview rather than from Rust, which is why the ticket exists at all: a
 * browser socket can present neither a client certificate nor a request header, so identity
 * is exchanged for a ticket over mTLS first and offered here as a subprotocol. A ticket in
 * the URL would land in every access log on the way.
 *
 * One ticket, one socket, held open. Tickets are single-use and issuing one revokes the
 * identity's previous, so two attempts in flight together are not merely wasteful — the
 * second revokes the first's credential, the first is refused at redeem time, and its retry
 * revokes the second's in turn. Nothing about that state converges, and the roster it feeds
 * stays empty throughout. Every await below is therefore fenced against a `stop()` or a newer
 * attempt that overtook it.
 *
 * Each snapshot supersedes the last, so nothing is queued and a late frame is simply stale.
 * Out-of-order frames are dropped on `seq`, because a snapshot that arrives after a newer
 * one would move every card backwards for a tick.
 */
export class PositionFeed {
    private static readonly PROTOCOL = 'bvc.positions.v1';

    /** First delay before redialling a socket lost to the network rather than to design. */
    static readonly BACKOFF_MIN_MS = 3_000;

    /**
     * Ceiling on that delay.
     *
     * The roster is only as fresh as this feed, so the ceiling is well short of the minute a
     * background channel would take — an outage that ends must not leave the screen empty for
     * another one.
     */
    static readonly BACKOFF_MAX_MS = 30_000;

    /**
     * How long a socket may stay in CONNECTING before it is treated as dead.
     *
     * A link discarded mid-session — a lapsed NAT binding, a network handover — does not
     * refuse the connection, it swallows it. The socket reports no error, no close, and no
     * open, and the browser's own timeout is minutes away. Without this the feed sat holding
     * that socket forever, because `connect` will not open a second one alongside it.
     */
    static readonly OPEN_TIMEOUT_MS = 10_000;

    /**
     * Failed attempts before the feed calls itself down.
     *
     * A dropped socket that redials successfully is the feed working, not a fault, and saying
     * so at warn once per attempt buries the breadcrumbs that explain a genuine error in an
     * outage nobody noticed. Three attempts is roughly ten seconds of backoff — long enough
     * that anything still failing is worth a reader's attention.
     */
    static readonly FAILURE_THRESHOLD = 3;

    private readonly server: string;
    private readonly onSnapshot: SnapshotListener;

    private socket: WebSocket | null = null;
    private retry: ReturnType<typeof setTimeout> | null = null;
    private opening: ReturnType<typeof setTimeout> | null = null;
    private backoff = PositionFeed.BACKOFF_MIN_MS;
    private lastSeq = -1;
    private closed = false;

    /** Consecutive failed attempts. Cleared by a socket that reaches the server. */
    private failures = 0;

    /**
     * Whether this outage has already been announced.
     *
     * Holds the pair together: an outage reports itself once, and the recovery that ends it
     * reports itself once. Without it a feed that recovers says nothing, and a reader watching
     * the log cannot tell a feed that came back from one that is still down and has simply
     * stopped complaining.
     */
    private reportedDown = false;

    /**
     * Bumped by every attempt and by `stop()`.
     *
     * An attempt compares this against the value it took on entry, which is how it discovers
     * that it was superseded while suspended on an await — the one thing a bare `closed` flag
     * cannot express, since a feed that is starting again is not closed.
     */
    private generation = 0;

    constructor(server: string, onSnapshot: SnapshotListener) {
        this.server = server;
        this.onSnapshot = onSnapshot;
    }

    async start(): Promise<void> {
        this.closed = false;
        this.backoff = PositionFeed.BACKOFF_MIN_MS;
        this.failures = 0;
        this.reportedDown = false;
        await this.connect();
    }

    private async connect(): Promise<void> {
        if (this.closed || this.socket) return;

        const generation = ++this.generation;
        const current = (): boolean => !this.closed && this.generation === generation;

        // `seq` counts per socket, starting at 1, so it must be forgotten with the socket that
        // issued it. Carrying it across a reconnect made the ordering guard reject every frame
        // of the new socket until its counter passed the old one's high-water mark, so a player
        // walking into range during that window appeared late, or not at all.
        this.lastSeq = -1;

        let ticket: WebsocketTicketResponse;
        try {
            ticket = await invoke<WebsocketTicketResponse>('api_websocket_ticket', {
                server: this.server,
            });
        } catch (e) {
            if (current()) {
                this.report(`could not get a ticket: ${e}`);
                this.scheduleRetry();
            }
            return;
        }

        // Spending a ticket that a newer attempt has already revoked would refuse that attempt
        // rather than this one, so the abandoned request is dropped here instead.
        if (!current()) return;

        const url = `${this.server.replace(/^http/, 'ws').replace(/\/$/, '')}/api/websocket/positions`;

        let socket: WebSocket;
        try {
            // Two subprotocols: the credential and the feed. The server echoes the feed's,
            // because echoing the ticket back would hand it to anything watching the
            // response.
            socket = new WebSocket(url, [`ticket.${ticket.ticket}`, PositionFeed.PROTOCOL]);
        } catch (e) {
            if (current()) {
                this.report(`could not open ${url}: ${e}`);
                this.scheduleRetry();
            }
            return;
        }

        this.socket = socket;

        // One terminal path for a socket that is not going to serve. A failure reports itself
        // as an error, or as a close, or as neither — and retrying from only one of those is
        // what left the feed holding a socket that would never be replaced. Whichever arrives
        // first owns the retry; the rest are already spent.
        let settled = false;
        const fail = (why: string): void => {
            if (settled) return;
            settled = true;
            PositionFeed.detach(socket);
            socket.close();
            // A socket the feed no longer holds was retired by `stop()` or by a newer attempt,
            // and that owner carries the retry and the watchdog with it.
            if (this.socket !== socket) return;
            this.socket = null;
            this.clearOpening();
            if (!current()) return;
            this.report(why);
            this.scheduleRetry();
        };

        this.opening = setTimeout(() => fail('socket never opened'), PositionFeed.OPEN_TIMEOUT_MS);

        const opened = (): void => {
            if (this.socket !== socket) return;
            this.clearOpening();
            // Reset by a socket that reached the server, never by one that was merely tried:
            // a feed that keeps failing must keep slowing down.
            this.backoff = PositionFeed.BACKOFF_MIN_MS;
            this.recovered();
        };

        socket.onopen = () => {
            opened();
            debug('PositionFeed: connected');
        };
        socket.onmessage = (event) => {
            // A socket delivering frames has plainly opened, whether or not `onopen` was seen.
            opened();
            if (current()) this.receive(event.data);
        };
        // The server holds this open, so a close is a lost link rather than a handover.
        socket.onclose = () => fail('socket closed');
        socket.onerror = () => fail('socket error');
    }

    /**
     * Records one failed attempt, and announces an outage at most once.
     *
     * Every attempt is kept at debug, where it costs a breadcrumb and nothing else. Only a run
     * of them is worth a warn, because a feed that redials successfully is the feed working:
     * reporting each attempt at warn spends the breadcrumb budget that a genuine error would
     * have been read through, and does it during the outage least likely to matter.
     */
    private report(why: string): void {
        this.failures += 1;
        debug(`PositionFeed: ${why} (attempt ${this.failures})`);

        if (this.reportedDown || this.failures < PositionFeed.FAILURE_THRESHOLD) {
            return;
        }
        this.reportedDown = true;
        warn(`PositionFeed: down after ${this.failures} attempts, still retrying`);
    }

    /**
     * Closes out an outage that was announced.
     *
     * Silent unless something was reported, so an ordinary redial stays quiet — the message
     * exists to end the warn above, and one without the other is what leaves a reader unable
     * to tell a recovered feed from a resigned one.
     */
    private recovered(): void {
        const attempts = this.failures;
        this.failures = 0;
        if (!this.reportedDown) {
            return;
        }
        this.reportedDown = false;
        info(`PositionFeed: reconnected after ${attempts} failed attempts`);
    }

    private clearOpening(): void {
        if (this.opening) {
            clearTimeout(this.opening);
            this.opening = null;
        }
    }

    /**
     * Silence a socket the feed has finished with.
     *
     * All four, not just `onclose`: a retired socket that still delivers a message would be
     * treated as proof that the attempt replacing it had reached the server.
     */
    private static detach(socket: WebSocket): void {
        socket.onopen = null;
        socket.onmessage = null;
        socket.onclose = null;
        socket.onerror = null;
    }

    private receive(data: unknown): void {
        if (typeof data !== 'string') return;
        let snapshot: PositionSnapshot;
        try {
            snapshot = JSON.parse(data) as PositionSnapshot;
        } catch {
            return;
        }

        // `seq` is a bigint through ts-rs, so it is compared as a Number rather than mixed
        // with one — arithmetic across the two throws at runtime.
        const seq = Number(snapshot.seq);
        if (seq <= this.lastSeq) return;
        this.lastSeq = seq;

        this.onSnapshot(snapshot);
    }

    private scheduleRetry(): void {
        if (this.closed || this.retry) return;

        // Jitter is load-bearing rather than tidy: a server restart drops every client's feed
        // in the same instant, and a fixed delay has all of them buy a ticket and redial
        // together, against a server that has only just come up.
        const delay = this.backoff + Math.random() * PositionFeed.BACKOFF_MIN_MS;
        this.backoff = Math.min(this.backoff * 2, PositionFeed.BACKOFF_MAX_MS);

        this.retry = setTimeout(() => {
            this.retry = null;
            // A fresh ticket every time: they are single-use and expire within the minute.
            void this.connect();
        }, delay);
    }

    stop(): void {
        this.closed = true;
        // Retires any attempt still suspended on an await, so it cannot open a socket that
        // nothing owns or spend a ticket against the feed that replaces this one.
        this.generation++;
        this.clearOpening();
        if (this.retry) {
            clearTimeout(this.retry);
            this.retry = null;
        }
        if (this.socket) {
            PositionFeed.detach(this.socket);
            this.socket.close();
            this.socket = null;
        }
        this.lastSeq = -1;
    }
}
