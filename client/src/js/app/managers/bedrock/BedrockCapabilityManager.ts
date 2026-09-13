import { get, writable, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@charlesportwoodii/tauri-plugin-curia';
import type { ApiConfigCheckResponse } from '../../../bindings/ApiConfigCheckResponse';
import type { ProxyServerEntry } from './ProxyServerEntry';
import { AppStore } from "../../services/AppStore";

export type BedrockCapabilityStatus = 'enabled' | 'disabled' | 'unknown';

// How long to wait before automatically re-checking after a failed capability
// fetch. Failures are usually transient (server restart, breaker open), so the
// page self-heals without a manual retry.
const RETRY_INTERVAL_MS = 30_000;

// Minimum spacing between focus-triggered refreshes so rapid window focus
// flips don't spam the config endpoint.
const FOCUS_REFRESH_MIN_INTERVAL_MS = 5_000;

// Where the last good advertised list is kept. One slot, stamped with the BVC server it
// came from: restoring one server's worlds onto another would offer worlds that server
// never named, and only the current server's list is ever read.
const SERVER_PROVIDED_KEY = 'bedrock_server_provided';

interface CachedServerList {
    host: string;
    entries: ProxyServerEntry[];
}

// Resolves whether the connected BVC server supports Bedrock features
// (`/api/config` -> bedrock.enabled) and carries the operator-curated proxy
// server list. Distinguishes "server says disabled" from "could not ask"
// (`unknown`): the two get different UI, and `unknown` re-checks itself.
export class BedrockCapabilityManager {
    private statusStore: Writable<BedrockCapabilityStatus | null>;
    public readonly status: Readable<BedrockCapabilityStatus | null>;
    private serverProvidedStore: Writable<ProxyServerEntry[]>;
    public readonly serverProvidedServers: Readable<ProxyServerEntry[]>;
    // Hostname of the connected BVC server, shown on the unsupported-server
    // notice so it's unambiguous which server lacks support.
    private serverHostStore: Writable<string>;
    public readonly serverHost: Readable<string>;
    // True while a capability check is in flight, so re-check affordances can
    // show progress and confirm the check actually ran.
    private checkingStore: Writable<boolean>;
    public readonly isChecking: Readable<boolean>;
    private retryTimer: ReturnType<typeof setTimeout> | null = null;
    private focusHandler: (() => void) | null = null;
    private lastRefreshMs = 0;
    private destroyed = false;
    // The store is read once. After that the list in memory is the newer answer, and
    // re-seeding from the cache would undo a successful check that reported fewer servers.
    private restored = false;
    // Set once a check has answered. The cache is a stand-in for an answer, never a
    // replacement for one: the store read is slow enough on Android to resolve after a
    // request that started later, and seeding then would put a stale list over a live one.
    private answered = false;
    // The check in flight, shared by every caller. `initialize`, the focus handler and the
    // retry timer all call `refresh`, and concurrent runs used to race each other's writes
    // — the later-resolving one winning, so a slow failure could report `unknown` over a
    // successful answer.
    private inFlight: Promise<void> | null = null;

    constructor() {
        this.statusStore = writable(null);
        this.status = { subscribe: this.statusStore.subscribe };
        this.serverProvidedStore = writable([]);
        this.serverProvidedServers = { subscribe: this.serverProvidedStore.subscribe };
        this.serverHostStore = writable('');
        this.serverHost = { subscribe: this.serverHostStore.subscribe };
        this.checkingStore = writable(false);
        this.isChecking = { subscribe: this.checkingStore.subscribe };
    }

    async refresh(): Promise<void> {
        this.inFlight ??= this.check().finally(() => {
            this.inFlight = null;
        });
        return this.inFlight;
    }

    private async check(): Promise<void> {
        this.clearRetry();
        this.attachFocusRefresh();
        this.lastRefreshMs = Date.now();
        this.checkingStore.set(true);
        await this.loadServerHost();
        await this.restoreServerProvided();
        try {
            const check = await invoke<ApiConfigCheckResponse>('api_get_config');
            const bedrock = check.config.bedrock;
            this.statusStore.set(bedrock.enabled ? 'enabled' : 'disabled');
            const entries = bedrock.servers.map((s) => ({
                // Deterministic id so favorites persist across restarts and
                // config refreshes.
                id: `server:${s.host}:${s.port}`,
                name: s.name,
                host: s.host,
                port: s.port,
                ...(s.protocol_version != null ? { protocolVersion: s.protocol_version } : {}),
                addonMode: s.addon_mode,
                source: 'server' as const,
            }));
            this.answered = true;
            this.serverProvidedStore.set(entries);
            await this.rememberServerProvided(entries);
        } catch (e) {
            // The status becomes unknown, but the operator's list is kept: a check that
            // could not complete is not evidence the server has no servers. This path runs
            // on every focus refresh, so discarding the list made adding a server of your
            // own — which closes a modal, which raises a focus — erase the advertised
            // ones. Only a successful response replaces it.
            logError(`Bedrock capability check failed: ${e}`);
            this.statusStore.set('unknown');
            this.scheduleRetry();
        } finally {
            this.checkingStore.set(false);
        }
    }

    /**
     * Seeds the advertised list from the last good answer for this BVC server.
     *
     * The check is asked again on every window focus, which includes the focus raised by
     * closing the add-server modal. Without this, a manager built from nothing — a restart,
     * or a check that has not answered yet — showed an empty "From your server" until the
     * network came back.
     */
    private async restoreServerProvided(): Promise<void> {
        const host = get(this.serverHostStore);
        // Latched only once a server is known. A first check that ran before the session
        // recorded one has nothing to look the cache up by, and latching there would spend
        // the single attempt on a lookup that could not have hit.
        if (this.restored || !host) {
            return;
        }
        this.restored = true;
        try {
            const store = await AppStore.load();
            const cached = await store.get<CachedServerList>(SERVER_PROVIDED_KEY);
            // Re-read after the await. A check that answered while the store was being
            // read holds the live list, and a cached one must not be written over it.
            if (this.answered || cached?.host !== host) {
                return;
            }
            const entries = BedrockCapabilityManager.usable(cached.entries);
            if (entries.length) {
                this.serverProvidedStore.set(entries);
            }
        } catch (e) {
            logError(`Could not read the cached server list: ${e}`);
        }
    }

    /**
     * Drops anything on disk that is not a server entry.
     *
     * Whatever was cached was written by an older build, and a row missing `source` files
     * itself under "Yours", where the plate offers Edit and Remove that the manager then
     * refuses by id — two buttons that silently do nothing.
     */
    private static usable(entries: unknown): ProxyServerEntry[] {
        if (!Array.isArray(entries)) {
            return [];
        }
        return entries.filter(
            (entry): entry is ProxyServerEntry =>
                typeof entry?.id === 'string' &&
                typeof entry?.host === 'string' &&
                typeof entry?.port === 'number' &&
                entry?.source === 'server',
        );
    }

    private async rememberServerProvided(entries: ProxyServerEntry[]): Promise<void> {
        const host = get(this.serverHostStore);
        if (!host) {
            return;
        }
        try {
            const store = await AppStore.load();
            const cached = await store.get<CachedServerList>(SERVER_PROVIDED_KEY);
            // The advertised list changes approximately never, and this runs on every
            // window focus. `save()` serialises the whole of `store.json`, which §20
            // documents as a round trip Android runs on the UI thread.
            if (cached?.host === host && JSON.stringify(cached.entries) === JSON.stringify(entries)) {
                return;
            }
            await store.set(SERVER_PROVIDED_KEY, { host, entries } satisfies CachedServerList);
            await store.save();
        } catch (e) {
            logError(`Could not cache the server list: ${e}`);
        }
    }

    private async loadServerHost(): Promise<void> {
        try {
            const store = await AppStore.load();
            const url = await store.get<string>('current_server');
            this.serverHostStore.set(url ? url.replace(/^https?:\/\//, '') : '');
        } catch {
            this.serverHostStore.set('');
        }
    }

    // A capability change on the server (operator enables Bedrock, restarts)
    // has no push channel; re-checking when the window regains focus picks it
    // up the next time the user returns to the app. Registered lazily on the
    // first refresh so construction stays side-effect free.
    private attachFocusRefresh(): void {
        if (this.destroyed || this.focusHandler !== null) {
            return;
        }
        this.focusHandler = () => {
            if (Date.now() - this.lastRefreshMs < FOCUS_REFRESH_MIN_INTERVAL_MS) {
                return;
            }
            void this.refresh();
        };
        window.addEventListener('focus', this.focusHandler);
    }

    private scheduleRetry(): void {
        if (this.destroyed || this.retryTimer !== null) {
            return;
        }
        this.retryTimer = setTimeout(() => {
            this.retryTimer = null;
            void this.refresh();
        }, RETRY_INTERVAL_MS);
    }

    private clearRetry(): void {
        if (this.retryTimer !== null) {
            clearTimeout(this.retryTimer);
            this.retryTimer = null;
        }
    }

    destroy(): void {
        this.destroyed = true;
        this.clearRetry();
        if (this.focusHandler !== null) {
            window.removeEventListener('focus', this.focusHandler);
            this.focusHandler = null;
        }
    }
}
