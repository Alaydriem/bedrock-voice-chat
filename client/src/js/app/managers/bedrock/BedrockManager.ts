import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { Store } from '@tauri-apps/plugin-store';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { openUrl } from '@tauri-apps/plugin-opener';
import { info, error as logError } from '@tauri-apps/plugin-log';
import type { BedrockStatus } from '../../../bindings/BedrockStatus';
import type { BedrockLogEntry } from '../../../bindings/BedrockLogEntry';
import type { RealmEntry } from '../../../bindings/RealmEntry';
import type { NetworkInterface } from './NetworkInterface';
import type { ProxyConfig } from './ProxyConfig';
import type { ProxyServerEntry } from './ProxyServerEntry';

const MAX_LOG_ENTRIES = 200;

const ALLOWED_LOG_LEVELS = new Set(['INFO', 'WARN', 'ERROR']);

interface RealmsConnectionError {
    kind: 'bds_rejected' | 'nethernet' | 'raknet' | 'auth' | 'generic';
    message: string;
    suggestion: string;
}

const FAILURE_WORDS = /\b(failed|failure|error|timed\s*out|timeout|rejected|refused|unable|cannot|aborted|disconnect(?:ed)?|closed)\b/i;

interface ErrorPattern {
    regex: RegExp;
    kind: RealmsConnectionError['kind'];
    suggestion: string;
    requireFailureWord: boolean;
    minLevel: 'WARN' | 'ERROR';
}

const ERROR_PATTERNS: ErrorPattern[] = [
    {
        regex: /BDS rejected login.*reason code:\s*(\d+)/i,
        kind: 'bds_rejected',
        suggestion: 'The Bedrock server rejected the login. This often happens right after a disconnect — wait 30 seconds and try Connect again. If it persists, click Refresh to renew tokens, or restart BVC.',
        requireFailureWord: false,
        minLevel: 'WARN',
    },
    {
        regex: /\b(nethernet|webrtc|ice\s+gathering)\b/i,
        kind: 'nethernet',
        suggestion: 'NetherNet transport failed. Try Refresh; if it keeps failing, quit and relaunch BVC.',
        requireFailureWord: true,
        minLevel: 'ERROR',
    },
    {
        regex: /\braknet\b/i,
        kind: 'raknet',
        suggestion: 'RakNet handshake failed. Quit and relaunch BVC, then try again — your Realm may also be offline.',
        requireFailureWord: true,
        minLevel: 'ERROR',
    },
    {
        regex: /\b(unauthorized|forbidden|xsts|xbl\s+token|access[_\s]token)\b|\b(401|403)\b/i,
        kind: 'auth',
        suggestion: 'Authentication rejected. Click Refresh to renew tokens; sign out and back in if it keeps failing.',
        requireFailureWord: true,
        minLevel: 'ERROR',
    },
];

const LEVEL_RANK: Record<string, number> = { INFO: 0, WARN: 1, ERROR: 2 };

export class BedrockManager {
    private isEntitledStore: Writable<boolean>;
    private isAuthenticatedStore: Writable<boolean>;
    private isRestoringAuthStore: Writable<boolean>;
    private proxyRunningStore: Writable<boolean>;
    private realmsRunningStore: Writable<boolean>;
    private interfacesStore: Writable<NetworkInterface[]>;
    private proxyConfigStore: Writable<ProxyConfig | null>;
    private statusMessageStore: Writable<string>;
    private showLoginModalStore: Writable<boolean>;
    private selectedInterfaceStore: Writable<string>;
    private isProxyLoadingStore: Writable<boolean>;

    private serverHostStore: Writable<string>;
    private serverPortStore: Writable<number>;
    private listenPortStore: Writable<number>;

    private realmsStore: Writable<RealmEntry[]>;
    private favoritesStore: Writable<Set<number>>;
    private isLoadingRealmsStore: Writable<boolean>;
    private activeRealmIdStore: Writable<number | null>;
    private activeRealmNameStore: Writable<string>;

    private deviceCodeStore: Writable<string>;
    private deviceUrlStore: Writable<string>;
    private loginErrorStore: Writable<string>;
    private codeCopiedStore: Writable<boolean>;

    private realmsLogsStore: Writable<BedrockLogEntry[]>;
    private logsExpandedStore: Writable<boolean>;
    private connectionErrorStore: Writable<RealmsConnectionError | null>;

