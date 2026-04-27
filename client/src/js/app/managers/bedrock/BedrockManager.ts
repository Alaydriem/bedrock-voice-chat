import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { Store } from '@tauri-apps/plugin-store';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { openUrl } from '@tauri-apps/plugin-opener';
import { info, error as logError } from '@tauri-apps/plugin-log';
import type { BedrockStatus } from '../../../bindings/BedrockStatus';
import type { RealmEntry } from '../../../bindings/RealmEntry';
import type { NetworkInterface } from './NetworkInterface';
import type { ProxyConfig } from './ProxyConfig';

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

    public readonly canStartProxy: Readable<boolean>;

    private initialized = false;
    private store: Store | null = null;
    private loginFlowUnlisten: (() => void) | null = null;
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

        this.store = await Store.load('store.json', { autoSave: false, defaults: {} });

        const savedFavs = await this.store.get<number[]>('bedrock_realm_favorites');
        if (savedFavs) {
            this.favoritesStore.set(new Set(savedFavs));
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
            this.statusMessageStore.set('Proxy stopped');
        } catch (e) {
            this.statusMessageStore.set(`Error stopping: ${e}`);
        }
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
        try {
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
        if (this.copiedTimeout) {
            clearTimeout(this.copiedTimeout);
        }
    }
}
