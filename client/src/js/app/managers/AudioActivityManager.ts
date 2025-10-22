import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import { info, error, warn, debug } from '@tauri-apps/plugin-log';
import type { Store } from '@tauri-apps/plugin-store';

interface AudioActivityState {
    activeSpeakers: Record<string, {
        level: number;
        lastActive: number;
        isHighlighted: boolean;
    }>;
}

const HIGHLIGHT_DURATION = 1000;

/**
 * AudioActivityManager handles all audio activity state and real-time updates.
 * Manages speaker activity levels, highlighting, and fade timeouts.
 */
export class AudioActivityManager {
    // Internal reactive stores
    private audioActivityStore: Writable<AudioActivityState>;
    private store: Store;

    // Readonly exports for components
    public readonly audioActivity: Readable<AudioActivityState>;

    // Internal state management
    private fadeTimeouts: Record<string, number> = {};
    private initialized = false;
    private eventUnlisten: (() => void) | null = null;

    constructor(store: Store) {
        this.store = store;
        // Initialize internal store
        this.audioActivityStore = writable<AudioActivityState>({
            activeSpeakers: {}
        });

        // Create readonly export
        this.audioActivity = { subscribe: this.audioActivityStore.subscribe };
    }

    /**
     * Initialize the audio activity listener
     */
    async initialize(): Promise<void> {
        if (this.initialized) {
            return;
        }

        this.initialized = true;

        try {
            this.eventUnlisten = await listen('audio-activity', (event) => {
                const activityData = event.payload as Record<string, number>;
                this.processActivityUpdate(activityData);
            });
        } catch (e) {
            error(`AudioActivityManager: Failed to initialize audio activity listener: ${e}`);
        }
    }

    /**
     * Process incoming audio activity data
     */
    private processActivityUpdate(activityData: Record<string, number>): void {
        const timestamp = Date.now();

        this.audioActivityStore.update(state => {
            const newState = { ...state };

            Object.entries(activityData).forEach(([playerName, level]) => {
                // Clear existing timeout for this player
                if (this.fadeTimeouts[playerName]) {
                    clearTimeout(this.fadeTimeouts[playerName]);
                    delete this.fadeTimeouts[playerName];
                }

                // Update player activity
                newState.activeSpeakers[playerName] = {
                    level,
                    lastActive: timestamp,
                    isHighlighted: true
                };

                // Set timeout to remove highlighting
                this.fadeTimeouts[playerName] = window.setTimeout(() => {
                    this.audioActivityStore.update(currentState => ({
                        ...currentState,
                        activeSpeakers: {
                            ...currentState.activeSpeakers,
                            [playerName]: {
                                ...currentState.activeSpeakers[playerName],
                                isHighlighted: false
                            }
                        }
                    }));
                    delete this.fadeTimeouts[playerName];
                }, HIGHLIGHT_DURATION);
            });

            return newState;
        });
    }

    /**
     * Check if a specific player is currently highlighted
     */
    isPlayerHighlighted(playerName: string): boolean {
        const currentState = get(this.audioActivityStore);
        return currentState.activeSpeakers[playerName]?.isHighlighted || false;
    }

    /**
     * Get the current activity level for a player
     */
    getPlayerActivityLevel(playerName: string): number {
        const currentState = get(this.audioActivityStore);
        return currentState.activeSpeakers[playerName]?.level || 0;
    }

    /**
     * Get the last active timestamp for a player
     */
    getPlayerLastActive(playerName: string): number | null {
        const currentState = get(this.audioActivityStore);
        return currentState.activeSpeakers[playerName]?.lastActive || null;
    }

    /**
     * Get all currently active speakers
     */
    getActiveSpeakers(): Record<string, { level: number; lastActive: number; isHighlighted: boolean; }> {
        const currentState = get(this.audioActivityStore);
        return currentState.activeSpeakers;
    }

    /**
     * Clear activity for a specific player
     */
    clearPlayerActivity(playerName: string): void {
        // Clear timeout if exists
        if (this.fadeTimeouts[playerName]) {
            clearTimeout(this.fadeTimeouts[playerName]);
            delete this.fadeTimeouts[playerName];
        }

        // Remove from active speakers
        this.audioActivityStore.update(state => {
            const newState = { ...state };
            delete newState.activeSpeakers[playerName];
            return newState;
        });
    }

    /**
     * Check if a specific player is currently speaking (highlighted)
     */
    isPlayerSpeaking(playerName: string): boolean {
        const state = get(this.audioActivityStore);
        return state.activeSpeakers[playerName]?.isHighlighted || false;
    }

    /**
     * Get audio activity level for a specific player
     */
    getPlayerLevel(playerName: string): number {
        const state = get(this.audioActivityStore);
        return state.activeSpeakers[playerName]?.level || 0;
    }

    /**
     * Clear all audio activity
     */
    clearAllActivity(): void {
        // Clear all timeouts
        Object.values(this.fadeTimeouts).forEach(timeout => clearTimeout(timeout));
        this.fadeTimeouts = {};

        // Reset store
        this.audioActivityStore.set({
            activeSpeakers: {}
        });
    }

    /**
     * Cleanup and destroy the manager
     */
    destroy(): void {
        // Clear all timeouts
        Object.values(this.fadeTimeouts).forEach(timeout => clearTimeout(timeout));
        this.fadeTimeouts = {};

        // Unlisten from events
        if (this.eventUnlisten) {
            this.eventUnlisten();
            this.eventUnlisten = null;
        }

        // Reset state
        this.audioActivityStore.set({
            activeSpeakers: {}
        });

        this.initialized = false;
    }
}