    private proxyServersStore: Writable<ProxyServerEntry[]>;
    private proxyFavoritesStore: Writable<Set<string>>;
    private activeProxyIdStore: Writable<string | null>;

    public readonly isEntitled: Readable<boolean>;
    public readonly isAuthenticated: Readable<boolean>;
    public readonly isRestoringAuth: Readable<boolean>;
    public readonly proxyRunning: Readable<boolean>;
    public readonly realmsRunning: Readable<boolean>;
    public readonly interfaces: Readable<NetworkInterface[]>;
    public readonly proxyConfig: Readable<ProxyConfig | null>;
    public readonly statusMessage: Readable<string>;
    public readonly showLoginModal: Readable<boolean>;
    public readonly selectedInterface: Readable<string>;
    public readonly isProxyLoading: Readable<boolean>;

    public readonly serverHost: Readable<string>;
    public readonly serverPort: Readable<number>;
    public readonly listenPort: Readable<number>;

    public readonly realms: Readable<RealmEntry[]>;
    public readonly favorites: Readable<Set<number>>;
    public readonly isLoadingRealms: Readable<boolean>;
    public readonly activeRealmId: Readable<number | null>;
    public readonly activeRealmName: Readable<string>;
    public readonly sortedRealms: Readable<RealmEntry[]>;

    public readonly deviceCode: Readable<string>;
    public readonly deviceUrl: Readable<string>;
    public readonly loginError: Readable<string>;
    public readonly codeCopied: Readable<boolean>;

    public readonly realmsLogs: Readable<BedrockLogEntry[]>;
    public readonly logsExpanded: Readable<boolean>;
    public readonly connectionError: Readable<RealmsConnectionError | null>;

    public readonly proxyServers: Readable<ProxyServerEntry[]>;
    public readonly proxyFavorites: Readable<Set<string>>;
    public readonly activeProxyId: Readable<string | null>;
    public readonly sortedProxyServers: Readable<ProxyServerEntry[]>;

    public readonly canStartProxy: Readable<boolean>;

    private initialized = false;
    private store: Store | null = null;
    private loginFlowUnlisten: (() => void) | null = null;
    private logUnlisten: (() => void) | null = null;
    private copiedTimeout: ReturnType<typeof setTimeout> | null = null;

