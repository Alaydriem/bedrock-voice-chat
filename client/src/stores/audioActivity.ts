import { writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';

interface AudioActivityState {
    activeSpeakers: Record<string, {
        level: number;
        lastActive: number;
        isHighlighted: boolean;
    }>;
}

export const audioActivity = writable<AudioActivityState>({
    activeSpeakers: {}
});

// Activity highlighting duration
const HIGHLIGHT_DURATION = 1000; // Keep highlight for 1 second after activity stops

class AudioActivityManager {
    private fadeTimeouts: Record<string, number> = {};
    private initialized = false;
    
    constructor() {
        this.initializeListener();
    }
    
    private async initializeListener() {
        if (this.initialized) return;
        this.initialized = true;
        
        try {
            console.log('Initializing audio activity listener...');
            
            // Listen to the streaming audio activity events from Rust
            await listen('audio-activity', (event) => {
                console.log('Received audio-activity event:', event.payload);
                const activityData = event.payload as Record<string, number>;
                this.processActivityUpdate(activityData);
            });
            
            console.log('Audio activity listener initialized successfully');
        } catch (error) {
            console.error('Failed to initialize audio activity listener:', error);
        }
    }
    
    private processActivityUpdate(activityData: Record<string, number>) {
        const timestamp = Date.now();
        console.log('Processing activity update:', activityData, 'at timestamp:', timestamp);
        
        audioActivity.update(state => {
            const newState = { ...state };
            
            // Process each player in the update
            Object.entries(activityData).forEach(([playerName, level]) => {
                console.log(`Updating activity for player: ${playerName}, level: ${level}`);
                
                // Clear existing fade timeout for this player
                if (this.fadeTimeouts[playerName]) {
                    clearTimeout(this.fadeTimeouts[playerName]);
                    delete this.fadeTimeouts[playerName];
                }
                
                // Update or create player activity state
                newState.activeSpeakers[playerName] = {
                    level,
                    lastActive: timestamp,
                    isHighlighted: true
                };
                
                // Set fade timer for this player
                this.fadeTimeouts[playerName] = window.setTimeout(() => {
                    console.log(`Fading activity for player: ${playerName}`);
                    audioActivity.update(currentState => ({
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
            
            console.log('Updated audioActivity state:', newState);
            return newState;
        });
    }
    
    // Cleanup method
    public destroy() {
        Object.values(this.fadeTimeouts).forEach(timeout => clearTimeout(timeout));
        this.fadeTimeouts = {};
    }
}

// Initialize the manager
export const audioActivityManager = new AudioActivityManager();

// Helper function to check if a player is currently highlighted
export function isPlayerHighlighted(playerName: string): boolean {
    let highlighted = false;
    audioActivity.subscribe(state => {
        highlighted = state.activeSpeakers[playerName]?.isHighlighted || false;
    })();
    return highlighted;
}

// Dev helper functions (only in development)
if (import.meta.env.DEV) {
    (window as any).simulateAudioActivity = function(playerName: string, level: number = 1.0) {
        audioActivity.update(state => ({
            ...state,
            activeSpeakers: {
                ...state.activeSpeakers,
                [playerName]: {
                    level,
                    lastActive: Date.now(),
                    isHighlighted: true
                }
            }
        }));
        
        // Auto-fade after 1 second for testing
        setTimeout(() => {
            audioActivity.update(state => ({
                ...state,
                activeSpeakers: {
                    ...state.activeSpeakers,
                    [playerName]: {
                        ...state.activeSpeakers[playerName],
                        isHighlighted: false
                    }
                }
            }));
        }, 1000);
        
        console.log(`Simulated audio activity for ${playerName} with level ${level}`);
    };
    
    (window as any).clearAudioActivity = function() {
        audioActivity.set({ activeSpeakers: {} });
        console.log('Cleared all audio activity');
    };
}
