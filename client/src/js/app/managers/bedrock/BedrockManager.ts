import { writable, derived, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { Store } from '@tauri-apps/plugin-store';
import { error as logError } from '@tauri-apps/plugin-log';
import type { BedrockStatus } from '../../../bindings/BedrockStatus';
import type { BedrockLogEntry } from '../../../bindings/BedrockLogEntry';
import type { BedrockConnectionInfo } from '../../../bindings/BedrockConnectionInfo';
import type { RealmEntry } from '../../../bindings/RealmEntry';
import type { NetworkInterface } from '../../../bindings/NetworkInterface';
import type { ProxyServerEntry } from './ProxyServerEntry';
import { BedrockAuthManager } from './auth/BedrockAuthManager';
import { BedrockProxyManager } from './proxy/BedrockProxyManager';
import { BedrockRealmsManager } from './realms/BedrockRealmsManager';
import { BedrockLogsManager } from './logs/BedrockLogsManager';
import { BedrockConnectionManager } from './connection/BedrockConnectionManager';
import type { RealmsConnectionError } from './connection/RealmsConnectionError';
import type { RealmsConnectionErrorKind } from './connection/RealmsConnectionErrorKind';

export type { RealmsConnectionError, RealmsConnectionErrorKind };

export class BedrockManager {
    private readonly authManager: BedrockAuthManager;
    private readonly proxyManager: BedrockProxyManager;
    private readonly realmsManager: BedrockRealmsManager;
    private readonly logsManager: BedrockLogsManager;
    private readonly connectionManager: BedrockConnectionManager;

    private statusMessageStore: Writable<string>;
    public readonly statusMessage: Readable<string>;

    public readonly isEntitled: Readable<boolean>;
    public readonly isAuthenticated: Readable<boolean>;
    public readonly isRestoringAuth: Readable<boolean>;
    public readonly showLoginModal: Readable<boolean>;
    public readonly deviceCode: Readable<string>;
    public readonly deviceUrl: Readable<string>;
    public readonly loginError: Readable<string>;
    public readonly codeCopied: Readable<boolean>;

    public readonly proxyRunning: Readable<boolean>;
    public readonly interfaces: Readable<NetworkInterface[]>;
    public readonly selectedInterface: Readable<string>;
    public readonly isProxyLoading: Readable<boolean>;
    public readonly serverHost: Readable<string>;
    public readonly serverPort: Readable<number>;
    public readonly listenPort: Readable<number>;
    public readonly proxyServers: Readable<ProxyServerEntry[]>;
    public readonly proxyFavorites: Readable<Set<string>>;
    public readonly activeProxyId: Readable<string | null>;
    public readonly sortedProxyServers: Readable<ProxyServerEntry[]>;

    public readonly realmsRunning: Readable<boolean>;
    public readonly realms: Readable<RealmEntry[]>;
    public readonly favorites: Readable<Set<string>>;
    public readonly isLoadingRealms: Readable<boolean>;
    public readonly activeRealmId: Readable<bigint | null>;
    public readonly activeRealmName: Readable<string>;
    public readonly sortedRealms: Readable<RealmEntry[]>;

    public readonly realmsLogs: Readable<BedrockLogEntry[]>;
    public readonly logsExpanded: Readable<boolean>;

    public readonly connectionError: Readable<RealmsConnectionError | null>;
    public readonly connectionInfo: Readable<BedrockConnectionInfo | null>;

    public readonly canStartProxy: Readable<boolean>;

    private initialized = false;
    private store: Store | null = null;

    constructor() {
        this.statusMessageStore = writable('');
        this.statusMessage = { subscribe: this.statusMessageStore.subscribe };

        const setStatus = (msg: string) => this.statusMessageStore.set(msg);

        this.logsManager = new BedrockLogsManager();
        this.connectionManager = new BedrockConnectionManager(() => this.realmsManager);
        this.proxyManager = new BedrockProxyManager({ setStatus });
        this.realmsManager = new BedrockRealmsManager(
            () => this.proxyManager.getSelectedInterface(),
            {
                setStatus,
                reportError: (raw) => this.connectionManager.setConnectErrorFromInvoke(raw),
                clearLogs: () => this.logsManager.clearLogs(),
                clearConnectionError: () => this.connectionManager.clearError(),
            },
        );

        this.authManager = new BedrockAuthManager({
            setStatus,
            onLoginSuccess: async () => {
                await this.proxyManager.loadInterfaces();
                await this.realmsManager.loadRealms();
            },
        });

        this.isEntitled = this.authManager.isEntitled;
        this.isAuthenticated = this.authManager.isAuthenticated;
        this.isRestoringAuth = this.authManager.isRestoringAuth;
        this.showLoginModal = this.authManager.showLoginModal;
        this.deviceCode = this.authManager.deviceCode;
        this.deviceUrl = this.authManager.deviceUrl;
        this.loginError = this.authManager.loginError;
        this.codeCopied = this.authManager.codeCopied;

        this.proxyRunning = this.proxyManager.proxyRunning;
        this.interfaces = this.proxyManager.interfaces;
        this.selectedInterface = this.proxyManager.selectedInterface;
        this.isProxyLoading = this.proxyManager.isProxyLoading;
        this.serverHost = this.proxyManager.serverHost;
        this.serverPort = this.proxyManager.serverPort;
        this.listenPort = this.proxyManager.listenPort;
        this.proxyServers = this.proxyManager.proxyServers;
        this.proxyFavorites = this.proxyManager.proxyFavorites;
        this.activeProxyId = this.proxyManager.activeProxyId;
        this.sortedProxyServers = this.proxyManager.sortedProxyServers;

        this.realmsRunning = this.realmsManager.realmsRunning;
        this.realms = this.realmsManager.realms;
        this.favorites = this.realmsManager.favorites;
        this.isLoadingRealms = this.realmsManager.isLoadingRealms;
        this.activeRealmId = this.realmsManager.activeRealmId;
        this.activeRealmName = this.realmsManager.activeRealmName;
        this.sortedRealms = this.realmsManager.sortedRealms;

        this.realmsLogs = this.logsManager.realmsLogs;
        this.logsExpanded = this.logsManager.logsExpanded;

        this.connectionError = this.connectionManager.connectionError;
        this.connectionInfo = this.connectionManager.connectionInfo;

        this.canStartProxy = derived(
            [this.authManager.isAuthenticated, this.proxyManager.proxyRunning, this.realmsManager.realmsRunning, this.proxyManager.serverHost],
            ([$auth, $proxy, $realms, $host]) =>
                $auth && !$proxy && !$realms && $host.length > 0,
        );
    }

    async initialize(): Promise<void> {
        if (this.initialized) {
            return;
        }
        this.initialized = true;

        await this.logsManager.initialize();
        await this.connectionManager.initialize();

        this.store = await Store.load('store.json', { autoSave: false, defaults: {} });
        await this.realmsManager.initialize(this.store);
        await this.proxyManager.initialize(this.store);

        await this.authManager.checkEntitlement();
        await this.authManager.restoreAuth();

        try {
            const status = await invoke<BedrockStatus>('bedrock_get_status');
            this.proxyManager.applyStatus({
                host: status.proxy_target_host ?? null,
                port: status.proxy_target_port ?? null,
                listenPort: status.proxy_listen_port ?? null,
                running: status.proxy_running,
            });
            this.realmsManager.applyStatus(
                status.active_realm_id ?? null,
                status.active_realm_name ?? null,
                status.realms_running,
            );
            this.authManager.setAuthenticated(status.xbox_authenticated);
        } catch (e) {
            logError(`Status check failed: ${e}`);
        }

        this.authManager.finishRestoring();

        if (this.authManager.isAuthenticatedNow()) {
            await this.proxyManager.loadInterfaces();
            await this.realmsManager.loadRealms();
        }
    }

    setServerHost(value: string): void {
        this.proxyManager.setServerHost(value);
    }

    setServerPort(value: number): void {
        this.proxyManager.setServerPort(value);
    }

    setListenPort(value: number): void {
        this.proxyManager.setListenPort(value);
    }

    setSelectedInterface(value: string): void {
        this.proxyManager.setSelectedInterface(value);
    }

    async loadInterfaces(): Promise<void> {
        return this.proxyManager.loadInterfaces();
    }

    async loadRealms(): Promise<void> {
        return this.realmsManager.loadRealms();
    }

    async refreshRealms(): Promise<void> {
        return this.realmsManager.refreshRealms();
    }

    toggleLogs(): void {
        this.logsManager.toggleLogs();
    }

    clearLogs(): void {
        this.logsManager.clearLogs();
        this.connectionManager.clearError();
    }

    dismissConnectionError(): void {
        this.connectionManager.dismissConnectionError();
    }

    dismissConnectionInfo(): void {
        this.connectionManager.dismissConnectionInfo();
    }

    async openLoginModal(): Promise<void> {
        return this.authManager.openLoginModal();
    }

    async closeLoginModal(): Promise<void> {
        return this.authManager.closeLoginModal();
    }

    async signOut(): Promise<void> {
        if (this.realmsManager.isRunning()) {
            await this.realmsManager.stopRealms();
        }
        if (this.proxyManager.isRunning()) {
            await this.proxyManager.stopProxy();
        }
        await this.authManager.signOut();
        this.realmsManager.reset();
    }

    async startProxy(): Promise<void> {
        return this.proxyManager.startProxy();
    }

    async stopProxy(): Promise<void> {
        return this.proxyManager.stopProxy();
    }

    async addProxyServer(name: string, host: string, port: number): Promise<ProxyServerEntry> {
        return this.proxyManager.addProxyServer(name, host, port);
    }

    async updateProxyServer(id: string, patch: Partial<Omit<ProxyServerEntry, 'id'>>): Promise<void> {
        return this.proxyManager.updateProxyServer(id, patch);
    }

    async deleteProxyServer(id: string): Promise<void> {
        return this.proxyManager.deleteProxyServer(id);
    }

    async toggleProxyFavorite(id: string): Promise<void> {
        return this.proxyManager.toggleProxyFavorite(id);
    }

    async connectToProxyServer(entry: ProxyServerEntry): Promise<void> {
        return this.proxyManager.connectToProxyServer(entry);
    }

    async toggleFavorite(realmId: bigint): Promise<void> {
        return this.realmsManager.toggleFavorite(realmId);
    }

    async connectToRealm(realm: RealmEntry): Promise<void> {
        return this.realmsManager.connectToRealm(realm);
    }

    async stopRealms(): Promise<void> {
        return this.realmsManager.stopRealms();
    }

    async copyDeviceCode(): Promise<void> {
        return this.authManager.copyDeviceCode();
    }

    async openLoginUrl(): Promise<void> {
        return this.authManager.openLoginUrl();
    }

    destroy(): void {
        this.authManager.destroy();
        this.logsManager.destroy();
        this.connectionManager.destroy();
    }
}
