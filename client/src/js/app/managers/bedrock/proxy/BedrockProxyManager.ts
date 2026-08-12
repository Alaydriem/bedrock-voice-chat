import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { Store } from '@tauri-apps/plugin-store';
import { info, error as logError } from '@tauri-apps/plugin-log';
import type { NetworkInterface } from '../../../../bindings/NetworkInterface';
import type { AddonTransport } from '../../../../bindings/AddonTransport';
import type { ProtocolVersionOption } from '../../../../bindings/ProtocolVersionOption';
import type { ProxyServerEntry } from '../ProxyServerEntry';
import type { BedrockProxyManagerCallbacks } from './BedrockProxyManagerCallbacks';
import type { ProxyStatusSnapshot } from './ProxyStatusSnapshot';

export class BedrockProxyManager {
    private proxyRunningStore: Writable<boolean>;
    private interfacesStore: Writable<NetworkInterface[]>;
    private selectedInterfaceStore: Writable<string>;
    private isProxyLoadingStore: Writable<boolean>;
    private serverHostStore: Writable<string>;
    private serverPortStore: Writable<number>;
    private listenPortStore: Writable<number>;
    private proxyServersStore: Writable<ProxyServerEntry[]>;
    // Entries advertised by the BVC server. Kept separate from the user's
    // persisted list: merged for display only, so switching BVC servers or
    // changing server config never leaves stale entries in the Store.
    private serverProvidedStore: Writable<ProxyServerEntry[]>;
    private proxyFavoritesStore: Writable<Set<string>>;
    private activeProxyIdStore: Writable<string | null>;

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

    private store: Store | null = null;
    private callbacks: BedrockProxyManagerCallbacks;

    constructor(callbacks: BedrockProxyManagerCallbacks) {
        this.callbacks = callbacks;

        this.proxyRunningStore = writable(false);
        this.interfacesStore = writable([]);
        this.selectedInterfaceStore = writable('');
        this.isProxyLoadingStore = writable(false);
        this.serverHostStore = writable('');
        this.serverPortStore = writable(19132);
        this.listenPortStore = writable(19137);
        this.proxyServersStore = writable([]);
        this.serverProvidedStore = writable([]);
        this.proxyFavoritesStore = writable(new Set());
        this.activeProxyIdStore = writable(null);

        this.proxyRunning = { subscribe: this.proxyRunningStore.subscribe };
        this.interfaces = { subscribe: this.interfacesStore.subscribe };
        this.selectedInterface = { subscribe: this.selectedInterfaceStore.subscribe };
        this.isProxyLoading = { subscribe: this.isProxyLoadingStore.subscribe };
        this.serverHost = { subscribe: this.serverHostStore.subscribe };
        this.serverPort = { subscribe: this.serverPortStore.subscribe };
        this.listenPort = { subscribe: this.listenPortStore.subscribe };
        this.proxyServers = { subscribe: this.proxyServersStore.subscribe };
        this.proxyFavorites = { subscribe: this.proxyFavoritesStore.subscribe };
        this.activeProxyId = { subscribe: this.activeProxyIdStore.subscribe };

        this.sortedProxyServers = derived(
            [this.serverProvidedStore, this.proxyServersStore, this.proxyFavoritesStore],
            ([$provided, $user, $favorites]) => {
                // A user entry with the same host:port wins the dedupe — it may
                // carry a custom name or protocol override.
                const userKeys = new Set($user.map((s) => `${s.host}:${s.port}`));
                const merged = [
                    ...$provided.filter((s) => !userKeys.has(`${s.host}:${s.port}`)),
                    ...$user,
                ];
                return merged.sort((a, b) => {
                    const aFav = $favorites.has(a.id) ? 0 : 1;
                    const bFav = $favorites.has(b.id) ? 0 : 1;
                    return aFav - bFav || a.name.localeCompare(b.name);
                });
            },
        );
    }

    setServerProvidedServers(entries: ProxyServerEntry[]): void {
        this.serverProvidedStore.set(entries);
    }

    getSelectedInterface(): string {
        return get(this.selectedInterfaceStore);
    }

    getServerHost(): string {
        return get(this.serverHostStore);
    }

    isRunning(): boolean {
        return get(this.proxyRunningStore);
    }

