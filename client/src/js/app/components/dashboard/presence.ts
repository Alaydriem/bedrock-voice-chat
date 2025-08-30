import type { PlayerGainSettings } from '../../../bindings/PlayerGainSettings';
import type { PlayerGainStore } from '../../../bindings/PlayerGainStore';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { Store } from '@tauri-apps/plugin-store';
import { debug, info, error } from '@tauri-apps/plugin-log';
import { invoke } from '@tauri-apps/api/core';
import { playerStore, setTauriStore } from '../../../../stores/players';

export class PlayerPresenceManager {
    private store: Store;
    private unlisten?: () => void;
    private isInitialized = false;
    
    constructor(store: Store) {
        this.store = store;
    }

    async initialize(): Promise<void> {
        if (this.isInitialized) {
            debug("PlayerPresenceManager already initialized, skipping");
            return;
        }

        debug("Initializing simplified PlayerPresenceManager...");
        
        setTauriStore(this.store);
        
        this.cleanup();
        
        this.unlisten = await getCurrentWebviewWindow().listen("player_presence", (event: any) => {
            this.handlePresenceEvent(event);
        });
        
        this.isInitialized = true;
        info("PlayerPresenceManager initialized successfully");
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
        
        info(`Processing player presence: ${playerName} - ${status}`);
        
        if (status === 'joined') {
            if (!playerStore.has(playerName)) {
                debug(`Adding new player: ${playerName}`);
                const settings = await this.getPlayerSettings(playerName);
                playerStore.add(playerName, settings);
                
                await this.savePlayerToStore(playerName, settings);
            } else {
                debug(`Player ${playerName} already exists, skipping add`);
            }
        } else if (status === 'disconnected') {
            debug(`Removing player: ${playerName}`);
            playerStore.remove(playerName);
            
            await this.removePlayerFromStore(playerName);
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
            
            debug(`Saved player ${playerName} to persistent store`);
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
            
            debug(`Removed player ${playerName} from persistent store`);
        } catch (err) {
            error(`Failed to remove player ${playerName} from store: ${err}`);
        }
    }
    
    async updatePlayerGain(playerName: string, gain: number): Promise<void> {
        const player = playerStore.get(playerName);
        if (!player) {
            error(`Player ${playerName} not found in store`);
            return;
        }
        
        const newSettings = { ...player.settings, gain };
        playerStore.update(playerName, newSettings);
        await this.savePlayerToStore(playerName, newSettings);
        
        debug(`Updated ${playerName} gain to ${gain}`);
    }
    
    async updatePlayerMute(playerName: string, muted: boolean): Promise<void> {
        const player = playerStore.get(playerName);
        if (!player) {
            error(`Player ${playerName} not found in store`);
            return;
        }
        
        const newSettings = { ...player.settings, muted };
        playerStore.update(playerName, newSettings);
        await this.savePlayerToStore(playerName, newSettings);
        
        debug(`${muted ? 'Muted' : 'Unmuted'} ${playerName}`);
    }
    
    getActivePlayerCount(): number {
        return playerStore.size();
    }
    
    getActivePlayerNames(): string[] {
        return playerStore.getAll().map((p: any) => p.name);
    }
    
    isPlayerActive(playerName: string): boolean {
        return playerStore.has(playerName);
    }
    
    cleanup(): void {
        if (this.unlisten) {
            try {
                this.unlisten();
                debug("Player presence event listener cleaned up");
            } catch (err) {
                error(`Error cleaning up event listener: ${err}`);
            }
            this.unlisten = undefined;
        }
        
        this.isInitialized = false;
        debug("PlayerPresenceManager cleanup complete");
    }
}
