import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { info, error, debug, warn } from '@tauri-apps/plugin-log';
import type { PlayerGainSettings } from '../../bindings/PlayerGainSettings';
import type { PlayerGainStore } from '../../bindings/PlayerGainStore';
import type { PlayerSource } from '../../bindings/PlayerSource';
import type { Store } from '@tauri-apps/plugin-store';

// Define PlayerData interface locally
interface PlayerData {
    name: string;
    settings: PlayerGainSettings;
    sources: Set<PlayerSource>;
}

/**
 * PlayerManager handles all player state and business logic.
 * Consolidates player presence, multi-source tracking, and audio controls.
 */
export class PlayerManager {
    // Internal reactive stores
    private playersMapStore: Writable<Map<string, PlayerData>>;
    private currentUserStore: Writable<string>;
    private store: Store;

    // Readonly exports for components
    public readonly playersMap: Readable<Map<string, PlayerData>>;
    public readonly currentUser: Readable<string>;
    public readonly activePlayers: Readable<PlayerData[]>;

    constructor(store: Store, currentUser: string = '') {
        // Initialize internal stores
        this.playersMapStore = writable(new Map<string, PlayerData>());
        this.currentUserStore = writable(currentUser);
        this.store = store;

        // Create readonly exports
        this.playersMap = { subscribe: this.playersMapStore.subscribe };
        this.currentUser = { subscribe: this.currentUserStore.subscribe };

        // Create derived store for active players (excluding current user)
        this.activePlayers = derived(
            [this.playersMapStore, this.currentUserStore],
            ([playersMap, currentUser]) => {
                const players = Array.from(playersMap.values());
                return players.filter(player => player.name !== currentUser);
            }
        );

        info(`PlayerManager: Initialized with current user: ${currentUser || 'none'}`);
    }

    /**
     * Set the current user name
     */
    setCurrentUser(name: string): void {
        debug(`PlayerManager: Setting current user to: ${name}`);
        this.currentUserStore.set(name);
    }

    /**
     * Get the current user name
     */
    getCurrentUser(): string {
        return get(this.currentUserStore);
    }

    /**
     * Add a player to the store
     */
    add(name: string, settings?: PlayerGainSettings): boolean {
        try {
            const playerSettings = settings || { gain: 1.0, muted: false };
            
            this.playersMapStore.update(map => {
                map.set(name, { 
                    name, 
                    settings: playerSettings, 
                    sources: new Set() 
                });
                debug(`PlayerManager: Added player: ${name}`);
                return new Map(map);
            });
            return true;
        } catch (err) {
            error(`PlayerManager: Failed to add player ${name}: ${err}`);
            return false;
        }
    }

    /**
     * Remove a player from the store
     */
    remove(name: string): boolean {
        try {
            this.playersMapStore.update(map => {
                const removed = map.delete(name);
                debug(`PlayerManager: Removed player: ${name}, success: ${removed}`);
                return new Map(map);
            });
            return true;
        } catch (err) {
            error(`PlayerManager: Failed to remove player ${name}: ${err}`);
            return false;
        }
    }

    /**
     * Update player settings
     */
    update(name: string, settings: Partial<PlayerGainSettings>): boolean {
        try {
            this.playersMapStore.update(map => {
                const player = map.get(name);
                if (player) {
                    player.settings = { ...player.settings, ...settings };
                    map.set(name, { ...player });
                    debug(`PlayerManager: Updated player ${name} settings: ${JSON.stringify(settings)}`);
                } else {
                    warn(`PlayerManager: Player ${name} not found for update`);
                }
                return new Map(map);
            });
            return true;
        } catch (err) {
            error(`PlayerManager: Failed to update player ${name}: ${err}`);
            return false;
        }
    }

    /**
     * Check if a player exists
     */
    has(name: string): boolean {
        const currentMap = get(this.playersMapStore);
        return currentMap.has(name);
    }

    /**
     * Get a specific player
     */
    get(name: string): PlayerData | undefined {
        const currentMap = get(this.playersMapStore);
        return currentMap.get(name);
    }

