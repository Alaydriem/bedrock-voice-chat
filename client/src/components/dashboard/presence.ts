import type { PlayerGainSettings } from '../../js/bindings/PlayerGainSettings';
import type { PlayerGainStore } from '../../js/bindings/PlayerGainStore';
import { mount } from 'svelte';
import PlayerPresence from '../events/PlayerPresence.svelte';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { Store } from '@tauri-apps/plugin-store';
import { info, error } from '@tauri-apps/plugin-log';

interface PlayerState {
    name: string;
    settings: PlayerGainSettings;
    component?: any;
    lastSeen: number;
}

interface SessionPlayerData {
    activePlayers: string[];
    lastUpdated: number;
    serverContext?: string;
}

export class PlayerPresenceManager {
    private playerStates = new Map<string, PlayerState>();
    private eventUnlistener: (() => void) | null = null;
    private sessionStore: Storage = sessionStorage;
    private store: Store;
    private containerSelector: string;
    private isInitialized = false;
    
    private static readonly SESSION_KEY = 'bvc_active_players';
    private static readonly MAX_AGE = 1000 * 60 * 30; // 30 minutes

    constructor(store: Store, containerSelector: string = "#player-presence-container") {
        this.store = store;
        this.containerSelector = containerSelector;
    }

    async initialize(): Promise<void> {
        if (this.isInitialized) return;

        const appWebview = getCurrentWebviewWindow();
        
        // Set up player presence event listener
        this.eventUnlistener = await appWebview.listen("player_presence", (event: { payload?: { player_name?: string, status?: string } }) => {
            info(`Player presence event received: ${JSON.stringify(event.payload)}`);
            this.handlePlayerPresence(event.payload?.player_name || '', event.payload?.status || '');
        });

        // Restore active players from session storage
        await this.restoreActivePlayersFromSession();

        // Register cleanup on page events
        this.registerCleanupEvents();

        this.isInitialized = true;
        info("PlayerPresenceManager initialized");
    }

    private registerCleanupEvents(): void {
        const cleanup = () => this.cleanup();
        
        window.addEventListener('beforeunload', cleanup);
        window.addEventListener('pagehide', cleanup);
        window.addEventListener('popstate', cleanup);
    }

    private async restoreActivePlayersFromSession(): Promise<void> {
        try {
            const activePlayers = this.getActivePlayersFromSession();
            if (activePlayers.length > 0) {
                info(`Restoring ${activePlayers.length} active players from session`);
                
                for (const playerName of activePlayers) {
                    await this.handlePlayerPresence(playerName, 'joined');
                }
            }
        } catch (err) {
            error(`Failed to restore active players: ${err}`);
        }
    }

    private async handlePlayerPresence(playerName: string, status: string): Promise<void> {
        const container = document.querySelector(this.containerSelector);
        if (!container) {
            error(`Container ${this.containerSelector} not found`);
            return;
        }

        if (status === 'joined') {
            if (!this.playerStates.has(playerName)) {
                const savedSettings = await this.getPlayerSettingsFromStore(playerName);
                
                const playerState: PlayerState = {
                    name: playerName,
                    settings: savedSettings,
                    lastSeen: Date.now()
                };

                const component = mount(PlayerPresence, {
                    target: container,
                    props: {
                        player: playerName,
                        initialGain: savedSettings.gain,
                        initialMuted: savedSettings.muted,
                        onGainChange: (gain: number) => this.updatePlayerGain(playerName, gain),
                        onMuteToggle: (muted: boolean) => this.updatePlayerMute(playerName, muted)
                    }
                });

                playerState.component = component;
                this.playerStates.set(playerName, playerState);
                
                this.saveActivePlayersToSession();
                info(`Player ${playerName} joined and mounted`);
            } else {
                // Update last seen timestamp
                const playerState = this.playerStates.get(playerName);
                if (playerState) {
                    playerState.lastSeen = Date.now();
                }
            }
        } else if (status === 'disconnected') {
            this.removePlayer(playerName);
        }
    }

    private async getPlayerSettingsFromStore(playerName: string): Promise<PlayerGainSettings> {
        try {
            const playerGainStore = await this.store.get("player_gain_store") as PlayerGainStore || {};
            return playerGainStore[playerName] || { gain: 1.0, muted: false };
        } catch (err) {
            error(`Failed to get player settings: ${err}`);
            return { gain: 1.0, muted: false };
        }
    }

