import { invoke } from '@tauri-apps/api/core';
import { debug, warn } from '@tauri-apps/plugin-log';
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

    /** Backoff between reconnects, for a socket lost to the network rather than to design. */
    private static readonly RETRY_MS = 3_000;

    private readonly server: string;
    private readonly onSnapshot: SnapshotListener;

    private socket: WebSocket | null = null;
    private retry: ReturnType<typeof setTimeout> | null = null;
    private lastSeq = -1;
    private closed = false;

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
            warn(`PositionFeed: could not get a ticket: ${e}`);
            if (current()) this.scheduleRetry();
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
            warn(`PositionFeed: could not open ${url}: ${e}`);
            if (current()) this.scheduleRetry();
            return;
        }

        this.socket = socket;

        socket.onmessage = (event) => {
            if (current()) this.receive(event.data);
        };
        socket.onclose = () => {
            if (this.socket === socket) this.socket = null;
            if (!current()) return;
            // The server holds this open, so a close is a lost link rather than a handover.
            debug('PositionFeed: socket closed');
            this.scheduleRetry();
        };
        socket.onerror = () => warn('PositionFeed: socket error');
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
        this.retry = setTimeout(() => {
            this.retry = null;
            // A fresh ticket every time: they are single-use and expire within the minute.
            void this.connect();
        }, PositionFeed.RETRY_MS);
    }

    stop(): void {
        this.closed = true;
        // Retires any attempt still suspended on an await, so it cannot open a socket that
        // nothing owns or spend a ticket against the feed that replaces this one.
        this.generation++;
        if (this.retry) {
            clearTimeout(this.retry);
            this.retry = null;
        }
        if (this.socket) {
            this.socket.onclose = null;
            this.socket.close();
            this.socket = null;
        }
        this.lastSeq = -1;
    }
}
