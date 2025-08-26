import { writable, derived, get } from 'svelte/store';
import type { PlayerGainSettings } from '../js/bindings/PlayerGainSettings';
import type { PlayerGainStore } from '../js/bindings/PlayerGainStore';
import { Store } from '@tauri-apps/plugin-store';
import { invoke } from '@tauri-apps/api/core';
import { info, error } from '@tauri-apps/plugin-log';

interface PlayerData {
    name: string;
    settings: PlayerGainSettings;
}

// Simple reactive store
export const playersMap = writable<Map<string, PlayerData>>(new Map());

// Derived store for easy component consumption
export const activePlayers = derived(playersMap, $map => Array.from($map.values()));

// Store reference for persistence
let tauriStore: Store | null = null;

export function setTauriStore(store: Store) {
    tauriStore = store;
}

export const playerStore = {
    add: (name: string, settings: PlayerGainSettings) => {
        playersMap.update(map => {
            map.set(name, { name, settings });
            return new Map(map); // Trigger reactivity
        });
    },
    
    remove: (name: string) => {
        playersMap.update(map => {
            map.delete(name);
            return new Map(map);
        });
    },
    
    update: (name: string, settings: PlayerGainSettings) => {
        playersMap.update(map => {
            const existing = map.get(name);
            if (existing) {
                map.set(name, { ...existing, settings });
            }
            return new Map(map);
        });
    },
    
    has: (name: string): boolean => {
        const currentMap = get(playersMap);
        return currentMap.has(name);
    },
    
    get: (name: string): PlayerData | undefined => {
        const currentMap = get(playersMap);
        return currentMap.get(name);
    },
    
    clear: () => playersMap.set(new Map()),
    
    getAll: (): PlayerData[] => {
        const currentMap = get(playersMap);
        return Array.from(currentMap.values());
    },
    
    size: (): number => {
        const currentMap = get(playersMap);
        return currentMap.size;
    }
};

// Audio control functions
export async function updatePlayerGain(playerName: string, gain: number): Promise<void> {
    if (!tauriStore) {
        error("Tauri store not initialized");
        return;
    }

    try {
        // Get current player to preserve muted state
        const currentPlayer = playerStore.get(playerName);
        const currentMuted = currentPlayer?.settings.muted || false;
        
        // Update reactive store
        playersMap.update(map => {
            const player = map.get(playerName);
            if (player) {
                player.settings.gain = gain;
                map.set(playerName, { ...player });
            }
            return new Map(map);
        });

        // Update persistent store
        await updatePlayerGainStore(playerName, { gain, muted: currentMuted });
    } catch (err) {
        error(`Failed to update player gain: ${err}`);
    }
}

export async function updatePlayerMute(playerName: string, muted: boolean): Promise<void> {
    if (!tauriStore) {
        error("Tauri store not initialized");
        return;
    }

    try {
        // Get current player to preserve gain
        const currentPlayer = playerStore.get(playerName);
        const currentGain = currentPlayer?.settings.gain || 1.0;
        
        // Update reactive store
        playersMap.update(map => {
            const player = map.get(playerName);
            if (player) {
                player.settings.muted = muted;
                map.set(playerName, { ...player });
            }
            return new Map(map);
        });

        // Update persistent store
        await updatePlayerGainStore(playerName, { gain: currentGain, muted });
    } catch (err) {
        error(`Failed to update player mute: ${err}`);
    }
}

async function updatePlayerGainStore(playerName: string, newSettings: Partial<PlayerGainSettings>): Promise<void> {
    if (!tauriStore) return;

    try {
        // Get current store
        let playerGainStore = await tauriStore.get("player_gain_store") as PlayerGainStore || {};
        
        // Get existing settings or defaults
        const existingSettings = playerGainStore[playerName] || { gain: 1.0, muted: false };
        
        // Merge with new settings
        const updatedSettings = { ...existingSettings, ...newSettings };
        playerGainStore[playerName] = updatedSettings;
        
        // Save to Tauri store
        await tauriStore.set("player_gain_store", playerGainStore);
        await tauriStore.save();
        
        // Send to backend
        await invoke("update_stream_metadata", {
            key: "player_gain_store",
            value: JSON.stringify(playerGainStore),
            device: "OutputDevice"
        });
    } catch (err) {
        error(`Failed to update player gain store: ${err}`);
    }
}