    constructor() {
        this.isEntitledStore = writable(false);
        this.isAuthenticatedStore = writable(false);
        this.isRestoringAuthStore = writable(true);
        this.proxyRunningStore = writable(false);
        this.realmsRunningStore = writable(false);
        this.interfacesStore = writable([]);
        this.proxyConfigStore = writable(null);
        this.statusMessageStore = writable('');
        this.showLoginModalStore = writable(false);
        this.selectedInterfaceStore = writable('');
        this.isProxyLoadingStore = writable(false);

        this.serverHostStore = writable('');
        this.serverPortStore = writable(19132);
        this.listenPortStore = writable(19137);

        this.realmsStore = writable([]);
        this.favoritesStore = writable(new Set());
        this.isLoadingRealmsStore = writable(false);
        this.activeRealmIdStore = writable(null);
        this.activeRealmNameStore = writable('');

        this.deviceCodeStore = writable('');
        this.deviceUrlStore = writable('');
        this.loginErrorStore = writable('');
        this.codeCopiedStore = writable(false);

        this.realmsLogsStore = writable([]);
        this.logsExpandedStore = writable(false);
        this.connectionErrorStore = writable(null);

        this.proxyServersStore = writable([]);
        this.proxyFavoritesStore = writable(new Set());
        this.activeProxyIdStore = writable(null);

        this.isEntitled = { subscribe: this.isEntitledStore.subscribe };
        this.isAuthenticated = { subscribe: this.isAuthenticatedStore.subscribe };
        this.isRestoringAuth = { subscribe: this.isRestoringAuthStore.subscribe };
        this.proxyRunning = { subscribe: this.proxyRunningStore.subscribe };
        this.realmsRunning = { subscribe: this.realmsRunningStore.subscribe };
        this.interfaces = { subscribe: this.interfacesStore.subscribe };
        this.proxyConfig = { subscribe: this.proxyConfigStore.subscribe };
        this.statusMessage = { subscribe: this.statusMessageStore.subscribe };
        this.showLoginModal = { subscribe: this.showLoginModalStore.subscribe };
        this.selectedInterface = { subscribe: this.selectedInterfaceStore.subscribe };
        this.isProxyLoading = { subscribe: this.isProxyLoadingStore.subscribe };

        this.serverHost = { subscribe: this.serverHostStore.subscribe };
        this.serverPort = { subscribe: this.serverPortStore.subscribe };
        this.listenPort = { subscribe: this.listenPortStore.subscribe };

        this.realms = { subscribe: this.realmsStore.subscribe };
        this.favorites = { subscribe: this.favoritesStore.subscribe };
        this.isLoadingRealms = { subscribe: this.isLoadingRealmsStore.subscribe };
        this.activeRealmId = { subscribe: this.activeRealmIdStore.subscribe };
        this.activeRealmName = { subscribe: this.activeRealmNameStore.subscribe };

        this.deviceCode = { subscribe: this.deviceCodeStore.subscribe };
        this.deviceUrl = { subscribe: this.deviceUrlStore.subscribe };
        this.loginError = { subscribe: this.loginErrorStore.subscribe };
        this.codeCopied = { subscribe: this.codeCopiedStore.subscribe };

        this.realmsLogs = { subscribe: this.realmsLogsStore.subscribe };
        this.logsExpanded = { subscribe: this.logsExpandedStore.subscribe };
        this.connectionError = { subscribe: this.connectionErrorStore.subscribe };

        this.proxyServers = { subscribe: this.proxyServersStore.subscribe };
        this.proxyFavorites = { subscribe: this.proxyFavoritesStore.subscribe };
        this.activeProxyId = { subscribe: this.activeProxyIdStore.subscribe };

        this.sortedProxyServers = derived(
            [this.proxyServersStore, this.proxyFavoritesStore],
            ([$servers, $favorites]) =>
                [...$servers].sort((a, b) => {
                    const aFav = $favorites.has(a.id) ? 0 : 1;
                    const bFav = $favorites.has(b.id) ? 0 : 1;
                    return aFav - bFav || a.name.localeCompare(b.name);
                })
        );

        this.sortedRealms = derived(
            [this.realmsStore, this.favoritesStore],
            ([$realms, $favorites]) =>
                [...$realms].sort((a, b) => {
                    const aFav = $favorites.has(a.id) ? 0 : 1;
                    const bFav = $favorites.has(b.id) ? 0 : 1;
                    return aFav - bFav || a.name.localeCompare(b.name);
                })
        );

        this.canStartProxy = derived(
            [this.isAuthenticatedStore, this.proxyRunningStore, this.realmsRunningStore, this.serverHostStore],
            ([$auth, $proxy, $realms, $host]) =>
                $auth && !$proxy && !$realms && $host.length > 0
        );
    }

    async initialize(): Promise<void> {
        if (this.initialized) {
            return;
        }
        this.initialized = true;

        await this.subscribeToLogs();

        this.store = await Store.load('store.json', { autoSave: false, defaults: {} });

        const savedFavs = await this.store.get<number[]>('bedrock_realm_favorites');
        if (savedFavs) {
            this.favoritesStore.set(new Set(savedFavs));
        }

        const savedProxies = await this.store.get<ProxyServerEntry[]>('bedrock_proxy_servers');
        if (savedProxies) {
            this.proxyServersStore.set(savedProxies);
        }

        const savedProxyFavs = await this.store.get<string[]>('bedrock_proxy_favorites');
        if (savedProxyFavs) {
            this.proxyFavoritesStore.set(new Set(savedProxyFavs));
        }

        try {
            const entitled = await invoke<boolean>('bedrock_check_entitlement');
            this.isEntitledStore.set(entitled);
        } catch (e) {
            logError(`Entitlement check failed: ${e}`);
        }

        try {
            const restored = await invoke<boolean>('bedrock_restore_auth');
            if (restored) {
                this.isAuthenticatedStore.set(true);
            }
        } catch (e) {
            logError(`Auth restore failed: ${e}`);
        }

        try {
            const status = await invoke<BedrockStatus>('bedrock_get_status');
            this.proxyRunningStore.set(status.proxy_running);
            this.realmsRunningStore.set(status.realms_running);
            this.isAuthenticatedStore.set(status.xbox_authenticated);

            if (status.proxy_target_host) {
                this.serverHostStore.set(status.proxy_target_host);
            }
            if (status.proxy_target_port) {
                this.serverPortStore.set(status.proxy_target_port);
            }
            if (status.proxy_listen_port) {
                this.listenPortStore.set(status.proxy_listen_port);
            }

            if (status.proxy_running && status.proxy_target_host && status.proxy_target_port) {
                const match = get(this.proxyServersStore).find(
                    (s) => s.host === status.proxy_target_host && s.port === status.proxy_target_port
                );
                if (match) {
                    this.activeProxyIdStore.set(match.id);
                }
            }

            if (status.active_realm_id) {
                this.activeRealmIdStore.set(status.active_realm_id);
            }
            if (status.active_realm_name) {
                this.activeRealmNameStore.set(status.active_realm_name);
            }
        } catch (e) {
            logError(`Status check failed: ${e}`);
        }

        this.isRestoringAuthStore.set(false);

        if (get(this.isAuthenticatedStore)) {
            await this.loadInterfaces();
            await this.loadRealms();
        }
    }