    async initialize(store: Store): Promise<void> {
        this.store = store;
        const savedProxies = await store.get<ProxyServerEntry[]>('bedrock_proxy_servers');
        if (savedProxies) {
            this.proxyServersStore.set(savedProxies);
        }
        const savedProxyFavs = await store.get<string[]>('bedrock_proxy_favorites');
        if (savedProxyFavs) {
            this.proxyFavoritesStore.set(new Set(savedProxyFavs));
        }
    }

    applyStatus(snapshot: ProxyStatusSnapshot): void {
        this.proxyRunningStore.set(snapshot.running);
        if (snapshot.host) {
            this.serverHostStore.set(snapshot.host);
        }
        if (snapshot.port) {
            this.serverPortStore.set(snapshot.port);
        }
        if (snapshot.listenPort) {
            this.listenPortStore.set(snapshot.listenPort);
        }
        if (snapshot.running && snapshot.host && snapshot.port) {
            const match = [...get(this.serverProvidedStore), ...get(this.proxyServersStore)].find(
                (s) => s.host === snapshot.host && s.port === snapshot.port,
            );
            if (match) {
                this.activeProxyIdStore.set(match.id);
            }
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
                const defaultIface = ifaces.find((i) => i.is_ipv4) ?? ifaces[0];
                this.selectedInterfaceStore.set(defaultIface.ip);
            }
        } catch (e) {
            logError(`Failed to load interfaces: ${e}`);
        }
    }

    async listProtocolVersions(): Promise<ProtocolVersionOption[]> {
        try {
            return await invoke<ProtocolVersionOption[]>('bedrock_list_protocol_versions');
        } catch (e) {
            logError(`Failed to load protocol versions: ${e}`);
            return [];
        }
    }

    async startProxy(
        advertisedProtocol?: number | null,
        addonTransport?: AddonTransport | null,
    ): Promise<void> {
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
                advertisedProtocol: advertisedProtocol ?? null,
                addonTransport: addonTransport ?? null,
            });
            this.proxyRunningStore.set(true);
            info(`Bedrock proxy started: ${targetHost}:${targetPort}`);
        } catch (e) {
            this.callbacks.setStatus(`Error: ${e}`);
            logError(`Proxy start failed: ${e}`);
        }
        this.isProxyLoadingStore.set(false);
    }

    async stopProxy(): Promise<void> {
        try {
            await invoke('bedrock_stop_proxy');
            this.proxyRunningStore.set(false);
            this.activeProxyIdStore.set(null);
            this.callbacks.setStatus('Proxy stopped');
        } catch (e) {
            this.callbacks.setStatus(`Error stopping: ${e}`);
        }
    }

    async addProxyServer(
        name: string,
        host: string,
        port: number,
        protocolVersion?: number,
    ): Promise<ProxyServerEntry> {
        const entry: ProxyServerEntry = {
            id: crypto.randomUUID(),
            name: name.trim(),
            host: host.trim(),
            port,
            ...(protocolVersion !== undefined ? { protocolVersion } : {}),
        };
        this.proxyServersStore.update((current) => [...current, entry]);
        await this.persistProxyServers();
        return entry;
    }

    async updateProxyServer(id: string, patch: Partial<Omit<ProxyServerEntry, 'id'>>): Promise<void> {
        if (get(this.serverProvidedStore).some((s) => s.id === id)) {
            return;
        }
        this.proxyServersStore.update((current) =>
            current.map((s) =>
                s.id === id
                    ? {
                          ...s,
                          ...(patch.name !== undefined ? { name: patch.name.trim() } : {}),
                          ...(patch.host !== undefined ? { host: patch.host.trim() } : {}),
                          ...(patch.port !== undefined ? { port: patch.port } : {}),
                          ...('protocolVersion' in patch
                              ? { protocolVersion: patch.protocolVersion }
                              : {}),
                      }
                    : s,
            ),
        );
        await this.persistProxyServers();
    }

    async deleteProxyServer(id: string): Promise<void> {
        if (get(this.serverProvidedStore).some((s) => s.id === id)) {
            return;
        }
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
        await this.startProxy(entry.protocolVersion ?? null, entry.addonTransport ?? null);
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
}