    private removePlayer(playerName: string): void {
        const playerState = this.playerStates.get(playerName);
        if (playerState?.component) {
            playerState.component.$destroy();
        }
        this.playerStates.delete(playerName);
        
        this.saveActivePlayersToSession();
        info(`Player ${playerName} disconnected and unmounted`);
    }

    private async updatePlayerGain(playerName: string, gain: number): Promise<void> {
        const playerState = this.playerStates.get(playerName);
        if (!playerState) return;
        
        try {
            playerState.settings.gain = gain;
            playerState.lastSeen = Date.now();
            
            await this.updatePlayerGainStore(playerName, playerState.settings);
            info(`Updated ${playerName} gain to ${gain}`);
        } catch (err) {
            error(`Failed to update player gain: ${err}`);
        }
    }

    private async updatePlayerMute(playerName: string, muted: boolean): Promise<void> {
        const playerState = this.playerStates.get(playerName);
        if (!playerState) return;
        
        try {
            playerState.settings.muted = muted;
            playerState.lastSeen = Date.now();
            
            await this.updatePlayerGainStore(playerName, playerState.settings);
            info(`${muted ? 'Muted' : 'Unmuted'} ${playerName}`);
        } catch (err) {
            error(`Failed to update player mute: ${err}`);
        }
    }

    private async updatePlayerGainStore(playerName: string, settings: PlayerGainSettings): Promise<void> {
        let playerGainStore = await this.store.get("player_gain_store") as PlayerGainStore || {};
        
        playerGainStore[playerName] = settings;
        
        await this.store.set("player_gain_store", playerGainStore);
        await this.store.save();
        
        await invoke("update_stream_metadata", {
            key: "player_gain_store",
            value: JSON.stringify(playerGainStore),
            device: "OutputDevice"
        });
    }

    // Session Storage Methods
    private saveActivePlayersToSession(): void {
        try {
            const activePlayerNames = Array.from(this.playerStates.keys());
            const data: SessionPlayerData = {
                activePlayers: activePlayerNames,
                lastUpdated: Date.now()
            };
            this.sessionStore.setItem(PlayerPresenceManager.SESSION_KEY, JSON.stringify(data));
        } catch (err) {
            error(`Failed to save players to session: ${err}`);
        }
    }

    private getActivePlayersFromSession(): string[] {
        try {
            const dataJson = this.sessionStore.getItem(PlayerPresenceManager.SESSION_KEY);
            if (!dataJson) return [];
            
            const data = JSON.parse(dataJson) as SessionPlayerData;
            
            // Check if data is too old
            if (Date.now() - data.lastUpdated > PlayerPresenceManager.MAX_AGE) {
                this.clearSession();
                return [];
            }
            
            return data.activePlayers || [];
        } catch (err) {
            error(`Error reading player session: ${err}`);
            return [];
        }
    }

    // Public Methods
    getActivePlayerCount(): number {
        return this.playerStates.size;
    }

    getActivePlayerNames(): string[] {
        return Array.from(this.playerStates.keys());
    }

    isPlayerActive(playerName: string): boolean {
        return this.playerStates.has(playerName);
    }

    clearSession(): void {
        this.sessionStore.removeItem(PlayerPresenceManager.SESSION_KEY);
        info("Cleared player session storage");
    }

    cleanup(): void {
        if (!this.isInitialized) return;

        info("Cleaning up PlayerPresenceManager...");
        
        // Save current state to session before cleanup
        this.saveActivePlayersToSession();
        
        // Clean up event listener
        if (this.eventUnlistener) {
            try {
                this.eventUnlistener();
            } catch (err) {
                error(`Error cleaning up event listener: ${err}`);
            }
            this.eventUnlistener = null;
        }
        
        // Clean up player components
        this.playerStates.forEach((state, playerName) => {
            if (state.component) {
                try {
                    state.component.$destroy();
                } catch (err) {
                    error(`Error destroying component for ${playerName}: ${err}`);
                }
            }
        });
        
        this.playerStates.clear();
        this.isInitialized = false;
        
        info("PlayerPresenceManager cleanup complete");
    }
}