    setServerHost(value: string): void {
        this.serverHostStore.set(value);
    }

    setServerPort(value: number): void {
        this.serverPortStore.set(value);
    }

    setListenPort(value: number): void {
        this.listenPortStore.set(value);
    }

    setSelectedInterface(value: string): void {
        this.selectedInterfaceStore.set(value);
    }

    async loadInterfaces(): Promise<void> {
        try {
            const ifaces = await invoke<NetworkInterface[]>('bedrock_list_interfaces');
            this.interfacesStore.set(ifaces);
            if (ifaces.length > 0 && !get(this.selectedInterfaceStore)) {
                this.selectedInterfaceStore.set(ifaces[0].ip);
            }
        } catch (e) {
            logError(`Failed to load interfaces: ${e}`);
        }
    }

    async loadRealms(): Promise<void> {
        this.isLoadingRealmsStore.set(true);
        try {
            const realms = await invoke<RealmEntry[]>('bedrock_list_realms');
            this.realmsStore.set(realms);
        } catch (e) {
            logError(`Failed to load realms: ${e}`);
        }
        this.isLoadingRealmsStore.set(false);
    }

    async refreshRealms(): Promise<void> {
        this.isLoadingRealmsStore.set(true);
        this.connectionErrorStore.set(null);
        try {
            await invoke('bedrock_force_refresh');
            info('Bedrock token refreshed');
        } catch (e) {
            logError(`Token refresh failed: ${e}`);
        }
        try {
            const realms = await invoke<RealmEntry[]>('bedrock_list_realms');
            this.realmsStore.set(realms);
        } catch (e) {
            logError(`Failed to load realms: ${e}`);
        }
        this.isLoadingRealmsStore.set(false);
    }

    toggleLogs(): void {
        this.logsExpandedStore.update((v) => !v);
    }

    clearLogs(): void {
        this.realmsLogsStore.set([]);
        this.connectionErrorStore.set(null);
    }

    dismissConnectionError(): void {
        this.connectionErrorStore.set(null);
    }

    async openLoginModal(): Promise<void> {
        this.deviceCodeStore.set('');
        this.deviceUrlStore.set('');
        this.loginErrorStore.set('');
        this.showLoginModalStore.set(true);

        const appWebview = getCurrentWebviewWindow();
        this.loginFlowUnlisten = await appWebview.listen(
            'bedrock-device-code',
            (event: { payload?: { code?: string; url?: string } }) => {
                const payload = event.payload;
                if (payload?.code) {
                    this.deviceCodeStore.set(payload.code);
                }
                if (payload?.url) {
                    this.deviceUrlStore.set(payload.url);
                }
                info(`Device code received: ${payload?.code}`);
            }
        );

        try {
            await invoke('bedrock_xbox_login');
            info('Xbox login succeeded');
            this.isAuthenticatedStore.set(true);
            this.statusMessageStore.set('Signed in to Xbox Live');
            this.showLoginModalStore.set(false);
            this.cleanupLoginListener();
            await this.loadInterfaces();
            await this.loadRealms();
        } catch (e) {
            const msg = String(e);
            if (msg === 'Login cancelled') {
                this.showLoginModalStore.set(false);
            } else {
                this.loginErrorStore.set(msg);
                logError(`Xbox login failed: ${msg}`);
            }
            this.cleanupLoginListener();
        }
    }

