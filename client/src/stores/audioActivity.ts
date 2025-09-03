import { writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import { info, error, warn, debug } from '@tauri-apps/plugin-log';

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

const HIGHLIGHT_DURATION = 1000;

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
            await listen('audio-activity', (event) => {
                const activityData = event.payload as Record<string, number>;
                this.processActivityUpdate(activityData);
            });

            info('Audio activity listener initialized successfully');
        } catch (e) {
            error('Failed to initialize audio activity listener:', e);
        }
    }
    
    private processActivityUpdate(activityData: Record<string, number>) {
        const timestamp = Date.now();

        audioActivity.update(state => {
            const newState = { ...state };
            
            Object.entries(activityData).forEach(([playerName, level]) => {
                if (this.fadeTimeouts[playerName]) {
                    clearTimeout(this.fadeTimeouts[playerName]);
                    delete this.fadeTimeouts[playerName];
                }
                newState.activeSpeakers[playerName] = {
                    level,
                    lastActive: timestamp,
                    isHighlighted: true
                };

                this.fadeTimeouts[playerName] = window.setTimeout(() => {
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

            return newState;
        });
    }

    public destroy() {
        Object.values(this.fadeTimeouts).forEach(timeout => clearTimeout(timeout));
        this.fadeTimeouts = {};
    }
}

export const audioActivityManager = new AudioActivityManager();

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
    };
    
    (window as any).clearAudioActivity = function() {
        audioActivity.set({ activeSpeakers: {} });
    };
}