    /**
     * Clear all players
     */
    clear(): void {
        this.playersMapStore.set(new Map());
        debug('PlayerManager: Cleared all players');
    }

    /**
     * Get all players as an array
     */
    getAll(): PlayerData[] {
        const currentMap = get(this.playersMapStore);
        return Array.from(currentMap.values());
    }

    /**
     * Get the number of players
     */
    size(): number {
        const currentMap = get(this.playersMapStore);
        return currentMap.size;
    }

    /**
     * Load player settings from persistent store
     */
    async loadPlayerSettings(playerName: string): Promise<PlayerGainSettings> {
        if (!this.store) {
            warn(`PlayerManager: Store not available, using defaults for ${playerName}`);
            return { gain: 1.0, muted: false };
        }

        try {
            const playerGainStore = await this.store.get("player_gain_store") as PlayerGainStore || {};
            const settings = playerGainStore[playerName] || { gain: 1.0, muted: false };
            debug(`PlayerManager: Loaded settings for ${playerName}: gain=${settings.gain}, muted=${settings.muted}`);
            return settings;
        } catch (err) {
            error(`PlayerManager: Failed to load settings for ${playerName}: ${err}`);
            return { gain: 1.0, muted: false };
        }
    }

    /**
     * Add a source to a player, creating the player if it doesn't exist
     * If no settings provided, will load from persistent store
     */
    async addPlayerSource(name: string, source: PlayerSource, settings?: PlayerGainSettings): Promise<boolean> {
        try {
            // Load settings if not provided
            const playerSettings = settings || await this.loadPlayerSettings(name);
            
            this.playersMapStore.update(map => {
                const existing = map.get(name);
                if (existing) {
                    // Player exists, just add the source
                    existing.sources.add(source);
                    map.set(name, { ...existing });
                    debug(`PlayerManager: Added ${source} source to existing player: ${name}`);
                } else {
                    // New player, create with this source and loaded settings
                    map.set(name, { 
                        name, 
                        settings: playerSettings, 
                        sources: new Set([source]) 
                    });
                    debug(`PlayerManager: Created new player ${name} with ${source} source and settings: gain=${playerSettings.gain}, muted=${playerSettings.muted}`);
                }
                return new Map(map);
            });
            return true;
        } catch (err) {
            error(`PlayerManager: Failed to add ${source} source for player ${name}: ${err}`);
            return false;
        }
    }

    /**
     * Remove a source from a player, removing the player entirely if no sources remain
     */
    removePlayerSource(name: string, source: PlayerSource): boolean {
        try {
            info(`PlayerManager: REMOVE_SOURCE DEBUG: Attempting to remove ${source} source from player ${name}`);
            
            this.playersMapStore.update(map => {
                const existing = map.get(name);
                info(`PlayerManager: REMOVE_SOURCE DEBUG: Player ${name} exists in map: ${!!existing}`);
                if (existing) {
                    info(`PlayerManager: REMOVE_SOURCE DEBUG: Player ${name} current sources: [${Array.from(existing.sources).join(', ')}]`);
                    if (existing.sources.has(source)) {
                        existing.sources.delete(source);
                        info(`PlayerManager: REMOVE_SOURCE DEBUG: Successfully removed ${source} source from player ${name}`);
                        
                        if (existing.sources.size === 0) {
                            // No more sources, remove player entirely
                            map.delete(name);
                            info(`PlayerManager: REMOVE_SOURCE DEBUG: Player ${name} removed completely (no remaining sources)`);
                        } else {
                            // Still has other sources, keep player
                            map.set(name, { ...existing });
                            info(`PlayerManager: REMOVE_SOURCE DEBUG: Player ${name} still has sources: [${Array.from(existing.sources).join(', ')}]`);
                        }
                    } else {
                        info(`PlayerManager: REMOVE_SOURCE DEBUG: Player ${name} does not have ${source} source, skipping removal`);
                    }
                } else {
                    info(`PlayerManager: REMOVE_SOURCE DEBUG: Player ${name} not found in map`);
                }
                return new Map(map);
            });
            
            info(`PlayerManager: REMOVE_SOURCE DEBUG: Returning true for ${name}`);
            return true;
        } catch (err) {
            error(`PlayerManager: Failed to remove ${source} source for player ${name}: ${err}`);
            return false;
        }
    }