    async closeLoginModal(): Promise<void> {
        try {
            await invoke('bedrock_cancel_xbox_login');
        } catch (e) {
            logError(`Cancel failed: ${e}`);
        }
        this.cleanupLoginListener();
        this.showLoginModalStore.set(false);
    }

    async signOut(): Promise<void> {
        try {
            await invoke('bedrock_xbox_logout');
            this.isAuthenticatedStore.set(false);
            this.realmsStore.set([]);
            this.statusMessageStore.set('');
        } catch (e) {
            this.statusMessageStore.set(`Error: ${e}`);
        }
    }

    async startProxy(): Promise<void> {
        this.isProxyLoadingStore.set(true);
        try {
            try {
                await invoke('bedrock_force_refresh');
            } catch (e) {
                logError(`Token refresh before proxy start failed: ${e}`);
            }
            const targetHost = get(this.serverHostStore);
            const targetPort = get(this.serverPortStore);
            await invoke('bedrock_start_proxy', {
                targetHost,
                targetPort,
                listenPort: get(this.listenPortStore),
                networkInterface: get(this.selectedInterfaceStore),
            });
            this.proxyRunningStore.set(true);
            this.statusMessageStore.set(`Proxy running → ${targetHost}:${targetPort}`);
            info(`Bedrock proxy started: ${targetHost}:${targetPort}`);
        } catch (e) {
            this.statusMessageStore.set(`Error: ${e}`);
            logError(`Proxy start failed: ${e}`);
        }
        this.isProxyLoadingStore.set(false);
    }

    async stopProxy(): Promise<void> {
        try {
            await invoke('bedrock_stop_proxy');
            this.proxyRunningStore.set(false);
            this.activeProxyIdStore.set(null);
            this.statusMessageStore.set('Proxy stopped');
        } catch (e) {
            this.statusMessageStore.set(`Error stopping: ${e}`);
        }
    }

    async addProxyServer(name: string, host: string, port: number): Promise<ProxyServerEntry> {
        const entry: ProxyServerEntry = {
            id: crypto.randomUUID(),
            name: name.trim(),
            host: host.trim(),
            port,
        };
        this.proxyServersStore.update((current) => [...current, entry]);
        await this.persistProxyServers();
        return entry;
    }

    async updateProxyServer(id: string, patch: Partial<Omit<ProxyServerEntry, 'id'>>): Promise<void> {
        this.proxyServersStore.update((current) =>
            current.map((s) =>
                s.id === id
                    ? {
                          ...s,
                          ...(patch.name !== undefined ? { name: patch.name.trim() } : {}),
                          ...(patch.host !== undefined ? { host: patch.host.trim() } : {}),
                          ...(patch.port !== undefined ? { port: patch.port } : {}),
                      }
                    : s
            )
        );
        await this.persistProxyServers();
    }

    async deleteProxyServer(id: string): Promise<void> {
        this.proxyServersStore.update((current) => current.filter((s) => s.id !== id));
        this.proxyFavoritesStore.update((current) => {
            const next = new Set(current);
            next.delete(id);
            return next;
        });
        if (get(this.activeProxyIdStore) === id) {
            this.activeProxyIdStore.set(null);
        }
        await this.persistProxyServers();
        await this.persistProxyFavorites();
    }

    async toggleProxyFavorite(id: string): Promise<void> {
        this.proxyFavoritesStore.update((current) => {
            const next = new Set(current);
            if (next.has(id)) {
                next.delete(id);
            } else {
                next.add(id);
            }
            return next;
        });
        await this.persistProxyFavorites();
    }

    async connectToProxyServer(entry: ProxyServerEntry): Promise<void> {
        this.serverHostStore.set(entry.host);
        this.serverPortStore.set(entry.port);
        this.activeProxyIdStore.set(entry.id);
        await this.startProxy();
        if (!get(this.proxyRunningStore)) {
            this.activeProxyIdStore.set(null);
        }
    }

    private async persistProxyServers(): Promise<void> {
        if (!this.store) {
            return;
        }
        await this.store.set('bedrock_proxy_servers', get(this.proxyServersStore));
        await this.store.save();
    }

    private async persistProxyFavorites(): Promise<void> {
        if (!this.store) {
            return;
        }
        await this.store.set('bedrock_proxy_favorites', [...get(this.proxyFavoritesStore)]);
        await this.store.save();
    }

