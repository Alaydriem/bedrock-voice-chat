import { I18n } from "$lib/i18n";
import type { PlayerGainSettings } from '../../../bindings/PlayerGainSettings';
import type { PlayerSource } from '../../../bindings/PlayerSource';
import type { GamerpicResponse } from '../../../bindings/GamerpicResponse';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { debug, info, error } from '@charlesportwoodii/tauri-plugin-curia';
import { invoke } from '@tauri-apps/api/core';
import type { PlayerManager } from '../../managers/PlayerManager';
import ImageCache from '../imageCache';
import ImageCacheOptions from '../imageCacheOptions';
import GameNameUtils from '../../utils/GameNameUtils';

export class PlayerPresenceManager {
    private playerManager: PlayerManager;
    private unlisten?: () => void;
    private isInitialized = false;
    private syncInterval?: ReturnType<typeof setInterval>;
    private imageCache: ImageCache = new ImageCache();
    private gamerpicFetchInProgress: Set<string> = new Set();

    constructor(playerManager: PlayerManager) {
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
            const backendPlayersMap = await invoke<Record<string, string | null>>("get_current_players");
            const backendPlayerNames = new Set(Object.keys(backendPlayersMap));
            const frontendPlayers = this.playerManager.getAll();
            const frontendPlayerNames = new Set(frontendPlayers.map(p => p.name));

            // Calculate differences
            const toAdd: string[] = [];
            const toRemove: string[] = [];

            for (const playerName of backendPlayerNames) {
                // Only add if not already present with Proximity source
                if (!this.playerManager.hasPlayerSource(playerName, 'Proximity')) {
                    toAdd.push(playerName);
                }
            }

            for (const playerName of frontendPlayerNames) {
                // Only remove Proximity source if backend doesn't have them
                if (!backendPlayerNames.has(playerName) && this.playerManager.hasPlayerSource(playerName, 'Proximity')) {
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
                const playerGame = backendPlayersMap[playerName] ?? undefined;
                await this.playerManager.addPlayerSource(playerName, 'Proximity', settings, undefined, playerGame);
                this.fetchAndSetGamepic(playerName, playerGame);
            }

            for (const playerName of toRemove) {
                this.playerManager.removePlayerSource(playerName, 'Proximity');
            }

            // Retry gamerpic fetch for existing proximity players missing one
            for (const playerName of backendPlayerNames) {
                const player = this.playerManager.get(playerName);
                if (player && !player.gamerpic) {
                    this.fetchAndSetGamepic(playerName, backendPlayersMap[playerName] ?? undefined);
                }
            }
        } catch (err) {
            error("failed to sync current players", {
                error: String(err),
            });
        }
    }

    private async handlePresenceEvent(event: any): Promise<void> {
        const payload = event.payload;
        if (!payload) {
            error(I18n.t("Player presence event received with no payload"));
            return;
        }

        // Support both 'player' (from auto-detection) and 'player_name' (from server events)
        const rawName = payload.player || payload.player_name;
        const status = payload.status;
        const game: string | undefined = payload.game ?? undefined;

        // Composed once, here, rather than at each of the three store helpers below. The
        // presence event's name form varies with which producer emitted it, and the gain store
        // is keyed on the canonical identity — so the boundary is the place to settle it.
        const playerName = rawName ? GameNameUtils.canonical(rawName, game ?? 'minecraft') : rawName;

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
            const success = await this.playerManager.addPlayerSource(playerName, 'Proximity', settings, undefined, game);
            if (success) {
                // Fire-and-forget gamerpic fetch
                this.fetchAndSetGamepic(playerName, game);
            }
        } else if (status === 'disconnected') {
            // Remove only from 'Proximity' source. Their settings are deliberately left
            // behind: a volume you set is about the person, not about them being in earshot,
            // and the settings pane exists so you can change it after they have gone. The
            // store's pruner drops the rows nobody decided anything about.
            this.playerManager.removePlayerSource(playerName, 'Proximity');
        } else {
            error(`Unknown presence status: ${status} for player ${playerName}`);
        }
    }

    /**
     * This player's persisted volume and mute, stamping them as seen in the same call.
     *
     * One round trip because both halves are the same fact: a presence event means the
     * player is around, which is what `last_seen` records, and the card needs the settings
     * that arrival should render with. The stamp is coalesced behind a debounce on the Rust
     * side, so a server-join storm does not become a write storm.
     */
    private async getPlayerSettings(playerName: string): Promise<PlayerGainSettings> {
        try {
            return await invoke<PlayerGainSettings>('player_settings_touch', { cn: playerName });
        } catch (err) {
            error("failed to get player settings", {
                player: playerName,
                error: String(err),
            });
            return { gain: 1.0, muted: false, last_seen: null };
        }
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

    private async fetchAndSetGamepic(playerName: string, gameOverride?: string): Promise<void> {
        // Prevent duplicate fetches for the same player
        if (this.gamerpicFetchInProgress.has(playerName)) {
            return;
        }
        this.gamerpicFetchInProgress.add(playerName);

        try {
            const game = gameOverride ?? GameNameUtils.extractGame(playerName);
            const gamertag = GameNameUtils.stripPrefix(playerName);

            // Ask the server for the gamerpic URL
            const response = await invoke<GamerpicResponse>('api_get_player_gamerpic', {
                game,
                gamertag
            });

            if (!response.gamerpic) {
                return;
            }

            const options = new ImageCacheOptions(response.gamerpic, 2592000);
            const dataUrl = await this.imageCache.getImage(options);
            this.playerManager.updatePlayerGamepic(playerName, dataUrl);
        } catch (err) {
            debug(`Failed to fetch gamerpic for ${playerName}: ${err}`);
        } finally {
            this.gamerpicFetchInProgress.delete(playerName);
        }
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
