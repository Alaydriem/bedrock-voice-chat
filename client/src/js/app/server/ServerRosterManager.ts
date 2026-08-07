import { invoke } from '@tauri-apps/api/core';
import { error as logError, info, warn } from '@tauri-apps/plugin-log';
import type { LoginResponse } from '../../bindings/LoginResponse';
import { get, writable, type Readable, type Writable } from 'svelte/store';
import ImageCache from '../components/imageCache';
import ImageCacheOptions from '../components/imageCacheOptions';
import { AppStore } from '../services/AppStore';
import { ServerListStore } from '../services/ServerListStore';
import SetupFlow from '../setup/SetupFlow';
import type { SetupState } from '../../bindings/SetupState';
import { BootTimeline } from '../shell/BootTimeline';
import type { NextAction } from '../shell/NextAction';
import { PlateView } from './PlateView';
import { PreflightRunner } from './preflight/PreflightRunner';
import type { ServerRosterDeps } from './ServerRosterDeps';
import type { ServerRosterEntry } from './ServerRosterEntry';

/**
 * Every saved server, and whether voice will actually work on it.
 *
 * Plates appear before their preflights finish and settle one at a time, because the
 * alternative is an empty screen for as long as the slowest server takes to time out — and
 * the server someone wants is usually not that one.
 *
 * Choosing a plate is returned as a destination rather than performed, so which plate leads
 * where is testable without a browser.
 */
export class ServerRosterManager {
    static readonly SIGN_IN_HREF = '/login';
    static readonly SETUP_HREF = '/setup';

    /** A week, matching what the old card used. Operator art does not change often. */
    private static readonly ART_TTL_SECONDS = 60 * 60 * 24 * 7;

    private readonly deps: ServerRosterDeps;

    private readonly entriesStore: Writable<ServerRosterEntry[]>;
    private readonly isRefreshingStore: Writable<boolean>;

    public readonly entries: Readable<ServerRosterEntry[]>;
    public readonly isRefreshing: Readable<boolean>;

    constructor(deps?: Partial<ServerRosterDeps>) {
        this.deps = {
            serverList: deps?.serverList ?? new ServerListStore(),
            preflight:
                deps?.preflight ??
                ((server, observer) => new PreflightRunner(observer).run(server)),
            imageCache: deps?.imageCache ?? new ImageCache(),
            forgetCredentials:
                deps?.forgetCredentials ??
                ((server: string) => invoke<void>('delete_credentials', { server })),
            checkForUpdates:
                deps?.checkForUpdates ?? (() => invoke<string | null>('check_for_updates')),
            credentials:
                deps?.credentials ??
                ((server: string) => invoke<LoginResponse>('get_credentials', { server })),
            isCertificateExpired:
                deps?.isCertificateExpired ??
                ((server: string) => invoke<boolean>('is_certificate_expired', { server })),
            isSetupComplete:
                deps?.isSetupComplete ??
                (async () => {
                    const store = await AppStore.load();
                    const state = await store.get<SetupState>(SetupFlow.STORE_KEY);
                    const flow = new SetupFlow();
                    if (state) flow.hydrate(state);
                    return flow.isComplete();
                }),
        };

        this.entriesStore = writable<ServerRosterEntry[]>([]);
        this.isRefreshingStore = writable(false);
        this.entries = { subscribe: this.entriesStore.subscribe };
        this.isRefreshing = { subscribe: this.isRefreshingStore.subscribe };
    }

