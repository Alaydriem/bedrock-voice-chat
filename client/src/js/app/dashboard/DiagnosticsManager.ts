import { invoke } from '@tauri-apps/api/core';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { warn } from '@tauri-apps/plugin-log';
import { type Readable, type Writable, writable } from 'svelte/store';
import type { ConnectionHealth } from '../../bindings/ConnectionHealth';
import type { LinkDiagnosticsSnapshot } from '../../bindings/LinkDiagnosticsSnapshot';

/** What the link is doing, as far as the dashboard is concerned. */
export interface LinkHealth {
    /**
     * False whenever the link is not carrying voice, for any reason.
     *
     * One boolean rather than a status to switch on, because what the dashboard does about it is
     * the same in every case: clear the roster. Cards that survive a dead link assert that those
     * people can hear you, which is the misleading kind of wrong — it keeps somebody talking.
     */
    connected: boolean;
    reconnecting: boolean;
    attempt?: number;
    /** Set when the link will not recover on its own, and why. */
    fatal?: string;
}

/**
 * The link's own account of itself.
 *
 * Arrives once a second while connected, and not at all while disconnected — deliberately,
 * because a snapshot of zeros draws as a flawless link with a 0 ms round trip, which misleads
 * worse than an empty panel does.
 */
export class DiagnosticsManager {
    private readonly snapshotStore: Writable<LinkDiagnosticsSnapshot | null>;
    private readonly healthStore: Writable<LinkHealth>;
    private unlisteners: UnlistenFn[] = [];

    public readonly snapshot: Readable<LinkDiagnosticsSnapshot | null>;
    public readonly health: Readable<LinkHealth>;

    constructor() {
        this.snapshotStore = writable(null);
        // Optimistic until told otherwise: the dashboard only mounts after a connection, and
        // `connection_health` is emitted on change rather than on a clock.
        this.healthStore = writable({ connected: true, reconnecting: false });
        this.snapshot = { subscribe: this.snapshotStore.subscribe };
        this.health = { subscribe: this.healthStore.subscribe };
    }

    async start(): Promise<void> {
        this.stop();

        // Seeded before the first push so the panel opens on real numbers, including the
        // scope's whole history, rather than filling in over the next seventy seconds.
        //
        // Absence is also the health seed, and the reason this is not merely cosmetic: the
        // backend returns nothing precisely when the session is not connected, so a null here is
        // a positive statement that the link is down.
        //
        // Without it the optimistic default stood unchallenged. `connection_health` is emitted on
        // change, and a webview reload leaves the Rust side running with no change left to
        // report — so a dashboard that came up over a dead link showed a full roster of people
        // who could not hear a word, which is the one thing this screen is not allowed to say.
        try {
            const initial = await invoke<LinkDiagnosticsSnapshot | null>('get_link_diagnostics');
            if (initial) this.snapshotStore.set(initial);
            else this.healthStore.set({ connected: false, reconnecting: false });
        } catch (e) {
            warn(`DiagnosticsManager: could not read the initial snapshot: ${e}`);
        }

        const webview = getCurrentWebviewWindow();
        try {
            this.unlisteners.push(
                await webview.listen<LinkDiagnosticsSnapshot>('link_diagnostics', (event) =>
                    this.snapshotStore.set(event.payload),
                ),
            );
            this.unlisteners.push(
                await webview.listen<ConnectionHealth>('connection_health', (event) =>
                    this.healthStore.set(DiagnosticsManager.toHealth(event.payload)),
                ),
            );
        } catch (e) {
            warn(`DiagnosticsManager: could not subscribe: ${e}`);
        }
    }

    /**
     * The wire's status, read off the tag it actually carries.
     *
     * This tested `'Reconnecting' in health`, and `ConnectionHealth` is a *tagged* union —
     * `{ status: "Reconnecting", attempt }`. There is no `Reconnecting` key, so the test was
     * always false and `reconnecting` was permanently false: the reconnect verdict could never
     * fire, and `Disconnected`, `Failed`, `VersionMismatch` and `Unauthorized` all collapsed into
     * a link reading as healthy.
     */
    private static toHealth(health: ConnectionHealth): LinkHealth {
        switch (health?.status) {
            case 'Connected':
                return { connected: true, reconnecting: false };
            case 'Reconnecting':
                // Attempts count from zero on the wire; a verdict that says "attempt 0" reads as
                // a bug rather than as a first try.
                return { connected: false, reconnecting: true, attempt: health.attempt + 1 };
            case 'Disconnected':
                return { connected: false, reconnecting: false };
            case 'Failed':
                return {
                    connected: false,
                    reconnecting: false,
                    fatal: 'The connection failed and will not retry on its own.',
                };
            case 'VersionMismatch':
                return {
                    connected: false,
                    reconnecting: false,
                    fatal: health.client_too_old
                        ? `This client (${health.client_version}) is too old for the server (${health.server_version}).`
                        : `The server (${health.server_version}) is older than this client (${health.client_version}).`,
                };
            case 'Unauthorized':
                return { connected: false, reconnecting: false, fatal: health.reason };
            default:
                // An unrecognised status is not evidence of a broken link, and blanking the
                // roster on one would be a worse failure than ignoring it.
                warn(`DiagnosticsManager: unrecognised connection health ${JSON.stringify(health)}`);
                return { connected: true, reconnecting: false };
        }
    }

    /**
     * Restart every measurement from now.
     *
     * The panel is repainted from the reset snapshot rather than left to the next push, so the
     * button visibly does something within the frame it was pressed in — a second of unchanged
     * cumulative totals reads as a button that did nothing, and this one is pressed precisely
     * when somebody is watching the numbers.
     */
    async reset(): Promise<void> {
        try {
            await invoke('reset_link_diagnostics');
            const fresh = await invoke<LinkDiagnosticsSnapshot | null>('get_link_diagnostics');
            this.snapshotStore.set(fresh ?? null);
        } catch (e) {
            warn(`DiagnosticsManager: could not reset the counters: ${e}`);
        }
    }

    /** The copyable text a support conversation is answered from. */
    async report(): Promise<string> {
        try {
            return await invoke<string>('get_diagnostics_report');
        } catch (e) {
            warn(`DiagnosticsManager: could not render the report: ${e}`);
            return '';
        }
    }

    stop(): void {
        for (const off of this.unlisteners) off();
        this.unlisteners = [];
    }
}
