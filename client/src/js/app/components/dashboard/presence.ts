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
    
    constructor(store: Store, playerManager: PlayerManager) {
        this.store = store;
        this.playerManager = playerManager;
    }

    async initialize(): Promise<void> {
        if (this.isInitialized) {
            debug("PlayerPresenceManager already initialized, skipping");
            return;
        }

        debug("Initializing simplified PlayerPresenceManager...");
        
        this.cleanup();
        
        this.unlisten = await getCurrentWebviewWindow().listen("player_presence", (event: any) => {
            this.handlePresenceEvent(event);
        });
        
        this.isInitialized = true;
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
        const player = this.playerManager.get(playerName);
        if (!player) {
            error(`Player ${playerName} not found in store`);
            return;
        }
        
        const newSettings = { ...player.settings, gain };
        this.playerManager.update(playerName, newSettings);
        await this.savePlayerToStore(playerName, newSettings);
        
        debug(`Updated ${playerName} gain to ${gain}`);
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
        
        debug(`${muted ? 'Muted' : 'Unmuted'} ${playerName}`);
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
