import { writable, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { Store } from '@tauri-apps/plugin-store';
import { error as logError } from '@tauri-apps/plugin-log';
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
    // The transfer relay's port, null unless the server runs one. Clients offer it
    // beside the local addresses.
    private transferPortStore: Writable<number | null>;
    public readonly transferPort: Readable<number | null>;

    private retryTimer: ReturnType<typeof setTimeout> | null = null;
    private focusHandler: (() => void) | null = null;
    private lastRefreshMs = 0;
    private destroyed = false;

    constructor() {
        this.statusStore = writable(null);
        this.status = { subscribe: this.statusStore.subscribe };
        this.serverProvidedStore = writable([]);
        this.serverProvidedServers = { subscribe: this.serverProvidedStore.subscribe };
        this.serverHostStore = writable('');
        this.serverHost = { subscribe: this.serverHostStore.subscribe };
        this.checkingStore = writable(false);
        this.isChecking = { subscribe: this.checkingStore.subscribe };
        this.transferPortStore = writable(null);
        this.transferPort = { subscribe: this.transferPortStore.subscribe };
    }

    async refresh(): Promise<void> {
        this.clearRetry();
        this.attachFocusRefresh();
        this.lastRefreshMs = Date.now();
        this.checkingStore.set(true);
        await this.loadServerHost();
        try {
            const check = await invoke<ApiConfigCheckResponse>('api_get_config');
            const bedrock = check.config.bedrock;
            this.statusStore.set(bedrock.enabled ? 'enabled' : 'disabled');
            this.transferPortStore.set(bedrock.transfer_port ?? null);
            this.serverProvidedStore.set(
                bedrock.servers.map((s) => ({
                    // Deterministic id so favorites persist across restarts and
                    // config refreshes.
                    id: `server:${s.host}:${s.port}`,
                    name: s.name,
                    host: s.host,
                    port: s.port,
                    ...(s.protocol_version != null ? { protocolVersion: s.protocol_version } : {}),
                    addonMode: s.addon_mode,
                    source: 'server' as const,
                })),
            );
        } catch (e) {
            // The status becomes unknown, but the operator's list and transfer port are kept:
            // a check that could not complete is not evidence the server has no servers. This
            // path runs on every focus refresh, so discarding them made adding a server of
            // your own — which closes a modal, which raises a focus — erase the advertised
            // ones. Only a successful response replaces either.
            logError(`Bedrock capability check failed: ${e}`);
            this.statusStore.set('unknown');
            this.scheduleRetry();
        } finally {
            this.checkingStore.set(false);
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
