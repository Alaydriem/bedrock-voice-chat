import { Store } from '@tauri-apps/plugin-store';
import type { ServerListEntry } from '../../bindings/ServerListEntry';

export type { ServerListEntry };

export class ServerListStore {
    private static readonly STORE_PATH = 'store.json';
    private static readonly KEY_SERVER_LIST = 'server_list';
    private static readonly KEY_CURRENT_SERVER = 'current_server';
    private static readonly KEY_CURRENT_PLAYER = 'current_player';
    private static readonly KEY_ACTIVE_GAME = 'active_game';

    private storePromise: Promise<Store> | null = null;

    private getStore(): Promise<Store> {
        if (!this.storePromise) {
            this.storePromise = Store.load(ServerListStore.STORE_PATH, { autoSave: false, defaults: {} });
        }
        return this.storePromise;
    }

    async getServerList(): Promise<ServerListEntry[]> {
        const store = await this.getStore();
        const list = await store.get(ServerListStore.KEY_SERVER_LIST) as ServerListEntry[] | null;
        return list ?? [];
    }

    async findEntry(server: string): Promise<ServerListEntry | undefined> {
        const list = await this.getServerList();
        return list.find((s) => s.server === server);
    }

    async setCurrent(entry: ServerListEntry): Promise<void> {
        const store = await this.getStore();
        await store.set(ServerListStore.KEY_CURRENT_SERVER, entry.server);
        await store.set(ServerListStore.KEY_CURRENT_PLAYER, entry.player);
        await store.set(ServerListStore.KEY_ACTIVE_GAME, entry.game ?? 'minecraft');
        await store.save();
    }

    async getCurrentServer(): Promise<string | null> {
        const store = await this.getStore();
        return await store.get(ServerListStore.KEY_CURRENT_SERVER) as string | null;
    }

    async removeServer(server: string): Promise<ServerListEntry[]> {
        const store = await this.getStore();
        const list = (await store.get(ServerListStore.KEY_SERVER_LIST) as ServerListEntry[] | null) ?? [];
        const filtered = list.filter((s) => s.server !== server);
        await store.set(ServerListStore.KEY_SERVER_LIST, filtered);

        const currentServer = await store.get(ServerListStore.KEY_CURRENT_SERVER) as string | null;
        if (currentServer === server) {
            await store.delete(ServerListStore.KEY_CURRENT_SERVER);
            await store.delete(ServerListStore.KEY_CURRENT_PLAYER);
            await store.delete(ServerListStore.KEY_ACTIVE_GAME);
        }
        await store.save();
        return filtered;
    }

    async addOrUpdateEntry(entry: ServerListEntry): Promise<void> {
        const store = await this.getStore();
        const list = (await store.get(ServerListStore.KEY_SERVER_LIST) as ServerListEntry[] | null) ?? [];
        const existing = list.find((s) => s.server === entry.server);
        if (existing) {
            existing.player = entry.player;
            existing.game = entry.game ?? existing.game;
        } else {
            list.push(entry);
        }
        await store.set(ServerListStore.KEY_SERVER_LIST, list);
        await store.save();
    }
}