    async toggleFavorite(realmId: number): Promise<void> {
        this.favoritesStore.update((current) => {
            const next = new Set(current);
            if (next.has(realmId)) {
                next.delete(realmId);
            } else {
                next.add(realmId);
            }
            return next;
        });

        if (this.store) {
            await this.store.set('bedrock_realm_favorites', [...get(this.favoritesStore)]);
            await this.store.save();
        }
    }

    async connectToRealm(realm: RealmEntry): Promise<void> {
        this.statusMessageStore.set('');
        this.realmsLogsStore.set([]);
        this.connectionErrorStore.set(null);
        try {
            try {
                await invoke('bedrock_force_refresh');
            } catch (e) {
                logError(`Token refresh before realm connect failed: ${e}`);
            }
            await invoke('bedrock_start_realms', {
                realmId: realm.id,
                realmName: realm.name,
                networkInterface: get(this.selectedInterfaceStore),
            });
            this.realmsRunningStore.set(true);
            this.activeRealmIdStore.set(realm.id);
            this.activeRealmNameStore.set(realm.name);
            this.statusMessageStore.set(`Connected to ${realm.name}`);
            info(`Bedrock realms started: ${realm.name} (${realm.id})`);
        } catch (e) {
            this.statusMessageStore.set(`Error: ${e}`);
            logError(`Realms start failed: ${e}`);
            this.detectError(String(e));
        }
    }

    async stopRealms(): Promise<void> {
        try {
            await invoke('bedrock_stop_realms');
            this.realmsRunningStore.set(false);
            this.activeRealmIdStore.set(null);
            this.activeRealmNameStore.set('');
            this.statusMessageStore.set('Disconnected');
        } catch (e) {
            this.statusMessageStore.set(`Error stopping: ${e}`);
        }
    }

    private async subscribeToLogs(): Promise<void> {
        if (this.logUnlisten) {
            return;
        }
        const appWebview = getCurrentWebviewWindow();
        this.logUnlisten = await appWebview.listen<BedrockLogEntry>(
            'bedrock-log',
            (event) => {
                const entry = event.payload;
                if (!ALLOWED_LOG_LEVELS.has(entry.level)) {
                    return;
                }
                this.realmsLogsStore.update((current) => {
                    const next = current.length >= MAX_LOG_ENTRIES
                        ? [...current.slice(current.length - MAX_LOG_ENTRIES + 1), entry]
                        : [...current, entry];
                    return next;
                });
                if (entry.level === 'WARN' || entry.level === 'ERROR') {
                    this.detectError(entry.message, entry.level);
                }
            }
        );
    }

    private detectError(message: string, level: string = 'ERROR'): void {
        if (get(this.connectionErrorStore)) {
            return;
        }
        const entryRank = LEVEL_RANK[level] ?? LEVEL_RANK.ERROR;
        for (const pattern of ERROR_PATTERNS) {
            if (entryRank < (LEVEL_RANK[pattern.minLevel] ?? LEVEL_RANK.ERROR)) {
                continue;
            }
            if (pattern.requireFailureWord && !FAILURE_WORDS.test(message)) {
                continue;
            }
            if (pattern.regex.test(message)) {
                this.connectionErrorStore.set({
                    kind: pattern.kind,
                    message,
                    suggestion: pattern.suggestion,
                });
                return;
            }
        }
    }

    async copyDeviceCode(): Promise<void> {
        try {
            await navigator.clipboard.writeText(get(this.deviceCodeStore));
            this.codeCopiedStore.set(true);
            if (this.copiedTimeout) {
                clearTimeout(this.copiedTimeout);
            }
            this.copiedTimeout = setTimeout(() => {
                this.codeCopiedStore.set(false);
                this.copiedTimeout = null;
            }, 2000);
        } catch (e) {
            logError(`Clipboard write failed: ${e}`);
        }
    }

    async openLoginUrl(): Promise<void> {
        try {
            await openUrl(get(this.deviceUrlStore));
        } catch (e) {
            logError(`Failed to open URL: ${e}`);
        }
    }

    private cleanupLoginListener(): void {
        if (this.loginFlowUnlisten) {
            this.loginFlowUnlisten();
            this.loginFlowUnlisten = null;
        }
    }

    destroy(): void {
        this.cleanupLoginListener();
        if (this.logUnlisten) {
            this.logUnlisten();
            this.logUnlisten = null;
        }
        if (this.copiedTimeout) {
            clearTimeout(this.copiedTimeout);
        }
    }
}
