import { invoke } from '@tauri-apps/api/core';
import { error as logError, info } from '@tauri-apps/plugin-log';
import { get, writable, type Readable, type Writable } from 'svelte/store';
import { ServerHealthService } from '../services/ServerHealthService';
import { ServerListStore } from '../services/ServerListStore';
import type { NextAction } from '../shell/NextAction';
import { RosterRowView } from './RosterRowView';
import type { ServerRosterDeps } from './ServerRosterDeps';
import type { ServerRosterEntry } from './ServerRosterEntry';

/**
 * Every saved server, and what each one is worth right now.
 *
 * Rows appear before their checks finish and settle one at a time, because the alternative
 * is an empty screen for as long as the slowest server takes to time out — and the server
 * someone wants is usually not that one.
 *
 * Choosing a row is returned as a destination rather than performed, so which row leads
 * where is testable without a browser.
 */
export class ServerRosterManager {
    static readonly ADD_HREF = '/login?addserver=true&return=/server';
    static readonly SIGN_IN_HREF = '/login';

    private readonly deps: ServerRosterDeps;

    private readonly entriesStore: Writable<ServerRosterEntry[]>;
    private readonly isRefreshingStore: Writable<boolean>;

    public readonly entries: Readable<ServerRosterEntry[]>;
    public readonly isRefreshing: Readable<boolean>;

    constructor(deps?: Partial<ServerRosterDeps>) {
        this.deps = {
            health: deps?.health ?? new ServerHealthService(),
            serverList: deps?.serverList ?? new ServerListStore(),
            forgetCredentials:
                deps?.forgetCredentials ??
                ((server: string) => invoke<void>('delete_credentials', { server })),
            checkForUpdates:
                deps?.checkForUpdates ?? (() => invoke<string | null>('check_for_updates')),
        };

        this.entriesStore = writable<ServerRosterEntry[]>([]);
        this.isRefreshingStore = writable(false);
        this.entries = { subscribe: this.entriesStore.subscribe };
        this.isRefreshing = { subscribe: this.isRefreshingStore.subscribe };
    }

    /**
     * Draw the list, unchecked, and report how many rows there are.
     *
     * Checking is a separate step so the caller decides whether to wait for it. One saved
     * server is worth waiting on, because the answer may be to skip this screen entirely;
     * several are not, because the list is the destination either way.
     */
    async load(): Promise<number> {
        const [saved, current] = await Promise.all([
            this.deps.serverList.getServerList(),
            this.deps.serverList.getCurrentServer(),
        ]);

        this.entriesStore.set(
            saved.map((entry) => ({
                server: entry.server,
                host: ServerRosterManager.hostOf(entry.server),
                player: entry.player,
                game: entry.game ?? 'minecraft',
                status: 'checking' as const,
                serverVersion: '',
                clientVersion: '',
                clientTooOld: false,
                isCurrent: entry.server === current,
            })),
        );

        return saved.length;
    }

    async refreshAll(): Promise<void> {
        this.isRefreshingStore.set(true);
        try {
            await this.load();
            await this.sweep();
        } finally {
            this.isRefreshingStore.set(false);
        }
    }

    /**
     * Check all servers at once. One slow host must not hold up the rest, so each row is
     * published as its own answer lands rather than all of them at the end.
     */
    async sweep(): Promise<void> {
        const servers = get(this.entriesStore).map((entry) => entry.server);
        await Promise.all(servers.map((server) => this.checkOne(server)));
    }

    /** The rows as they stand, for a caller deciding what to do after a sweep. */
    current(): readonly ServerRosterEntry[] {
        return get(this.entriesStore);
    }

    /** Re-check one server, for a row whose answer was "not answering". */
    async recheck(server: string): Promise<void> {
        this.patch(server, { status: 'checking', note: undefined });
        await this.checkOne(server);
    }

    /**
     * Where this row leads.
     *
     * A row that cannot lead anywhere returns `none` and says why on the row itself,
     * because a button that navigates nowhere is worse than one that explains.
     */
    async choose(server: string): Promise<NextAction> {
        const entry = this.find(server);
        if (!entry) return { kind: 'none' };

        switch (entry.status) {
            case 'connect':
                await this.deps.serverList.setCurrent({
                    server: entry.server,
                    player: entry.player,
                    game: entry.game,
                });
                return { kind: 'navigate', href: `/dashboard?server=${entry.server}` };

            case 'reauth':
            case 'missing':
                return {
                    kind: 'navigate',
                    href: `/login?reauth=true&server=${entry.server}`,
                };

            case 'version_mismatch':
                return entry.clientTooOld ? this.offerUpdate(server) : { kind: 'none' };

            case 'unreachable':
                await this.recheck(server);
                return { kind: 'none' };

            case 'checking':
                return { kind: 'none' };
        }
    }

    /**
     * Forget a server.
     *
     * The credentials go first: a list entry with no credentials behind it is a row that
     * offers a sign-in, while credentials with no row are invisible and stay on the device.
     */
    async remove(server: string): Promise<NextAction> {
        try {
            await this.deps.forgetCredentials(server).catch((e) => {
                info(`Could not clear credentials for ${server}: ${e}`);
            });

            const remaining = await this.deps.serverList.removeServer(server);
            this.entriesStore.update((entries) => entries.filter((e) => e.server !== server));

            if (remaining.length === 0) {
                info('Last server removed, returning to sign-in');
                return { kind: 'navigate', href: ServerRosterManager.SIGN_IN_HREF };
            }
            return { kind: 'none' };
        } catch (e) {
            logError(`Failed to remove ${server}: ${e}`);
            this.patch(server, { note: 'That server could not be removed. Try again.' });
            return { kind: 'none' };
        }
    }

    /**
     * The server to join without asking, or null to show the list.
     *
     * One saved server that is ready is not a choice, so it is not presented as one.
     * Anything else about that server — a lapsed sign-in, a protocol it cannot speak — is
     * something to be told, and the list is where that is said.
     */
    static autoJoin(entries: readonly ServerRosterEntry[]): string | null {
        if (entries.length !== 1) return null;
        return RosterRowView.isJoinable(entries[0]) ? entries[0].server : null;
    }

    static hostOf(server: string): string {
        return server.replace(/^https?:\/\//, '').replace(/\/$/, '');
    }

    private async offerUpdate(server: string): Promise<NextAction> {
        try {
            const version = await this.deps.checkForUpdates();
            if (version) {
                return {
                    kind: 'navigate',
                    href: `/error?code=UPD01&version=${encodeURIComponent(version)}`,
                };
            }
            this.patch(server, {
                note: 'No update has been published yet. That server is ahead of every build of the app.',
            });
        } catch (e) {
            info(`No updater available: ${e}`);
            this.patch(server, {
                note: 'Updates are installed from wherever you got the app.',
            });
        }
        return { kind: 'none' };
    }

    private async checkOne(server: string): Promise<void> {
        const result = await this.deps.health.check(server);
        this.patch(server, {
            status: result.status,
            serverVersion: result.serverVersion,
            clientVersion: result.clientVersion,
            clientTooOld: result.clientTooOld,
        });
    }

    private find(server: string): ServerRosterEntry | undefined {
        return get(this.entriesStore).find((entry) => entry.server === server);
    }

    private patch(server: string, changes: Partial<ServerRosterEntry>): void {
        this.entriesStore.update((entries) =>
            entries.map((entry) => (entry.server === server ? { ...entry, ...changes } : entry)),
        );
    }
}