    /**
     * Draw the plates, unchecked, and report how many there are.
     *
     * Preflighting is a separate step so the caller decides whether to wait for it. One
     * saved server is worth waiting on, because the answer may be to skip this screen
     * entirely; several are not, because the list is the destination either way.
     */
    async load(): Promise<number> {
        const saved = await this.deps.serverList.getServerList();
        BootTimeline.shared().mark('  ↳ server list read');

        this.entriesStore.set(
            saved.map((entry) => ({
                server: entry.server,
                host: ServerRosterManager.hostOf(entry.server),
                player: entry.player,
                game: entry.game ?? 'minecraft',
                status: 'checking' as const,
                steps: PreflightRunner.pending(),
                rtt: 0,
                slow: false,
                quicPort: 443,
                serverVersion: '',
                clientVersion: '',
                clientTooOld: false,
                avatarUrl: '',
                canvasUrl: '',
            })),
        );

        // Art is decorative and slow, so it never gates a plate. A fetch that fails leaves
        // the derived glyph and hue in place, which is the case that always works.
        for (const entry of saved) void this.loadArt(entry.server);

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
     * Preflight every server at once. Sequential within one server, concurrent across them,
     * which is why the plates resolve in a ragged order rather than top to bottom.
     */
    async sweep(): Promise<void> {
        const servers = get(this.entriesStore).map((entry) => entry.server);
        await Promise.all(servers.map((server) => this.checkOne(server)));
    }

    /** The plates as they stand, for a caller deciding what to do after a sweep. */
    current(): readonly ServerRosterEntry[] {
        return get(this.entriesStore);
    }

    /** Re-run one server's preflight, leaving the others alone. */
    async recheck(server: string): Promise<void> {
        this.patch(server, {
            status: 'checking',
            steps: PreflightRunner.pending(),
            note: undefined,
        });
        await this.checkOne(server);
    }

    /**
     * Where this plate leads.
     *
     * A plate that cannot lead anywhere returns `none` and says why on itself, because a
     * button that navigates nowhere is worse than one that explains.
     */
    async choose(server: string): Promise<NextAction> {
        const entry = this.find(server);
        if (!entry) return { kind: 'none' };

        switch (PlateView.of(entry).kind) {
            case 'connect':
                return this.connectTo(entry);

            case 'signin':
                return { kind: 'navigate', href: `/login?reauth=true&server=${entry.server}` };

            case 'recheck':
                await this.recheck(server);
                return { kind: 'none' };

            case 'blocked':
                return entry.status === 'version_mismatch' && entry.clientTooOld
                    ? this.offerUpdate(server)
                    : { kind: 'none' };
        }
    }

    /**
     * Forget a server.
     *
     * The credentials go first: a list entry with no credentials behind it is a plate that
     * offers a sign-in, while credentials with no plate are invisible and stay on the device.
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
     * Record this server as current and hand back the dashboard.
     *
     * Shared with `choose`, because a plate that leads to the dashboard and a sole server that
     * leads there have to record the same thing — a divergence here shows up as the dashboard
     * opening the previous server.
     */
    private async connectTo(
        entry: ServerRosterEntry,
        setupComplete?: boolean,
    ): Promise<NextAction> {
        await this.deps.serverList.setCurrent({
            server: entry.server,
            player: entry.player,
            game: entry.game,
        });

        // Taken from the caller when it already asked, so the launch path does not pay a second
        // round trip for an answer it has.
        const complete = setupComplete ?? (await this.deps.isSetupComplete());

        // Recorded first, because setup hands off to the dashboard when it finishes and the
        // dashboard opens whichever server is current.
        //
        // Only here, where the destination is already known to be the dashboard. A launch with
        // nothing saved goes to sign-in, which is where the introduction lives, and setup is a
        // device concern that cannot come before an account.
        if (!complete) {
            info(`Device setup is not finished, going to setup before ${entry.server}`);
            return { kind: 'navigate', href: ServerRosterManager.SETUP_HREF };
        }

        return { kind: 'navigate', href: `/dashboard?server=${entry.server}` };
    }

    /**
     * Where a single saved server leads, decided without touching the network.
     *
     * Only the checks the dashboard cannot recover from are made here. Absent credentials throw
     * inside its own initialize with no redirect, and an expired certificate has to be reissued
     * before anything else is worth attempting. Everything else the preflight would have
     * reported — whether the server answers, whether UDP gets through — the connect measures
     * for itself and reports through its own error path.
     */
    async soleDestination(): Promise<NextAction> {
        const entries = get(this.entriesStore);
        if (entries.length !== 1) return { kind: 'none' };

        const entry = entries[0];

        // Asked together rather than in turn, so the launch pays one round trip rather than three.
        //
        // `allSettled`, not `all`: absent credentials, an unreadable expiry and an unreadable
        // setup state mean different things, and `all` loses the distinction.
        const [credentials, expired, setup] = await Promise.allSettled([
            this.deps.credentials(entry.server),
            this.deps.isCertificateExpired(entry.server),
            this.deps.isSetupComplete(),
        ]);

        if (credentials.status === 'rejected') {
            info(
                `No usable credentials for ${entry.server}, going to sign-in: ${credentials.reason}`,
            );
            return { kind: 'navigate', href: ServerRosterManager.SIGN_IN_HREF };
        }

        if (expired.status === 'rejected') {
            // An unreadable expiry is not an expired certificate. The dashboard checks this
            // again and does redirect on expiry, so guessing wrong here would send somebody to
            // re-authenticate over a keyring hiccup.
            warn(`Could not read certificate expiry for ${entry.server}: ${expired.reason}`);
        } else if (expired.value) {
            info(`Certificate expired for ${entry.server}, going to reauth`);
            return {
                kind: 'navigate',
                href: `/login?reauth=true&server=${entry.server}`,
            };
        }

        // An unreadable setup state counts as finished. The dashboard checks it again, and a
        // store hiccup should not divert somebody into onboarding they have already done.
        if (setup.status === 'rejected') {
            warn(`Could not read the setup state: ${setup.reason}`);
        }

        return this.connectTo(entry, setup.status === 'fulfilled' ? setup.value : true);
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
        const outcome = await this.deps.preflight(server, (steps) => {
            this.patch(server, { steps });
        });
        this.patch(server, outcome);
    }

    /**
     * `avatar.png` and `canvas.png`, either of which can be absent. `getImage` answers with
     * an empty string rather than throwing, so an absent asset and a failed fetch arrive the
     * same way — which is what the derived fallback is for.
     */
    private async loadArt(server: string): Promise<void> {
        const ttl = ServerRosterManager.ART_TTL_SECONDS;
        const [avatarUrl, canvasUrl] = await Promise.all([
            this.deps.imageCache
                .getImage(new ImageCacheOptions(`${server}/assets/avatar.png`, ttl))
                .catch(() => ''),
            this.deps.imageCache
                .getImage(new ImageCacheOptions(`${server}/assets/canvas.png`, ttl))
                .catch(() => ''),
        ]);
        this.patch(server, { avatarUrl, canvasUrl });
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
