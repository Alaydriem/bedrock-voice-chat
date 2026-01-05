import type { PlayerGainSettings } from '../../../bindings/PlayerGainSettings';
import type { PlayerGainStore } from '../../../bindings/PlayerGainStore';
import type { PlayerSource } from '../../../bindings/PlayerSource';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { Store } from '@tauri-apps/plugin-store';
import { debug, info, error } from '@tauri-apps/plugin-log';
import { invoke } from '@tauri-apps/api/core';
import type { PlayerManager } from '../../managers/PlayerManager';

export class PlayerPresenceManager {
    private store: Store;
    private playerManager: PlayerManager;
    private unlisten?: () => void;
    private isInitialized = false;
    private syncInterval?: ReturnType<typeof setInterval>;

    constructor(store: Store, playerManager: PlayerManager) {
        this.store = store;
        this.playerManager = playerManager;
    }

    async initialize(): Promise<void> {
        if (this.isInitialized) {
            return;
        }

        this.cleanup();

        this.unlisten = await getCurrentWebviewWindow().listen("player_presence", (event: any) => {
            this.handlePresenceEvent(event);
        });

        // Initial sync from backend
        await this.syncCurrentPlayers();

        // Periodic sync every 30 seconds as safety net
        this.syncInterval = setInterval(() => this.syncCurrentPlayers(), 10000);

        this.isInitialized = true;
    }

    private async syncCurrentPlayers(): Promise<void> {
        try {
            const backendPlayers = new Set(await invoke<string[]>("get_current_players"));
            const frontendPlayers = this.playerManager.getAll();
            const frontendPlayerNames = new Set(frontendPlayers.map(p => p.name));

            // Calculate differences
            const toAdd: string[] = [];
            const toRemove: string[] = [];

            for (const playerName of backendPlayers) {
                // Only add if not already present with Proximity source
                if (!this.playerManager.hasPlayerSource(playerName, 'Proximity')) {
                    toAdd.push(playerName);
                }
            }

            for (const playerName of frontendPlayerNames) {
                // Only remove Proximity source if backend doesn't have them
                if (!backendPlayers.has(playerName) && this.playerManager.hasPlayerSource(playerName, 'Proximity')) {
                    toRemove.push(playerName);
                }
            }

            // Skip if no changes needed
            if (toAdd.length === 0 && toRemove.length === 0) {
                return;
            }

            // Apply changes
            for (const playerName of toAdd) {
                const settings = await this.getPlayerSettings(playerName);
                await this.playerManager.addPlayerSource(playerName, 'Proximity', settings);
            }

            for (const playerName of toRemove) {
                this.playerManager.removePlayerSource(playerName, 'Proximity');
            }
        } catch (err) {
            error(`Failed to sync current players: ${err}`);
        }
    }

    private async handlePresenceEvent(event: any): Promise<void> {
        const payload = event.payload;
        if (!payload) {
            error("Player presence event received with no payload");
            return;
        }

        // Support both 'player' (from auto-detection) and 'player_name' (from server events)
        const playerName = payload.player || payload.player_name;
        const status = payload.status;

        if (!playerName) {
            error(`Player presence event missing player name: ${JSON.stringify(payload)}`);
            return;
        }

        if (!status) {
            error(`Player presence event missing status: ${JSON.stringify(payload)}`);
            return;
        }

        if (status === 'joined') {
            const settings = await this.getPlayerSettings(playerName);

            // Use source-aware addition with 'Proximity' source for audio detection
            const success = await this.playerManager.addPlayerSource(playerName, 'Proximity', settings);
            if (success) {
                await this.savePlayerToStore(playerName, settings);
            }
        } else if (status === 'disconnected') {
            // Remove only from 'Proximity' source
            const success = this.playerManager.removePlayerSource(playerName, 'Proximity');
            if (success) {
                // Only remove from persistent store if player has no remaining sources
                if (!this.playerManager.has(playerName)) {
                    await this.removePlayerFromStore(playerName);
                }
            }
        } else {
            error(`Unknown presence status: ${status} for player ${playerName}`);
        }
    }

    private async getPlayerSettings(playerName: string): Promise<PlayerGainSettings> {
        try {
            const playerGainStore = await this.store.get("player_gain_store") as PlayerGainStore || {};
            return playerGainStore[playerName] || { gain: 1.0, muted: false };
        } catch (err) {
            error(`Failed to get player settings for ${playerName}: ${err}`);
            return { gain: 1.0, muted: false };
        }
    }

    private async savePlayerToStore(playerName: string, settings: PlayerGainSettings): Promise<void> {
        try {
            let playerGainStore = await this.store.get("player_gain_store") as PlayerGainStore || {};
            playerGainStore[playerName] = settings;

            await this.store.set("player_gain_store", playerGainStore);
            await this.store.save();

            await invoke("update_stream_metadata", {
                key: "player_gain_store",
                value: JSON.stringify(playerGainStore),
                device: "OutputDevice"
            });
        } catch (err) {
            error(`Failed to save player ${playerName} to store: ${err}`);
        }
    }

    private async removePlayerFromStore(playerName: string): Promise<void> {
        try {
            let playerGainStore = await this.store.get("player_gain_store") as PlayerGainStore || {};
            delete playerGainStore[playerName];

            await this.store.set("player_gain_store", playerGainStore);
            await this.store.save();

            await invoke("update_stream_metadata", {
                key: "player_gain_store",
                value: JSON.stringify(playerGainStore),
                device: "OutputDevice"
            });
        } catch (err) {
            error(`Failed to remove player ${playerName} from store: ${err}`);
        }
    }

    async updatePlayerGain(playerName: string, gain: number): Promise<void> {
        const player = this.playerManager.get(playerName);
        if (!player) {
            error(`Player ${playerName} not found in store`);
            return;
        }

        const newSettings = { ...player.settings, gain };
        this.playerManager.update(playerName, newSettings);
        await this.savePlayerToStore(playerName, newSettings);
    }

    async updatePlayerMute(playerName: string, muted: boolean): Promise<void> {
        const player = this.playerManager.get(playerName);
        if (!player) {
            error(`Player ${playerName} not found in store`);
            return;
        }

        const newSettings = { ...player.settings, muted };
        this.playerManager.update(playerName, newSettings);
        await this.savePlayerToStore(playerName, newSettings);
    }

    getActivePlayerCount(): number {
        return this.playerManager.size();
    }

    getActivePlayerNames(): string[] {
        return this.playerManager.getAll().map((p: any) => p.name);
    }

    isPlayerActive(playerName: string): boolean {
        return this.playerManager.has(playerName);
    }

    cleanup(): void {
        if (this.syncInterval) {
            clearInterval(this.syncInterval);
            this.syncInterval = undefined;
        }

        if (this.unlisten) {
            try {
                this.unlisten();
            } catch (err) {
                error(`Error cleaning up event listener: ${err}`);
            }
            this.unlisten = undefined;
        }

        this.isInitialized = false;
    }
}
