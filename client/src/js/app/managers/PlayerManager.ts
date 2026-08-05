import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { info, error, debug, warn } from '@tauri-apps/plugin-log';
import type { PlayerGainSettings } from '../../bindings/PlayerGainSettings';
import type { PlayerGainStore } from '../../bindings/PlayerGainStore';
import type { PlayerSource } from '../../bindings/PlayerSource';
import type { Store } from '@tauri-apps/plugin-store';
import { Coalescer } from '../utils/Coalescer';
import GameNameUtils from '../utils/GameNameUtils';

// Define PlayerData interface locally
interface PlayerData {
    name: string;
    settings: PlayerGainSettings;
    sources: Set<PlayerSource>;
    gamerpic?: string;
    game?: string;
}

/**
 * PlayerManager handles all player state and business logic.
 * Consolidates player presence, multi-source tracking, and audio controls.
 */
export class PlayerManager {
    /**
     * How often gain changes are allowed out of the webview.
     *
     * The compromise between the two things a flush does: the audio has to track the finger
     * closely enough to set a level by ear, and the disk does not need writing sixty times a
     * second to record where a slider ended up.
     */
    private static readonly GAIN_WRITE_GAP_MS = 120;

    // Internal reactive stores
    private playersMapStore: Writable<Map<string, PlayerData>>;
    private currentUserStore: Writable<string>;
    private store: Store;
    private gainStoreUnlisten: UnlistenFn | null = null;
    private visibilityHandler: (() => void) | null = null;
    private reseedInterval: ReturnType<typeof setInterval> | null = null;

    /** Settings changed in the webview that have not yet crossed to the backend. */
    private pendingGains: Record<string, Partial<PlayerGainSettings>> = {};
    private readonly gainWrites = new Coalescer(PlayerManager.GAIN_WRITE_GAP_MS, () =>
        this.flushGains(),
    );

    // Readonly exports for components
    public readonly playersMap: Readable<Map<string, PlayerData>>;
    public readonly currentUser: Readable<string>;
    public readonly activePlayers: Readable<PlayerData[]>;

