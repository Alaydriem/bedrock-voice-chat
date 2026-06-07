import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { Store } from '@tauri-apps/plugin-store';
import { info, error as logError } from '@tauri-apps/plugin-log';
import type { RealmEntry } from '../../../../bindings/RealmEntry';
import type { RealmsLifecycle } from './RealmsLifecycle';
import type { BedrockRealmsManagerCallbacks } from './BedrockRealmsManagerCallbacks';

export class BedrockRealmsManager implements RealmsLifecycle {
    private realmsRunningStore: Writable<boolean>;
    private realmsStore: Writable<RealmEntry[]>;
    private favoritesStore: Writable<Set<string>>;
    private isLoadingRealmsStore: Writable<boolean>;
    private activeRealmIdStore: Writable<bigint | null>;
    private activeRealmNameStore: Writable<string>;

    public readonly realmsRunning: Readable<boolean>;
    public readonly realms: Readable<RealmEntry[]>;
    public readonly favorites: Readable<Set<string>>;
    public readonly isLoadingRealms: Readable<boolean>;
    public readonly activeRealmId: Readable<bigint | null>;
    public readonly activeRealmName: Readable<string>;
    public readonly sortedRealms: Readable<RealmEntry[]>;

    private store: Store | null = null;
    private selectedInterface: () => string;
    private callbacks: BedrockRealmsManagerCallbacks;

    constructor(selectedInterfaceGetter: () => string, callbacks: BedrockRealmsManagerCallbacks) {
        this.selectedInterface = selectedInterfaceGetter;
        this.callbacks = callbacks;

        this.realmsRunningStore = writable(false);
        this.realmsStore = writable([]);
        this.favoritesStore = writable(new Set());
        this.isLoadingRealmsStore = writable(false);
        this.activeRealmIdStore = writable(null);
        this.activeRealmNameStore = writable('');

        this.realmsRunning = { subscribe: this.realmsRunningStore.subscribe };
        this.realms = { subscribe: this.realmsStore.subscribe };
        this.favorites = { subscribe: this.favoritesStore.subscribe };
        this.isLoadingRealms = { subscribe: this.isLoadingRealmsStore.subscribe };
        this.activeRealmId = { subscribe: this.activeRealmIdStore.subscribe };
        this.activeRealmName = { subscribe: this.activeRealmNameStore.subscribe };

        this.sortedRealms = derived(
            [this.realmsStore, this.favoritesStore],
            ([$realms, $favorites]) =>
                [...$realms].sort((a, b) => {
                    const aFav = $favorites.has(String(a.id)) ? 0 : 1;
                    const bFav = $favorites.has(String(b.id)) ? 0 : 1;
                    return aFav - bFav || a.name.localeCompare(b.name);
                }),
        );
    }

    async initialize(store: Store): Promise<void> {
        this.store = store;
        const savedFavs = await store.get<string[]>('bedrock_realm_favorites');
        if (savedFavs) {
            this.favoritesStore.set(new Set(savedFavs));
        }
    }

    isRunning(): boolean {
        return get(this.realmsRunningStore);
    }

    applyStatus(activeId: bigint | null, activeName: string | null, running: boolean): void {
        this.realmsRunningStore.set(running);
        if (activeId !== null && activeId !== undefined) {
            this.activeRealmIdStore.set(activeId);
        }
        if (activeName) {
            this.activeRealmNameStore.set(activeName);
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
        this.callbacks.clearConnectionError();
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

    async toggleFavorite(realmId: bigint): Promise<void> {
        const key = String(realmId);
        this.favoritesStore.update((current) => {
            const next = new Set(current);
            if (next.has(key)) {
                next.delete(key);
            } else {
                next.add(key);
            }
            return next;
        });

        if (this.store) {
            await this.store.set(
                'bedrock_realm_favorites',
                [...get(this.favoritesStore)],
            );
            await this.store.save();
        }
    }

    async connectToRealm(realm: RealmEntry): Promise<void> {
        this.callbacks.setStatus('');
        this.callbacks.clearLogs();
        this.callbacks.clearConnectionError();
        try {
            try {
                await invoke('bedrock_force_refresh');
            } catch (e) {
                logError(`Token refresh before realm connect failed: ${e}`);
            }
            await invoke('bedrock_start_realms', {
                realmId: Number(realm.id),
                realmName: realm.name,
                networkInterface: this.selectedInterface(),
            });
            this.realmsRunningStore.set(true);
            this.activeRealmIdStore.set(realm.id);
            this.activeRealmNameStore.set(realm.name);
            this.callbacks.setStatus(`Connected to ${realm.name}`);
            info(`Bedrock realms started: ${realm.name} (${realm.id})`);
        } catch (e) {
            this.callbacks.setStatus(`Error: ${e}`);
            logError(`Realms start failed: ${e}`);
            this.callbacks.reportError(String(e));
        }
    }

    async stopRealms(): Promise<void> {
        try {
            await invoke('bedrock_stop_realms');
            this.realmsRunningStore.set(false);
            this.activeRealmIdStore.set(null);
            this.activeRealmNameStore.set('');
            this.callbacks.setStatus('Disconnected');
        } catch (e) {
            this.callbacks.setStatus(`Error stopping: ${e}`);
        }
    }

    reset(): void {
        this.realmsStore.set([]);
        this.realmsRunningStore.set(false);
        this.activeRealmIdStore.set(null);
        this.activeRealmNameStore.set('');
    }
}