    /**
     * Check if a player has a specific source
     */
    hasPlayerSource(name: string, source: PlayerSource): boolean {
        const player = this.get(name);
        return player?.sources.has(source) || false;
    }

    /**
     * Get all sources for a player
     */
    getPlayerSources(name: string): Set<PlayerSource> {
        const player = this.get(name);
        return player?.sources || new Set();
    }

    /**
     * Update player gain setting
     */
    async updatePlayerGain(playerName: string, gain: number): Promise<void> {
        if (!this.store) {
            error("PlayerManager: Tauri store not initialized");
            return;
        }

        try {
            // Get current player to preserve muted state
            const currentPlayer = this.get(playerName);
            const currentMuted = currentPlayer?.settings.muted || false;
            
            // Update reactive store
            this.update(playerName, { gain });

            // Update persistent store
            await this.updatePlayerGainStore(playerName, { gain, muted: currentMuted });
            debug(`PlayerManager: Updated gain for ${playerName} to ${gain}`);
        } catch (err) {
            error(`PlayerManager: Failed to update player gain: ${err}`);
        }
    }

    /**
     * Update player mute setting
     */
    async updatePlayerMute(playerName: string, muted: boolean): Promise<void> {
        if (!this.store) {
            error("PlayerManager: Tauri store not initialized");
            return;
        }

        try {
            // Get current player to preserve gain
            const currentPlayer = this.get(playerName);
            const currentGain = currentPlayer?.settings.gain || 1.0;
            
            // Update reactive store
            this.update(playerName, { muted });

            // Update persistent store
            await this.updatePlayerGainStore(playerName, { gain: currentGain, muted });
            debug(`PlayerManager: Updated mute for ${playerName} to ${muted}`);
        } catch (err) {
            error(`PlayerManager: Failed to update player mute: ${err}`);
        }
    }

    /**
     * Private method to update the persistent Tauri store
     */
    private async updatePlayerGainStore(playerName: string, newSettings: Partial<PlayerGainSettings>): Promise<void> {
        if (!this.store) return;

        try {
            // Get current store
            let playerGainStore = await this.store.get("player_gain_store") as PlayerGainStore || {};
            
            // Get existing settings or defaults
            const existingSettings = playerGainStore[playerName] || { gain: 1.0, muted: false };
            
            // Merge with new settings
            const updatedSettings = { ...existingSettings, ...newSettings };
            playerGainStore[playerName] = updatedSettings;
            
            // Save to Tauri store
            await this.store.set("player_gain_store", playerGainStore);
            await this.store.save();
            
            // Send to backend
            await invoke("update_stream_metadata", {
                key: "player_gain_store",
                value: JSON.stringify(playerGainStore),
                device: "OutputDevice"
            });
            
            debug(`PlayerManager: Updated persistent store for ${playerName}`);
        } catch (err) {
            error(`PlayerManager: Failed to update player gain store: ${err}`);
        }
    }

    async loadFromPersistentStore(): Promise<void> {
        if (!this.store) {
            warn("PlayerManager: Tauri store not available for loading");
            return;
        }

        try {
            const playerGainStore = await this.store.get("player_gain_store") as PlayerGainStore || {};
            
            // Update settings for existing players
            this.playersMapStore.update(map => {
                for (const [playerName, settings] of Object.entries(playerGainStore)) {
                    const player = map.get(playerName);
                    if (player && settings) {
                        player.settings = settings;
                        map.set(playerName, { ...player });
                    }
                }
                return new Map(map);
            });

            info(`PlayerManager: Loaded settings for ${Object.keys(playerGainStore).length} players from persistent store`);
        } catch (err) {
            error(`PlayerManager: Failed to load from persistent store: ${err}`);
        }
    }
}