    /**
     * Canonical map/store key for a player. Cards arrive from three sources
     * with two name forms: proximity presence and get_current_players use the
     * bare gamertag, while channel membership uses the CN form
     * ("minecraft:Bob"). The persisted gain store, the sink's name remap, and
     * the control plane all key on the BARE gamertag — so every name entering
     * this manager is normalized here, or the same player forks into two map
     * entries and a freshly group-added card never reacts to gain updates.
     */
    private static key(name: string): string {
        return GameNameUtils.stripPrefix(name);
    }

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
                return players.filter(player => !GameNameUtils.namesMatch(player.name, currentUser));
            }
        );

        info(`PlayerManager: Initialized with current user: ${currentUser || 'none'}`);
    }

    /**
     * Set the current user name
     */
    setCurrentUser(name: string): void {
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
            name = PlayerManager.key(name);
            const playerSettings = settings || { gain: 1.0, muted: false };

            this.playersMapStore.update(map => {
                map.set(name, {
                    name,
                    settings: playerSettings,
                    sources: new Set()
                });
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
            name = PlayerManager.key(name);
            this.playersMapStore.update(map => {
                const removed = map.delete(name);
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
            name = PlayerManager.key(name);
            this.playersMapStore.update(map => {
                const player = map.get(name);
                if (player) {
                    player.settings = { ...player.settings, ...settings };
                    map.set(name, { ...player });
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
        return currentMap.has(PlayerManager.key(name));
    }

    /**
     * Get a specific player
     */
    get(name: string): PlayerData | undefined {
        const currentMap = get(this.playersMapStore);
        return currentMap.get(PlayerManager.key(name));
    }

    /**
     * Clear all players
     */
    clear(): void {
        this.playersMapStore.set(new Map());
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
            const settings = playerGainStore[PlayerManager.key(playerName)] || { gain: 1.0, muted: false };
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
    async addPlayerSource(name: string, source: PlayerSource, settings?: PlayerGainSettings, gamerpic?: string, game?: string): Promise<boolean> {
        try {
            name = PlayerManager.key(name);
            // Load settings if not provided
            const playerSettings = settings || await this.loadPlayerSettings(name);

            this.playersMapStore.update(map => {
                const existing = map.get(name);
                if (existing) {
                    // Player exists, just add the source
                    existing.sources.add(source);
                    if (gamerpic && !existing.gamerpic) {
                        existing.gamerpic = gamerpic;
                    }
                    if (game && !existing.game) {
                        existing.game = game;
                    }
                    map.set(name, { ...existing });
                } else {
                    // New player, create with this source and loaded settings
                    map.set(name, {
                        name,
                        settings: playerSettings,
                        sources: new Set([source]),
                        gamerpic,
                        game
                    });
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
     * Update a player's gamerpic
     */
    updatePlayerGamepic(name: string, gamerpic: string): void {
        name = PlayerManager.key(name);
        this.playersMapStore.update(map => {
            const player = map.get(name);
            if (player) {
                player.gamerpic = gamerpic;
                map.set(name, { ...player });
            }
            return new Map(map);
        });
    }

    /**
     * Remove a source from a player, removing the player entirely if no sources remain
     */
    removePlayerSource(name: string, source: PlayerSource): boolean {
        try {
            name = PlayerManager.key(name);
            this.playersMapStore.update(map => {
                const existing = map.get(name);
                if (existing) {
                    if (existing.sources.has(source)) {
                        existing.sources.delete(source);

                        if (existing.sources.size === 0) {
                            // No more sources, remove player entirely
                            map.delete(name);
                        } else {
                            // Still has other sources, keep player
                            map.set(name, { ...existing });
                        }
                    }
                }
                return new Map(map);
            });

            return true;
        } catch (err) {
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
     * Get the game type for a player
     */
    getPlayerGame(name: string): string | undefined {
        return this.get(name)?.game;
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
        } catch (err) {
            error(`PlayerManager: Failed to update player mute: ${err}`);
        }
    }

    /**
     * Private method to update the persistent Tauri store
     */
    private async updatePlayerGainStore(playerName: string, newSettings: Partial<PlayerGainSettings>): Promise<void> {
        // The persisted store keys on the bare gamertag — the same key the
        // sink's name remap and the control plane use.
        const key = PlayerManager.key(playerName);
        this.pendingGains[key] = { ...this.pendingGains[key], ...newSettings };
        this.gainWrites.request();
    }

    /**
     * Persist and push whatever has accumulated since the last flush.
     *
     * Reads the pending set rather than taking an argument, which is what lets a drag's worth of
     * events collapse: each one overwrites the last in `pendingGains`, and this runs once against
     * the result. Previously every input event did this work itself — a `get`, a `set`, a `save`
     * to disk and an `update_stream_metadata` carrying the whole serialised store, four crossings
     * per pixel of travel on a channel Android serialises.
     */
    private async flushGains(): Promise<void> {
        if (!this.store) return;

        const pending = this.pendingGains;
        this.pendingGains = {};
        if (Object.keys(pending).length === 0) return;

        try {
            const current = (await this.store.get("player_gain_store")) as PlayerGainStore || {};
            const merged: PlayerGainStore = { ...current };
            for (const [key, settings] of Object.entries(pending)) {
                merged[key] = { ...(current[key] ?? { gain: 1.0, muted: false }), ...settings };
            }

            await this.store.set("player_gain_store", merged);
            await this.store.save();

            await invoke("update_stream_metadata", {
                key: "player_gain_store",
                value: JSON.stringify(merged),
                device: "OutputDevice"
            });
        } catch (err) {
            // Put the work back so the next request retries it. Dropping it would lose whatever
            // the user just set with no indication that it had not taken.
            this.pendingGains = { ...pending, ...this.pendingGains };
            error(`PlayerManager: Failed to update player gain store: ${err}`);
        }
    }

    /**
     * Subscribe to backend-initiated gain-store changes (in-game control
     * actions): the backend mutates the persisted store directly, so the
     * reactive map must be re-seeded for the player cards to re-render.
     *
     * The event alone is not enough on mobile: Android suspends the webview
     * whenever the app is not the actively-watched foreground surface, and a
     * Tauri event delivered during that window is one-shot and lost while the
     * backend keeps applying audio state underneath. The cards must CONVERGE,
     * not just react — re-seed on returning to the foreground and on a slow
     * backstop interval, so a missed event costs seconds of staleness, never
     * a permanently wrong card.
     */
    async listenForBackendUpdates(): Promise<void> {
        // Idempotent: the dashboard's cold-start path tears listeners down via
        // cleanup() mid-initialize and must be able to re-register safely.
        this.cleanup();
        // Webview-scoped listen, NOT the global `listen` from api/event: on
        // Android, backend emits reliably reach webview-scoped listeners (the
        // mute/deafen buttons' working pattern) while global-target listeners
        // can miss them; desktop delivers both.
        this.gainStoreUnlisten = await getCurrentWebviewWindow().listen(
            'player_gain_store_updated',
            () => {
                debug('PlayerManager: player_gain_store_updated received; re-seeding cards');
                void this.loadFromPersistentStore();
            },
        );

        this.visibilityHandler = () => {
            if (document.visibilityState === 'visible') {
                debug('PlayerManager: webview foregrounded; re-seeding cards');
                void this.loadFromPersistentStore();
            }
        };
        document.addEventListener('visibilitychange', this.visibilityHandler);

        this.reseedInterval = setInterval(() => {
            void this.loadFromPersistentStore();
        }, PlayerManager.RESEED_BACKSTOP_MS);
    }

    private static readonly RESEED_BACKSTOP_MS = 10_000;

    cleanup(): void {
        // Flushed rather than cancelled: a level set in the last fraction of a second before
        // teardown is still a level the user set, and the coalescer's whole job is that the
        // trailing value survives.
        this.gainWrites.cancel();
        void this.flushGains();

        if (this.gainStoreUnlisten) {
            this.gainStoreUnlisten();
            this.gainStoreUnlisten = null;
        }
        if (this.visibilityHandler) {
            document.removeEventListener('visibilitychange', this.visibilityHandler);
            this.visibilityHandler = null;
        }
        if (this.reseedInterval) {
            clearInterval(this.reseedInterval);
            this.reseedInterval = null;
        }
    }

    async loadFromPersistentStore(): Promise<void> {
        if (!this.store) {
            warn("PlayerManager: Tauri store not available for loading");
            return;
        }

        try {
            const playerGainStore = await this.store.get("player_gain_store") as PlayerGainStore || {};

            // Update settings for existing players. Store keys are normalized
            // too: entries written before key canonicalization may carry the
            // CN-prefixed form.
            this.playersMapStore.update(map => {
                for (const [storeKey, settings] of Object.entries(playerGainStore)) {
                    const playerName = PlayerManager.key(storeKey);
                    const player = map.get(playerName);
                    if (player && settings) {
                        player.settings = settings;
                        map.set(playerName, { ...player });
                    }
                }
                return new Map(map);
            });
        } catch (err) {
            error(`PlayerManager: Failed to load from persistent store: ${err}`);
        }
    }
}